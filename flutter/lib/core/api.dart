import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:http/http.dart' as http;
import 'package:http_parser/http_parser.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

import 'browser_cookies.dart';
import 'models.dart';

const _defaultBaseUrl = String.fromEnvironment(
  'FLACKLE_BASE_URL',
  defaultValue: 'https://flackle.com',
);

class FlackleException implements Exception {
  const FlackleException(this.message, {this.statusCode, this.code});

  final String message;
  final int? statusCode;
  final String? code;

  bool get isUnauthorized => statusCode == 401 || statusCode == 403;

  @override
  String toString() => message;
}

enum GatewayStatus { offline, connecting, connected }

class GatewayEvent {
  const GatewayEvent(this.type, this.payload);

  final String type;
  final Map<String, dynamic> payload;

  String? get serverId => payload['server_id'] as String?;
  String? get channelId => payload['channel_id'] as String?;
  String? get conversationId => payload['conversation_id'] as String?;
}

class LoginResult {
  const LoginResult.account(this.account)
      : mfaToken = null,
        requiresMfa = false;
  const LoginResult.mfa(this.mfaToken)
      : account = null,
        requiresMfa = true;

  final SelfUser? account;
  final String? mfaToken;
  final bool requiresMfa;
}

class _Session {
  const _Session({this.accessToken, this.refreshToken, this.csrfToken});

  final String? accessToken;
  final String? refreshToken;
  final String? csrfToken;

  Map<String, dynamic> toJson() => {
        'access_token': accessToken,
        'refresh_token': refreshToken,
        'csrf_token': csrfToken,
      };

  factory _Session.fromJson(Map<String, dynamic> json) => _Session(
        accessToken: asStringOrNull(json['access_token']),
        refreshToken: asStringOrNull(json['refresh_token']),
        csrfToken: asStringOrNull(json['csrf_token']),
      );
}

class FlackleClient {
  FlackleClient({
    String? baseUrl,
    http.Client? httpClient,
  })  : baseUrl = (baseUrl ?? _defaultBaseUrl).replaceAll(RegExp(r'/$'), ''),
        _http = httpClient ?? http.Client();

  final String baseUrl;
  final http.Client _http;

  static const _sessionKey = 'flackle.mobile.session.v2';
  static const _secure = FlutterSecureStorage();

  final Map<String, String> _cookies = {};
  final Map<String, Completer<Message?>> _pending = {};
  final StreamController<GatewayEvent> _events =
      StreamController<GatewayEvent>.broadcast();
  final StreamController<GatewayStatus> _statusController =
      StreamController<GatewayStatus>.broadcast();

  String? _accessToken;
  String? _refreshToken;
  String? _csrfToken;
  WebSocketChannel? _socket;
  StreamSubscription<dynamic>? _socketSubscription;
  Timer? _heartbeat;
  Timer? _reconnectTimer;
  List<String> _gatewayServers = const [];
  int _reconnectAttempts = 0;
  bool _manualDisconnect = true;
  bool _disposed = false;
  bool _cookieSession = false;
  GatewayStatus _status = GatewayStatus.offline;

  Stream<GatewayEvent> get events => _events.stream;
  Stream<GatewayStatus> get statusStream => _statusController.stream;
  GatewayStatus get status => _status;
  bool get isConnected => _status == GatewayStatus.connected;
  bool get hasSession =>
      _accessToken?.isNotEmpty == true ||
      _refreshToken?.isNotEmpty == true ||
      (_cookieSession && kIsWeb);

  // ---------------------------------------------------------------------------
  // Session
  // ---------------------------------------------------------------------------

  Future<SelfUser?> restoreSession() async {
    final session = await _readSession();
    if (session != null) {
      _accessToken = session.accessToken;
      _refreshToken = session.refreshToken;
      _csrfToken = session.csrfToken;
      _syncCookies();
    }
    _absorbBrowserCookies();
    if (hasSession) {
      try {
        return await fetchMe();
      } on FlackleException {
        // Keep the stored session; a network outage must not sign the user out.
        return null;
      } catch (_) {
        return null;
      }
    }
    if (kIsWeb) {
      // The web client signs in with first-party cookies; probe for an existing
      // session before showing the login screen.
      try {
        final user = await fetchMe();
        _cookieSession = true;
        return user;
      } catch (_) {
        return null;
      }
    }
    return null;
  }

  Future<LoginResult> login(String email, String password) async {
    await _ensureCsrf();
    final data = asMap(await _json(
      'POST',
      '/api/auth/login',
      body: {'email': email.trim(), 'password': password},
      authenticated: false,
    ));
    if (asBool(data['mfa_required']) ||
        asString(data['status']) == 'mfa_required') {
      final token = asStringOrNull(data['mfa_token']);
      if (token == null) {
        throw const FlackleException(
            'The two-factor challenge did not include a token.');
      }
      return LoginResult.mfa(token);
    }
    return LoginResult.account(await _completeLogin(data));
  }

  Future<SelfUser> loginMfa(
    String mfaToken,
    String code, {
    bool backupCode = false,
  }) async {
    final body = <String, dynamic>{'mfa_token': mfaToken};
    body[backupCode ? 'backup_code' : 'code'] = code.trim();
    final data = asMap(await _json(
      'POST',
      '/api/auth/login/mfa',
      body: body,
      authenticated: false,
    ));
    return _completeLogin(data);
  }

  Future<SelfUser> _completeLogin(Map<String, dynamic> data) async {
    _accessToken =
        asStringOrNull(data['access_token']) ?? _cookies['access_token'];
    _refreshToken =
        asStringOrNull(data['refresh_token']) ?? _cookies['refresh_token'];
    _csrfToken = asStringOrNull(data['csrf_token']) ?? _cookies['__Host-csrf'];
    _absorbBrowserCookies();
    final cookieSignIn =
        kIsWeb && (_accessToken == null || _accessToken!.isEmpty);
    if (cookieSignIn) {
      _cookieSession = true;
    } else if (_accessToken == null || _accessToken!.isEmpty) {
      throw FlackleException(
        asStringOrNull(data['error']) ??
            'Sign-in did not return an access token.',
      );
    }
    final rawUser = data['user'];
    final SelfUser user;
    if (rawUser is Map) {
      user = SelfUser.fromJson(asMap(rawUser));
    } else {
      user = SelfUser.fromJson(asMap(await _json('GET', '/api/auth/me')));
    }
    if (!cookieSignIn) {
      // Persisted tokens are useful on mobile and for web builds that receive
      // bearer tokens from the API.
      await _saveSession();
    }
    return user;
  }

  Future<SelfUser> fetchMe() async =>
      SelfUser.fromJson(asMap(await _json('GET', '/api/auth/me')));

  Future<void> logout() async {
    try {
      await _json('POST', '/api/auth/logout', body: const {});
    } catch (_) {
      // Signing out locally is still correct when the network is unavailable.
    } finally {
      await disconnect();
      _accessToken = null;
      _refreshToken = null;
      _csrfToken = null;
      _cookieSession = false;
      _cookies.clear();
      _pending.forEach((_, completer) {
        if (!completer.isCompleted) {
          completer.completeError(const FlackleException('Signed out.'));
        }
      });
      _pending.clear();
      await _clearSession();
    }
  }

  Future<void> _saveSession() async {
    _syncCookies();
    final payload = jsonEncode(_Session(
      accessToken: _accessToken,
      refreshToken: _refreshToken,
      csrfToken: _csrfToken,
    ).toJson());
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(_sessionKey, payload);
    } catch (_) {
      // Preference storage can be unavailable in private windows.
    }
    try {
      await _secure.write(key: _sessionKey, value: payload);
    } catch (_) {
      // Web and some Linux setups have no keyring; the disk copy remains.
    }
  }

  Future<_Session?> _readSession() async {
    try {
      final secure = await _secure.read(key: _sessionKey);
      if (secure != null && secure.isNotEmpty) {
        return _Session.fromJson(asMap(jsonDecode(secure)));
      }
    } catch (_) {
      // Fall through to disk.
    }
    try {
      final prefs = await SharedPreferences.getInstance();
      final raw = prefs.getString(_sessionKey);
      if (raw != null && raw.isNotEmpty) {
        return _Session.fromJson(asMap(jsonDecode(raw)));
      }
    } catch (_) {
      // No saved session.
    }
    return null;
  }

  Future<void> _clearSession() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.remove(_sessionKey);
    } catch (_) {}
    try {
      await _secure.delete(key: _sessionKey);
    } catch (_) {}
  }

  void _absorbBrowserCookies() {
    if (!kIsWeb) return;
    final browser = readBrowserCookies();
    if (browser.isEmpty) return;
    _cookies.addAll(browser);
    _csrfToken ??= browser['__Host-csrf'] ??
        browser['csrf_token'] ??
        browser['XSRF-TOKEN'];
    _accessToken ??= browser['access_token'];
    _refreshToken ??= browser['refresh_token'];
    if (browser.containsKey('session') ||
        browser.containsKey('refresh_token') ||
        browser.containsKey('access_token')) {
      _cookieSession = true;
    }
  }

  void _syncCookies() {
    if (_accessToken?.isNotEmpty == true)
      _cookies['access_token'] = _accessToken!;
    if (_refreshToken?.isNotEmpty == true)
      _cookies['refresh_token'] = _refreshToken!;
    if (_csrfToken?.isNotEmpty == true) _cookies['__Host-csrf'] = _csrfToken!;
  }

  // ---------------------------------------------------------------------------
  // Bootstrap and servers
  // ---------------------------------------------------------------------------

  Future<Map<String, dynamic>> bootstrap() async {
    final values = await Future.wait([
      _json('GET', '/api/auth/me'),
      _json('GET', '/api/servers'),
      _json('GET', '/api/conversations'),
      _json('GET', '/api/servers/@me/unread-summary'),
      _json('GET', '/api/servers/@me/settings'),
    ]);
    return {
      'me': SelfUser.fromJson(asMap(values[0])),
      'servers': _list(values[1], 'servers')
          .map((value) => Server.fromJson(asMap(value)))
          .toList(),
      'conversations': _list(values[2], 'conversations')
          .map((value) => Conversation.fromJson(asMap(value)))
          .toList(),
      'unread': UnreadSummary.fromJson(asMap(values[3])),
      'server_settings': _list(values[4], 'server_settings')
          .map((value) => ServerUserSettings.fromJson(asMap(value)))
          .toList(),
      'folders': asList(asMap(values[4])['folders'])
          .whereType<Map>()
          .map((value) => ServerFolder.fromJson(asMap(value)))
          .toList(),
    };
  }

  Future<List<Server>> fetchServers() async {
    final data = await _json('GET', '/api/servers');
    return _list(data, 'servers')
        .map((value) => Server.fromJson(asMap(value)))
        .toList();
  }

  Future<Server> fetchServer(String serverId) async =>
      Server.fromJson(asMap(await _json('GET', '/api/servers/$serverId')));

  Future<List<Conversation>> fetchConversations() async {
    final data = await _json('GET', '/api/conversations');
    return _list(data, 'conversations')
        .map((value) => Conversation.fromJson(asMap(value)))
        .toList();
  }

  Future<UnreadSummary> fetchUnread() async => UnreadSummary.fromJson(
      asMap(await _json('GET', '/api/servers/@me/unread-summary')));

  Future<List<ServerUserSettings>> fetchServerSettings() async {
    final data = await _json('GET', '/api/servers/@me/settings');
    return _list(data, 'server_settings')
        .map((value) => ServerUserSettings.fromJson(asMap(value)))
        .toList();
  }

  Future<void> updateServerUserSettings(
      String serverId, Map<String, dynamic> patch) async {
    await _json('PUT', '/api/servers/@me/settings', body: {
      'servers': [
        {'server_id': serverId, ...patch}
      ]
    });
  }

  Future<void> updateServerPositions(List<Map<String, dynamic>> servers) async {
    await _json('PUT', '/api/servers/@me/settings', body: {'servers': servers});
  }

  Future<List<ServerFolder>> fetchFolders() async {
    final data = asMap(await _json('GET', '/api/servers/@me/settings'));
    return asList(data['folders'])
        .whereType<Map>()
        .map((value) => ServerFolder.fromJson(asMap(value)))
        .toList();
  }

  Future<ServerFolder> createServerFolder(
    String name, {
    String? color,
  }) async {
    final data = asMap(await _json('POST', '/api/servers/@me/folders', body: {
      'name': name,
      if (color != null && color.isNotEmpty) 'color': color,
    }));
    return ServerFolder.fromJson(asMap(data['folder'] ?? data));
  }

  Future<ServerFolder> updateServerFolder(
    String folderId, {
    String? name,
    String? color,
    String? emoji,
  }) async {
    final data = asMap(await _json(
      'PATCH',
      '/api/servers/@me/folders/${Uri.encodeComponent(folderId)}',
      body: {
        if (name != null) 'name': name,
        if (color != null) 'color': color,
        if (emoji != null) 'emoji': emoji,
      },
    ));
    return ServerFolder.fromJson(asMap(data['folder'] ?? data));
  }

  Future<void> deleteServerFolder(String folderId) async {
    await _json(
      'DELETE',
      '/api/servers/@me/folders/${Uri.encodeComponent(folderId)}',
    );
  }

  Future<void> reorderServerFolders(List<String> order) async {
    await _json('PUT', '/api/servers/@me/folders/positions',
        body: {'order': order});
  }

  Future<void> saveServerLayout({
    required List<ServerFolder> folders,
    required List<Map<String, dynamic>> servers,
    List<String> deleteEmptyFolders = const [],
  }) async {
    await _json('PUT', '/api/servers/@me/settings', body: {
      'folders': folders.map((folder) => folder.toJson()).toList(),
      if (deleteEmptyFolders.isNotEmpty)
        'delete_empty_folders': deleteEmptyFolders,
      'servers': servers,
    });
  }

  Future<Server> updateServer(
          String serverId, Map<String, dynamic> patch) async =>
      Server.fromJson(
          asMap(await _json('PATCH', '/api/servers/$serverId', body: patch)));

  Future<String> uploadServerIcon(
      String serverId, String fileName, List<int> bytes) async {
    final data = await _upload(
      '/api/servers/$serverId/icon',
      field: 'icon',
      fileName: fileName,
      bytes: bytes,
      contentType: _contentTypeFor(fileName),
    );
    final map = asMap(data);
    return asString(map['icon_url'] ?? map['url']);
  }

  Future<void> markServerRead(String serverId) async {
    await _json('POST', '/api/servers/$serverId/mark-read', body: const {});
  }

  Future<Map<String, dynamic>> createServer(String name) async =>
      asMap(await _json('POST', '/api/servers', body: {'name': name}));

  Future<void> leaveServer(String serverId) async {
    await _json('DELETE', '/api/servers/$serverId/members/@me');
  }

  Future<String> fetchServerPermissions(String serverId) async {
    final data =
        asMap(await _json('GET', '/api/servers/$serverId/permissions/@me'));
    return asString(data['permissions'], '0');
  }

  // ---------------------------------------------------------------------------
  // Channels, categories, analytics
  // ---------------------------------------------------------------------------

  Future<ChannelBundle> fetchChannels(String serverId) async =>
      ChannelBundle.fromJson(
          asMap(await _json('GET', '/api/servers/$serverId/channels')));

  Future<List<String>> fetchChannelKinds() async {
    final data = asMap(await _json('GET', '/api/channel-kinds/creatable'));
    return asList(data['types']).map((value) => '$value').toList();
  }

  Future<Channel> createChannel(
          String serverId, Map<String, dynamic> options) async =>
      Channel.fromJson(asMap(await _json(
          'POST', '/api/servers/$serverId/channels',
          body: options)));

  Future<Channel> updateChannel(
    String serverId,
    String channelId,
    Map<String, dynamic> patch,
  ) async =>
      Channel.fromJson(asMap(
        await _json('PATCH', '/api/servers/$serverId/channels/$channelId',
            body: patch),
      ));

  Future<void> deleteChannel(String serverId, String channelId) async {
    await _json('DELETE', '/api/servers/$serverId/channels/$channelId');
  }

  Future<void> updateChannelPositions(
    String serverId,
    List<Map<String, dynamic>> channels,
  ) async {
    await _json('POST', '/api/servers/$serverId/channels/positions',
        body: {'channels': channels});
  }

  Future<Category> createCategory(String serverId, String name) async {
    final data = asMap(await _json('POST', '/api/servers/$serverId/categories',
        body: {'name': name}));
    return Category.fromJson(asMap(data['category'] ?? data));
  }

  Future<Category> updateCategory(
      String serverId, String categoryId, String name) async {
    final data = asMap(await _json(
      'PATCH',
      '/api/servers/$serverId/categories/$categoryId',
      body: {'name': name},
    ));
    return Category.fromJson(asMap(data['category'] ?? data));
  }

  Future<void> deleteCategory(String serverId, String categoryId) async {
    await _json('DELETE', '/api/servers/$serverId/categories/$categoryId');
  }

  Future<AnalyticsBundle> fetchAnalytics(
          String serverId, String channelId) async =>
      AnalyticsBundle.fromJson(asMap(await _json(
        'GET',
        '/api/servers/$serverId/channels/$channelId/analytics/bundle',
      )));

  Future<List<Map<String, dynamic>>> fetchActivityCalendar(
    String serverId,
    String channelId,
    int year,
  ) async {
    final data = asMap(await _json(
      'GET',
      '/api/servers/$serverId/channels/$channelId/analytics/calendar',
      query: {'year': '$year'},
    ));
    return asList(data['days']).whereType<Map>().map(asMap).toList();
  }

  Future<List<Map<String, dynamic>>> fetchPunchcard(
    String serverId,
    String channelId,
    int days,
  ) async {
    final data = asMap(await _json(
      'GET',
      '/api/servers/$serverId/channels/$channelId/analytics/punchcard',
      query: {'days': '$days'},
    ));
    return asList(data['cells']).whereType<Map>().map(asMap).toList();
  }

  // ---------------------------------------------------------------------------
  // Messages
  // ---------------------------------------------------------------------------

  Future<MessagePage> fetchMessages(Target target,
      {String? before, int limit = 50}) async {
    final data = await _json('GET', target.messagesPath, query: {
      'limit': '$limit',
      if (before != null) 'before': before,
    });
    return MessagePage.fromJson(asMap(data));
  }

  Future<MessagePage> fetchMessagesAround(Target target, String around,
      {int limit = 50}) async {
    final data = await _json('GET', target.messagesPath, query: {
      'limit': '$limit',
      'around': around,
    });
    return MessagePage.fromJson(asMap(data));
  }

  Future<Message> sendMessage(
    Target target,
    String content, {
    List<Attachment> attachments = const [],
    String? replyTo,
    String? nonce,
  }) async {
    if (content.trim().isEmpty && attachments.isEmpty) {
      throw const FlackleException(
          'Write a message or add an attachment first.');
    }
    if (!isConnected) {
      throw const FlackleException('The live connection is not ready yet.');
    }
    final messageNonce = nonce ?? _newNonce();
    final completer = Completer<Message?>();
    _pending[messageNonce] = completer;
    final payload = <String, dynamic>{
      ...target.toJson(),
      'content': content,
      'nonce': messageNonce,
      if (attachments.isNotEmpty)
        'attachments': attachments.map((entry) => entry.toJson()).toList(),
      if (replyTo != null) 'reply_to_message_id': replyTo,
    };
    _sendFrame({
      'type': target.isChannel ? 'send_channel_message' : 'send_message',
      'payload': payload,
    });
    try {
      final message =
          await completer.future.timeout(const Duration(seconds: 15));
      if (message == null)
        throw const FlackleException(
            'Flackle did not return the sent message.');
      return message;
    } on TimeoutException {
      throw const FlackleException(
          'Flackle did not confirm the message. Refresh before retrying.');
    } finally {
      _pending.remove(messageNonce);
    }
  }

  Future<void> editMessage(Target target, String messageId, String content) =>
      _messageOp(target, messageId, content, edit: true);

  Future<void> deleteMessage(Target target, String messageId) =>
      _messageOp(target, messageId, '', edit: false);

  Future<void> _messageOp(
    Target target,
    String messageId,
    String content, {
    required bool edit,
  }) async {
    if (!isConnected)
      throw const FlackleException('The live connection is not ready yet.');
    final nonce = _newNonce();
    final completer = Completer<Message?>();
    _pending[nonce] = completer;
    _sendFrame({
      'type': target.isChannel
          ? (edit ? 'edit_channel_message' : 'delete_channel_message')
          : (edit ? 'edit_message' : 'delete_message'),
      'payload': {
        ...target.toJson(),
        'message_id': messageId,
        'nonce': nonce,
        if (edit) 'content': content,
      },
    });
    try {
      await completer.future.timeout(const Duration(seconds: 15));
    } on TimeoutException {
      throw FlackleException(edit
          ? 'Flackle did not confirm the edit.'
          : 'Flackle did not confirm the deletion.');
    } finally {
      _pending.remove(nonce);
    }
  }

  Future<void> react(Target target, String messageId, String emoji) async {
    final path = _reactionPath(target, messageId, emoji);
    await _json('PUT', path);
  }

  Future<void> unreact(Target target, String messageId, String emoji) async {
    final path = _reactionPath(target, messageId, emoji);
    await _json('DELETE', path);
  }

  String _reactionPath(Target target, String messageId, String emoji) {
    final encoded = Uri.encodeComponent(emoji);
    final base = '${target.messagesPath}/$messageId/reactions/$encoded';
    return target.isChannel ? '$base/@me' : base;
  }

  Future<void> sendTyping(Target target) async {
    if (!isConnected) return;
    _sendFrame({
      'type': target.isChannel ? 'channel_typing_start' : 'typing_start',
      'payload': target.toJson(),
    });
  }

  Future<void> stopTyping(Target target) async {
    if (!isConnected) return;
    _sendFrame({
      'type': target.isChannel ? 'channel_typing_stop' : 'typing_stop',
      'payload': target.toJson(),
    });
  }

  Future<void> markRead(Target target, String messageId) async {
    if (!isConnected) return;
    _sendFrame({
      'type': target.isChannel ? 'mark_channel_read_channel' : 'mark_read',
      'payload': {...target.toJson(), 'message_id': messageId},
    });
  }

  Future<Attachment> uploadAttachment(
    Target target,
    String fileName,
    List<int> bytes,
  ) async {
    final data = await _upload(
      target.attachmentsPath,
      field: 'file',
      fileName: fileName,
      bytes: bytes,
      contentType: _contentTypeFor(fileName),
    );
    final map = asMap(data);
    final list = asList(map['attachments']);
    if (list.isNotEmpty) return Attachment.fromJson(asMap(list.first));
    return Attachment.fromJson(asMap(map['attachment'] ?? map));
  }

  Future<Map<String, dynamic>> createLinkPreview(String url) async =>
      asMap(await _json('GET', '/api/link-preview', query: {'url': url}));

  Future<Map<String, dynamic>> updateMessageEmbeds(
    Target target,
    String messageId,
    List<String> tokens,
  ) async =>
      asMap(await _json('POST', '${target.messagesPath}/$messageId/embeds',
          body: {'tokens': tokens}));

  Future<String> fetchChannelPermissions(
      String serverId, String channelId) async {
    final data = asMap(await _json(
      'GET',
      '/api/servers/$serverId/channels/$channelId/permissions/@me',
    ));
    return asString(data['permissions'], '0');
  }

  // ---------------------------------------------------------------------------
  // Members, roles, invites, moderation
  // ---------------------------------------------------------------------------

  Future<List<Member>> fetchMembers(String serverId) async {
    final data = await _json('GET', '/api/servers/$serverId/members');
    return _list(data, 'members')
        .map((value) => Member.fromJson(asMap(value)))
        .toList();
  }

  Future<MemberListSnapshot> fetchMemberSnapshot(
          String serverId, String channelId) async =>
      MemberListSnapshot.fromJson(asMap(await _json(
        'GET',
        '/api/servers/$serverId/channels/$channelId/member-list-snapshot',
      )));

  Future<void> setMemberRoles(
    String serverId,
    String userId,
    List<String> add,
    List<String> remove,
  ) async {
    await _json('PUT', '/api/servers/$serverId/members/$userId/roles',
        body: {'add': add, 'remove': remove});
  }

  Future<void> updateMember(
    String serverId,
    String userId,
    Map<String, dynamic> patch,
  ) async {
    await _json('PATCH', '/api/servers/$serverId/members/$userId', body: patch);
  }

  Future<void> kickMember(String serverId, String userId,
      {String? reason}) async {
    await _json('DELETE',
        '/api/servers/$serverId/members/$userId?reason=${Uri.encodeComponent(reason ?? '')}');
  }

  Future<RolesResponse> fetchRoles(String serverId) async =>
      RolesResponse.fromJson(
          asMap(await _json('GET', '/api/servers/$serverId/roles')));

  Future<RolesResponse> batchRoles(
          String serverId, Map<String, dynamic> batch) async =>
      RolesResponse.fromJson(asMap(
        await _json('PATCH', '/api/servers/$serverId/roles/batch', body: batch),
      ));

  Future<RolesResponse> uploadRoleIcon(
    String serverId,
    String roleId,
    String fileName,
    List<int> bytes, {
    Map<String, dynamic>? batch,
  }) async {
    final data = await _upload(
      '/api/servers/$serverId/roles/batch',
      field: 'icon_$roleId',
      fileName: fileName,
      bytes: bytes,
      contentType: _contentTypeFor(fileName),
      method: 'PATCH',
      extraFields: {
        'batch': jsonEncode(batch ??
            {
              'updates': [
                {'id': roleId}
              ]
            }),
      },
    );
    return RolesResponse.fromJson(asMap(data));
  }

  Future<List<Invite>> fetchServerInvites(String serverId) async {
    final data = await _json('GET', '/api/servers/$serverId/invites');
    return _list(data, 'invites')
        .map((value) => Invite.fromJson(asMap(value)))
        .toList();
  }

  Future<Invite> createServerInvite(
          String serverId, Map<String, dynamic> options) async =>
      Invite.fromJson(asMap(
        await _json('POST', '/api/servers/$serverId/invites', body: options),
      ));

  Future<void> deleteServerInvite(String serverId, String code) async {
    await _json('DELETE', '/api/servers/$serverId/invites/$code');
  }

  Future<Map<String, dynamic>> acceptInvite(String code) async => asMap(
      await _json('POST', '/api/server-invites/$code/accept', body: const {}));

  Future<List<ServerBan>> fetchBans(String serverId) async {
    final data = await _json('GET', '/api/servers/$serverId/bans');
    return _list(data, 'bans')
        .map((value) => ServerBan.fromJson(asMap(value)))
        .toList();
  }

  Future<void> banMember(String serverId, String userId,
      {String? reason, int? deleteDays}) async {
    await _json('POST', '/api/servers/$serverId/bans', body: {
      'user_id': userId,
      if (reason != null) 'reason': reason,
      if (deleteDays != null) 'delete_message_seconds': deleteDays * 86400,
    });
  }

  Future<void> unbanMember(String serverId, String userId) async {
    await _json('DELETE', '/api/servers/$serverId/bans/$userId');
  }

  Future<Map<String, dynamic>> fetchAuditLog(
    String serverId, {
    int limit = 50,
    String? before,
    String? actionType,
  }) async =>
      asMap(await _json('GET', '/api/servers/$serverId/audit-log', query: {
        'limit': '$limit',
        if (before != null) 'before': before,
        if (actionType != null) 'action_type': actionType,
      }));

  Future<LiftsInfo> fetchLifts(String serverId) async => LiftsInfo.fromJson(
      asMap(await _json('GET', '/api/servers/$serverId/lifts')));

  Future<List<User>> fetchLiftSupporters(String serverId) async {
    final data = await _json('GET', '/api/servers/$serverId/lifts/supporters');
    return _list(data, 'supporters')
        .map((value) => User.fromJson(asMap(value)))
        .toList();
  }

  Future<SavedMediaPage> fetchSavedMedia(String serverId) async =>
      SavedMediaPage.fromJson(
          asMap(await _json('GET', '/api/media/saved?server_id=$serverId')));

  // ---------------------------------------------------------------------------
  // Friends, users, conversations
  // ---------------------------------------------------------------------------

  Future<List<User>> fetchFriends() async {
    final data = await _json('GET', '/api/friends');
    return _list(data, 'friends')
        .map((value) => User.fromJson(asMap(value)))
        .toList();
  }

  Future<List<User>> fetchBlocked() async {
    final data = await _json('GET', '/api/friends/blocked');
    return _list(data, 'blocked')
        .map((value) => User.fromJson(asMap(value)))
        .toList();
  }

  Future<FriendRequests> fetchFriendRequests() async => FriendRequests.fromJson(
      asMap(await _json('GET', '/api/friends/requests')));

  Future<void> addFriend(String username) async {
    await _json('POST', '/api/friends/requests',
        body: {'username': username.trim()});
  }

  Future<void> acceptFriendRequest(String requestId) async {
    await _json('POST', '/api/friends/requests/$requestId/accept',
        body: const {});
  }

  Future<void> rejectFriendRequest(String requestId) async {
    await _json('POST', '/api/friends/requests/$requestId/reject',
        body: const {});
  }

  Future<void> cancelFriendRequest(String requestId) async {
    await _json('DELETE', '/api/friends/requests/$requestId');
  }

  Future<void> removeFriend(String userId) async {
    await _json('DELETE', '/api/friends/$userId');
  }

  Future<void> blockUser(String userId) async {
    await _json('POST', '/api/friends/block', body: {'user_id': userId});
  }

  Future<void> unblockUser(String userId) async {
    await _json('DELETE', '/api/friends/block/$userId');
  }

  Future<void> reportUser(
      String userId, String category, String details) async {
    await _json('POST', '/api/reports', body: {
      'user_id': userId,
      'category': category,
      'additional_info': details,
    });
  }

  Future<Conversation> createDM(String userId) async => Conversation.fromJson(
          asMap(await _json('POST', '/api/conversations', body: {
        'member_ids': [userId]
      })));

  Future<UserCard> fetchUserCard(String username, {String? serverId}) async =>
      UserCard.fromJson(asMap(await _json(
        'GET',
        '/api/users/${Uri.encodeComponent(username)}/card',
        query: {if (serverId != null) 'server_id': serverId},
      )));

  Future<List<BentoCard>> fetchUserBento(String username) async {
    final data =
        await _json('GET', '/api/users/${Uri.encodeComponent(username)}/bento');
    final list = data is List ? data : asList(asMap(data)['cards']);
    return list.map((value) => BentoCard.fromJson(asMap(value))).toList();
  }

  // ---------------------------------------------------------------------------
  // Account, security, status
  // ---------------------------------------------------------------------------

  Future<List<AuthSession>> fetchSessions() async {
    final data = await _json('GET', '/api/auth/sessions');
    return _list(data, 'sessions')
        .map((value) => AuthSession.fromJson(asMap(value)))
        .toList();
  }

  Future<void> revokeSession(String sessionId, {String? code}) async {
    await _json('DELETE', '/api/auth/sessions/$sessionId',
        body: {if (code != null) 'code': code});
  }

  Future<void> revokeAllSessions({String? code}) async {
    await _json('DELETE', '/api/auth/sessions',
        body: {if (code != null) 'code': code});
  }

  Future<List<SecurityEvent>> fetchSecurityEvents() async {
    final data = await _json('GET', '/api/auth/security-events');
    return _list(data, 'events')
        .map((value) => SecurityEvent.fromJson(asMap(value)))
        .toList();
  }

  Future<void> clearSecurityEvents({String? code}) async {
    await _json('POST', '/api/auth/security-events/clear',
        body: {if (code != null) 'code': code});
  }

  Future<TwoFactorStatus> fetchTwoFactorStatus() async =>
      TwoFactorStatus.fromJson(asMap(await _json('GET', '/api/auth/2fa')));

  Future<TwoFactorSetup> setupTwoFactor(String password) async =>
      TwoFactorSetup.fromJson(asMap(
        await _json('POST', '/api/auth/2fa/setup',
            body: {'password': password}),
      ));

  Future<void> confirmTwoFactor(String code) async {
    await _json('POST', '/api/auth/2fa/confirm', body: {'code': code});
  }

  Future<void> disableTwoFactor(String password,
      {String? code, String? backupCode}) async {
    await _json('POST', '/api/auth/2fa/disable', body: {
      'password': password,
      if (code != null) 'code': code,
      if (backupCode != null) 'backup_code': backupCode,
    });
  }

  Future<List<String>> createBackupCodes(String password, String code) async {
    final data = asMap(await _json('POST', '/api/auth/2fa/backup-codes',
        body: {'password': password, 'code': code}));
    return asList(data['codes']).map((value) => '$value').toList();
  }

  Future<String> fetchStatus() async {
    final data = asMap(await _json('GET', '/api/status'));
    return asString(data['status'], 'online');
  }

  Future<void> setStatus(String status) async {
    await _json('PATCH', '/api/status', body: {'status': status});
  }

  Future<void> updateTypingIndicator({
    required bool enabled,
    String? text,
    String? style,
    List<String>? emojis,
  }) async {
    await _json('PATCH', '/api/users/typing-indicator', body: {
      'enabled': enabled,
      if (enabled && text != null) 'text': text,
      if (enabled && style != null) 'style': style,
      if (enabled && emojis != null) 'emojis': emojis,
    });
  }

  // ---------------------------------------------------------------------------
  // Gateway
  // ---------------------------------------------------------------------------

  Future<void> connect(List<String> serverIds) async {
    if (_disposed || _socket != null) return;
    _manualDisconnect = false;
    _gatewayServers = List.of(serverIds);
    _setStatus(GatewayStatus.connecting);
    final ticketData = asMap(await _json('GET', '/api/ws/url'));
    final rawUrl = asStringOrNull(ticketData['url']);
    final ticket = asStringOrNull(ticketData['ticket']);
    if (rawUrl == null || ticket == null) {
      _setStatus(GatewayStatus.offline);
      throw const FlackleException('Flackle did not return a gateway ticket.');
    }
    final base = Uri.parse(rawUrl);
    final uri = base.replace(queryParameters: {
      ...base.queryParameters,
      'ticket': ticket,
    });
    final socket = WebSocketChannel.connect(uri);
    _socket = socket;
    _socketSubscription = socket.stream.listen(
      _onSocketData,
      onError: (_) => _handleSocketClosed(),
      onDone: _handleSocketClosed,
      cancelOnError: true,
    );
    _sendFrame({
      'type': 'set_idle',
      'payload': {'idle': false}
    });
    if (serverIds.isNotEmpty) {
      _sendFrame({
        'type': 'subscribe_servers',
        'payload': {'server_ids': serverIds}
      });
    }
    _heartbeat?.cancel();
    _heartbeat = Timer.periodic(const Duration(seconds: 45), (_) {
      _sendFrame({
        'type': 'ping',
        'payload': {'idle': false}
      });
    });
  }

  Future<void> disconnect() async {
    _manualDisconnect = true;
    _reconnectTimer?.cancel();
    _reconnectTimer = null;
    _reconnectAttempts = 0;
    _heartbeat?.cancel();
    _heartbeat = null;
    await _socketSubscription?.cancel();
    _socketSubscription = null;
    final socket = _socket;
    _socket = null;
    _setStatus(GatewayStatus.offline);
    try {
      await socket?.sink.close();
    } catch (_) {}
    for (final completer in _pending.values) {
      if (!completer.isCompleted) {
        completer.completeError(
            const FlackleException('The live connection closed.'));
      }
    }
    _pending.clear();
  }

  void _handleSocketClosed() {
    _heartbeat?.cancel();
    _heartbeat = null;
    _socket = null;
    _setStatus(GatewayStatus.offline);
    for (final completer in _pending.values) {
      if (!completer.isCompleted) {
        completer.completeError(
            const FlackleException('The live connection closed.'));
      }
    }
    _pending.clear();
    if (_manualDisconnect || _disposed || _reconnectAttempts >= 8) return;
    final attempts = _reconnectAttempts > 4 ? 4 : _reconnectAttempts;
    final delay = Duration(seconds: 1 << attempts);
    _reconnectAttempts += 1;
    _reconnectTimer?.cancel();
    _reconnectTimer = Timer(delay, () {
      unawaited(connect(_gatewayServers).catchError((_) {}));
    });
  }

  void _setStatus(GatewayStatus status) {
    _status = status;
    if (!_statusController.isClosed) _statusController.add(status);
  }

  void _sendFrame(Map<String, dynamic> frame) {
    final socket = _socket;
    if (socket == null) {
      throw const FlackleException('The live connection is not ready yet.');
    }
    socket.sink.add(jsonEncode(frame));
  }

  void _onSocketData(dynamic data) {
    final text = data is String ? data : utf8.decode(data as List<int>);
    for (final line in text.split('\n')) {
      if (line.trim().isEmpty) continue;
      Map<String, dynamic> packet;
      try {
        packet = asMap(jsonDecode(line));
      } catch (_) {
        continue;
      }
      final type = asString(packet['type']);
      if (type.isEmpty) continue;
      final payload = asMap(packet['payload']);
      if (type == 'hello') {
        _reconnectAttempts = 0;
        _setStatus(GatewayStatus.connected);
      }
      if (type == 'message_ack') _handleAck(payload);
      if ((type == 'new_message' || type == 'channel_message') &&
          payload.isNotEmpty) {
        _resolvePendingMessage(payload);
      }
      if (!_events.isClosed) _events.add(GatewayEvent(type, payload));
    }
  }

  void _handleAck(Map<String, dynamic> payload) {
    final nonce = asStringOrNull(payload['nonce']);
    if (nonce == null) return;
    final completer = _pending[nonce];
    if (completer == null || completer.isCompleted) return;
    final status = asString(payload['status']);
    if (status == 'sent') {
      final message = payload['message'];
      completer
          .complete(message is Map ? Message.fromJson(asMap(message)) : null);
      return;
    }
    if (status == 'edit_ok' || status == 'delete_ok') {
      completer.complete(null);
      return;
    }
    final error =
        asStringOrNull(payload['error']) ?? 'Message action failed ($status).';
    completer.completeError(FlackleException(error));
  }

  void _resolvePendingMessage(Map<String, dynamic> payload) {
    final raw = payload['message'] is Map ? asMap(payload['message']) : payload;
    final nonce = asStringOrNull(raw['nonce']);
    if (nonce == null) return;
    final completer = _pending[nonce];
    if (completer == null || completer.isCompleted) return;
    completer.complete(Message.fromJson(raw));
  }

  // ---------------------------------------------------------------------------
  // HTTP plumbing
  // ---------------------------------------------------------------------------

  Future<dynamic> _json(
    String method,
    String path, {
    Map<String, String>? query,
    Object? body,
    bool authenticated = true,
    bool retry = true,
    bool retryCsrf = true,
  }) async {
    final response = await _request(
      method,
      path,
      query: query,
      body: body,
      authenticated: authenticated,
      retry: retry,
      retryCsrf: retryCsrf,
    );
    return _requireSuccess(response);
  }

  Future<http.Response> _request(
    String method,
    String path, {
    Map<String, String>? query,
    Object? body,
    bool authenticated = true,
    bool retry = true,
    bool retryCsrf = true,
  }) async {
    _absorbBrowserCookies();
    final uri = Uri.parse('$baseUrl$path').replace(
      queryParameters: query == null || query.isEmpty ? null : query,
    );
    final headers = <String, String>{
      'Accept': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
      'Origin': baseUrl,
      'Referer': '$baseUrl/app',
    };
    if (body != null) headers['Content-Type'] = 'application/json';
    if (!kIsWeb && _cookies.isNotEmpty) {
      final cookie = _cookies.entries
          .where((entry) => entry.value.isNotEmpty)
          .map((entry) => '${entry.key}=${entry.value}')
          .join('; ');
      if (cookie.isNotEmpty) headers['Cookie'] = cookie;
    }
    if (authenticated && _accessToken?.isNotEmpty == true) {
      headers['Authorization'] = 'Bearer $_accessToken';
    }
    if (_csrfToken?.isNotEmpty == true) headers['X-CSRF-Token'] = _csrfToken!;

    http.Response response;
    try {
      final request = http.Request(method, uri)..headers.addAll(headers);
      if (body != null) request.body = jsonEncode(body);
      response = await http.Response.fromStream(
        await _http.send(request).timeout(const Duration(seconds: 45)),
      );
    } on TimeoutException {
      throw const FlackleException('Flackle took too long to respond.');
    } catch (error) {
      throw FlackleException('Could not reach Flackle: $error');
    }

    final previous = '$_accessToken|$_refreshToken|$_csrfToken';
    _absorbCookies(response);
    final rotatedCsrf = asStringOrNull(response.headers['x-csrf-token']);
    if (rotatedCsrf != null) _csrfToken = rotatedCsrf;
    _absorbBrowserCookies();
    if (hasSession && previous != '$_accessToken|$_refreshToken|$_csrfToken') {
      await _saveSession();
    }

    if (response.statusCode == 401 &&
        authenticated &&
        retry &&
        (_refreshToken?.isNotEmpty == true || (kIsWeb && _cookieSession))) {
      await _refreshSession();
      return _request(method, path,
          query: query,
          body: body,
          authenticated: authenticated,
          retry: false,
          retryCsrf: retryCsrf);
    }
    if (response.statusCode == 403 && retryCsrf && _isCsrfFailure(response)) {
      await _ensureCsrf(force: true);
      return _request(method, path,
          query: query,
          body: body,
          authenticated: authenticated,
          retry: retry,
          retryCsrf: false);
    }
    if (response.statusCode == 429 && retry) {
      await Future<void>.delayed(const Duration(seconds: 2));
      return _request(method, path,
          query: query,
          body: body,
          authenticated: authenticated,
          retry: false,
          retryCsrf: retryCsrf);
    }
    return response;
  }

  Future<dynamic> _upload(
    String path, {
    required String field,
    required String fileName,
    required List<int> bytes,
    required String contentType,
    String method = 'POST',
    Map<String, String> extraFields = const {},
  }) async {
    await _ensureCsrf();
    final uri = Uri.parse('$baseUrl$path');
    final request = http.MultipartRequest(method, uri);
    request.headers.addAll({
      'Accept': 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
      'Origin': baseUrl,
      'Referer': '$baseUrl/app',
      if (_accessToken?.isNotEmpty == true)
        'Authorization': 'Bearer $_accessToken',
      if (_csrfToken?.isNotEmpty == true) 'X-CSRF-Token': _csrfToken!,
    });
    request.fields.addAll(extraFields);
    request.files.add(http.MultipartFile.fromBytes(
      field,
      bytes,
      filename: fileName,
      contentType: MediaType.parse(contentType),
    ));
    http.Response response;
    try {
      response = await http.Response.fromStream(
        await _http.send(request).timeout(const Duration(minutes: 5)),
      );
    } on TimeoutException {
      throw const FlackleException('The upload timed out.');
    }
    _absorbCookies(response);
    return _requireSuccess(response);
  }

  dynamic _requireSuccess(http.Response response) {
    dynamic data;
    if (response.body.isNotEmpty) {
      try {
        data = jsonDecode(response.body);
      } catch (_) {
        data = {'message': response.body};
      }
    } else {
      data = <String, dynamic>{};
    }
    if (response.statusCode < 200 || response.statusCode >= 300) {
      final map = asMap(data);
      final message = asStringOrNull(map['message']) ??
          asStringOrNull(map['error']) ??
          asStringOrNull(map['detail']) ??
          'Flackle returned an error (${response.statusCode}).';
      throw FlackleException(
        message,
        statusCode: response.statusCode,
        code: asStringOrNull(map['code']),
      );
    }
    return data;
  }

  Future<void> _ensureCsrf({bool force = false}) async {
    if (force) {
      _csrfToken = null;
      _cookies.remove('__Host-csrf');
    }
    if (_csrfToken?.isNotEmpty == true) return;
    final response = await _request('GET', '/api/auth/me',
        authenticated: _accessToken?.isNotEmpty == true,
        retry: false,
        retryCsrf: false);
    _absorbCookies(response);
    final data = _tryDecode(response);
    _csrfToken ??= asStringOrNull(asMap(data)['csrf_token']);
    _csrfToken ??= asStringOrNull(response.headers['x-csrf-token']);
    _absorbBrowserCookies();
  }

  Future<void> _refreshSession() async {
    final response = await _request(
      'POST',
      '/api/auth/refresh',
      body: {
        if (_refreshToken?.isNotEmpty == true) 'refresh_token': _refreshToken,
      },
      authenticated: false,
      retry: false,
      retryCsrf: false,
    );
    if (response.statusCode == 401 || response.statusCode == 403) {
      _accessToken = null;
      _refreshToken = null;
      _cookies.clear();
      await _clearSession();
      throw const FlackleException('Your session expired. Sign in again.');
    }
    final data = asMap(_requireSuccess(response));
    _accessToken =
        asStringOrNull(data['access_token']) ?? _cookies['access_token'];
    _refreshToken = asStringOrNull(data['refresh_token']) ?? _refreshToken;
    if (_accessToken == null || _accessToken!.isEmpty) {
      throw const FlackleException('Flackle did not refresh the session.');
    }
    await _saveSession();
  }

  bool _isCsrfFailure(http.Response response) {
    final data = _tryDecode(response);
    return asString(asMap(data)['code']) == 'csrf_invalid';
  }

  dynamic _tryDecode(http.Response response) {
    if (response.body.isEmpty) return <String, dynamic>{};
    try {
      return jsonDecode(response.body);
    } catch (_) {
      return <String, dynamic>{};
    }
  }

  void _absorbCookies(http.Response response) {
    final values = response.headers['set-cookie'];
    if (values == null || values.isEmpty) return;
    final chunks = values.split(RegExp(r',(?=\s*[^;,\s]+=)'));
    for (final chunk in chunks) {
      final first = chunk.split(';').first.trim();
      final separator = first.indexOf('=');
      if (separator <= 0) continue;
      final name = first.substring(0, separator).trim();
      final value = first.substring(separator + 1).trim().replaceAll('"', '');
      final lower = chunk.toLowerCase();
      final expired = lower.contains('max-age=0') ||
          lower.contains('expires=thu, 01 jan 1970') ||
          lower.contains('expires=wed, 21 oct 2015');
      if (expired) {
        _cookies.remove(name);
        if (name == 'access_token') _accessToken = null;
        if (name == 'refresh_token') _refreshToken = null;
        if (name == '__Host-csrf') _csrfToken = null;
        continue;
      }
      _cookies[name] = value;
      if (name == 'access_token') _accessToken = value;
      if (name == 'refresh_token') _refreshToken = value;
      if (name == '__Host-csrf') _csrfToken = value;
    }
    _syncCookies();
  }

  List<dynamic> _list(dynamic data, String key) {
    if (data is List) return data;
    final map = asMap(data);
    final value = map[key];
    return value is List ? value : const [];
  }

  String _newNonce() =>
      '${DateTime.now().microsecondsSinceEpoch.toRadixString(16)}${_nonceCounter++}';

  static int _nonceCounter = 0;

  String _contentTypeFor(String fileName) {
    final lower = fileName.toLowerCase();
    if (lower.endsWith('.png')) return 'image/png';
    if (lower.endsWith('.jpg') || lower.endsWith('.jpeg')) return 'image/jpeg';
    if (lower.endsWith('.gif')) return 'image/gif';
    if (lower.endsWith('.webp')) return 'image/webp';
    if (lower.endsWith('.mp4')) return 'video/mp4';
    if (lower.endsWith('.webm')) return 'video/webm';
    if (lower.endsWith('.mp3')) return 'audio/mpeg';
    if (lower.endsWith('.ogg')) return 'audio/ogg';
    if (lower.endsWith('.pdf')) return 'application/pdf';
    if (lower.endsWith('.zip')) return 'application/zip';
    if (lower.endsWith('.json')) return 'application/json';
    if (lower.endsWith('.txt')) return 'text/plain';
    return 'application/octet-stream';
  }

  Future<void> dispose() async {
    _disposed = true;
    await disconnect();
    await _events.close();
    await _statusController.close();
    _http.close();
  }
}
