import 'dart:convert';
import 'dart:math' as math;

import 'package:characters/characters.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../core/format.dart';
import '../core/models.dart';
import '../core/permissions.dart';
import '../core/theme.dart';
import 'icons.dart';

class EmojiEntry {
  const EmojiEntry({
    required this.char,
    required this.name,
    required this.keywords,
    required this.file,
  });

  final String char;
  final String name;
  final String keywords;
  final String file;
}

class EmojiCategory {
  const EmojiCategory(
      {required this.id, required this.label, required this.entries});

  final String id;
  final String label;
  final List<EmojiEntry> entries;
}

class EmojiIndex {
  static List<EmojiCategory> categories = const [];
  static final Map<String, EmojiEntry> _byChar = {};
  static final Map<String, EmojiEntry> _byBase = {};
  static final Set<String> _files = {};
  static List<String> recent = const [];
  static bool loaded = false;

  static Future<void> load() async {
    if (loaded) return;
    try {
      final raw = await rootBundle.loadString('assets/emoji_data.json');
      final list = jsonDecode(raw);
      final parsed = <EmojiCategory>[];
      if (list is List) {
        for (final value in list.whereType<Map>()) {
          final entries = <EmojiEntry>[];
          for (final entry
              in (value['entries'] as List? ?? const []).whereType<Map>()) {
            final char = '${entry['char'] ?? ''}';
            if (char.isEmpty) continue;
            final item = EmojiEntry(
              char: char,
              name: '${entry['name'] ?? ''}',
              keywords: '${entry['keywords'] ?? ''}',
              file: '${entry['file'] ?? ''}',
            );
            entries.add(item);
            _byChar[char] = item;
            _files.add(item.file);
            final base = _stripVariation(char);
            _byBase.putIfAbsent(base, () => item);
          }
          parsed.add(EmojiCategory(
            id: '${value['id'] ?? ''}',
            label: '${value['label'] ?? ''}',
            entries: entries,
          ));
        }
      }
      categories = parsed;
      final prefs = await SharedPreferences.getInstance();
      recent = (prefs.getStringList('flackle.emoji.recent') ?? const [])
          .take(24)
          .toList();
    } catch (_) {
      categories = const [];
    }
    loaded = true;
  }

  static String _stripVariation(String value) =>
      value.replaceAll('\uFE0F', '').replaceAll('\uFE0E', '');

  static String? assetFor(String grapheme) {
    if (grapheme.isEmpty) return null;
    final entry = _byChar[grapheme] ?? _byBase[_stripVariation(grapheme)];
    if (entry != null) return 'assets/emoji/${entry.file}';
    final code = _codeFor(grapheme);
    final candidate = '$code.svg';
    if (_files.contains(candidate)) return 'assets/emoji/$candidate';
    return null;
  }

  static String _codeFor(String value) => value.runes
      .where((code) => code != 0xFE0F && code != 0xFE0E)
      .map((code) => code.toRadixString(16))
      .join('-');

  static bool isEmoji(String grapheme) => assetFor(grapheme) != null;

  static String nameFor(String grapheme) =>
      _byChar[grapheme]?.name ??
      _byBase[_stripVariation(grapheme)]?.name ??
      grapheme;

  static List<EmojiEntry> search(String query) {
    final term = query.trim().toLowerCase();
    if (term.isEmpty) return const [];
    final seen = <String>{};
    final results = <EmojiEntry>[];
    for (final category in categories) {
      for (final entry in category.entries) {
        if (seen.contains(entry.char)) continue;
        if (entry.name.toLowerCase().contains(term) ||
            entry.keywords.toLowerCase().contains(term) ||
            entry.char == term) {
          seen.add(entry.char);
          results.add(entry);
          if (results.length >= 120) return results;
        }
      }
    }
    return results;
  }

  static List<EmojiEntry> recentEntries() {
    final entries = <EmojiEntry>[];
    for (final char in recent) {
      final entry = _byChar[char] ?? _byBase[_stripVariation(char)];
      if (entry != null) entries.add(entry);
    }
    return entries;
  }

  static Future<void> pushRecent(String char) async {
    recent =
        [char, ...recent.where((value) => value != char)].take(24).toList();
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setStringList('flackle.emoji.recent', recent);
    } catch (_) {}
  }
}

/// One emoji rendered from the bundled Twemoji set.
class EmojiGlyph extends StatelessWidget {
  const EmojiGlyph(this.char, {this.size = 18, super.key});

  final String char;
  final double size;

  @override
  Widget build(BuildContext context) {
    final asset = EmojiIndex.assetFor(char);
    if (asset == null || !BrandAssets.has(asset)) {
      return Text(char, style: TextStyle(fontSize: size * 0.95, height: 1));
    }
    return SizedBox(
      width: size,
      height: size,
      child: SvgPicture.asset(asset, width: size, height: size),
    );
  }
}

/// Renders text, swapping bundled emoji into inline SVG widgets.
class EmojiText extends StatelessWidget {
  const EmojiText(
    this.text, {
    this.style,
    this.maxLines,
    this.overflow = TextOverflow.ellipsis,
    this.textAlign,
    super.key,
  });

  final String text;
  final TextStyle? style;
  final int? maxLines;
  final TextOverflow overflow;
  final TextAlign? textAlign;

  @override
  Widget build(BuildContext context) {
    final base = style ?? context.label(14, color: context.c.textDim);
    final size = (base.fontSize ?? 14) * 1.28;
    final spans = <InlineSpan>[];
    final buffer = StringBuffer();

    void flush() {
      if (buffer.isEmpty) return;
      spans.add(TextSpan(text: buffer.toString(), style: base));
      buffer.clear();
    }

    for (final grapheme in text.characters) {
      if (EmojiIndex.isEmoji(grapheme)) {
        flush();
        spans.add(WidgetSpan(
          alignment: PlaceholderAlignment.middle,
          child: EmojiGlyph(grapheme, size: size),
        ));
      } else {
        buffer.write(grapheme);
      }
    }
    flush();

    return RichText(
      text: TextSpan(children: spans),
      maxLines: maxLines,
      overflow: overflow,
      textAlign: textAlign ?? TextAlign.start,
    );
  }
}

class MentionResolver {
  const MentionResolver(
      {this.user, this.role, this.channel, this.onUser, this.onChannel});

  final User? Function(String id)? user;
  final Role? Function(String id)? role;
  final Channel? Function(String id)? channel;
  final void Function(User user)? onUser;
  final void Function(String channelId)? onChannel;
}

/// Rich message rendering: Flackle markup, mentions, links and Twemoji.
class MessageText extends StatefulWidget {
  const MessageText({
    required this.content,
    required this.style,
    this.resolver,
    this.maxLines,
    this.overflow = TextOverflow.clip,
    this.onOpenUrl,
    this.onOpenChannel,
    this.emojiScale = 1.28,
    super.key,
  });

  final String content;
  final TextStyle style;
  final MentionResolver? resolver;
  final int? maxLines;
  final TextOverflow overflow;
  final void Function(String url)? onOpenUrl;
  final void Function(String channelId)? onOpenChannel;
  final double emojiScale;

  @override
  State<MessageText> createState() => _MessageTextState();
}

class _MessageTextState extends State<MessageText> {
  final List<TapGestureRecognizerHolder> _recognizers = [];

  @override
  void dispose() {
    for (final holder in _recognizers) {
      holder.dispose();
    }
    super.dispose();
  }

  TapGestureRecognizerHolder _recognizer(VoidCallback onTap) {
    final holder = TapGestureRecognizerHolder(onTap);
    _recognizers.add(holder);
    return holder;
  }

  @override
  Widget build(BuildContext context) {
    final c = context.c;
    final tokens = parseMessageContent(widget.content);
    final spans = <InlineSpan>[];
    final buffer = StringBuffer();
    var bufferStyle = widget.style;

    void flush() {
      if (buffer.isEmpty) return;
      _appendEmojiText(
          spans, buffer.toString(), bufferStyle, widget.emojiScale);
      buffer.clear();
    }

    for (final token in tokens) {
      if (token.newline) {
        flush();
        spans.add(const TextSpan(text: '\n'));
        continue;
      }
      if (token.mention != null) {
        flush();
        final mentionStyle = widget.style.copyWith(fontWeight: FontWeight.w600);
        switch (token.mention!) {
          case MentionKind.user:
            final user = widget.resolver?.user?.call(token.mentionId ?? '');
            final holder = _recognizer(() {
              if (user != null) widget.resolver?.onUser?.call(user);
            });
            spans.add(TextSpan(
              text: '@${user?.name ?? 'user'}',
              style: mentionStyle.copyWith(
                color: c.accent,
                backgroundColor: c.accentDim,
                decoration: TextDecoration.none,
              ),
              recognizer: holder.recognizer,
            ));
          case MentionKind.role:
            final role = widget.resolver?.role?.call(token.mentionId ?? '');
            final color = _parseColor(role?.color) ?? c.textDim;
            spans.add(TextSpan(
              text: '@${role?.name ?? 'role'}',
              style: mentionStyle.copyWith(
                color: color,
                backgroundColor: color.withValues(alpha: 0.14),
              ),
            ));
          case MentionKind.channel:
            final channel =
                widget.resolver?.channel?.call(token.mentionId ?? '');
            final holder = _recognizer(() {
              widget.onOpenChannel?.call(token.mentionId ?? '');
              widget.resolver?.onChannel?.call(token.mentionId ?? '');
            });
            spans.add(TextSpan(
              text: '#${channel?.name ?? 'channel'}',
              style: mentionStyle.copyWith(
                color: c.accent,
                backgroundColor: c.accentDim,
              ),
              recognizer: holder.recognizer,
            ));
        }
        bufferStyle = widget.style;
        continue;
      }
      if (token.url != null) {
        flush();
        final holder = _recognizer(() => widget.onOpenUrl?.call(token.url!));
        spans.add(TextSpan(
          text: token.text,
          style: widget.style.copyWith(
            color: c.accent,
            decoration: TextDecoration.underline,
            decorationColor: c.accent.withValues(alpha: 0.5),
          ),
          recognizer: holder.recognizer,
        ));
        bufferStyle = widget.style;
        continue;
      }
      var style = widget.style;
      if (token.styles.contains(RichStyle.bold)) {
        style = style.copyWith(fontWeight: FontWeight.w700);
      }
      if (token.styles.contains(RichStyle.italic)) {
        style = style.copyWith(fontStyle: FontStyle.italic);
      }
      if (token.styles.contains(RichStyle.underline)) {
        style = style.copyWith(decoration: TextDecoration.underline);
      }
      if (token.styles.contains(RichStyle.strike)) {
        style = style.copyWith(decoration: TextDecoration.lineThrough);
      }
      if (token.styles.contains(RichStyle.code) ||
          token.styles.contains(RichStyle.block)) {
        style = style.copyWith(
          fontFamily: 'monospace',
          fontSize: (style.fontSize ?? 14) * 0.92,
          backgroundColor: c.panel3,
          height: 1.45,
        );
      }
      if (style != bufferStyle) flush();
      bufferStyle = style;
      buffer.write(token.text);
    }
    flush();

    return RichText(
      text: TextSpan(style: widget.style, children: spans),
      maxLines: widget.maxLines,
      overflow: widget.overflow,
    );
  }

  void _appendEmojiText(
      List<InlineSpan> spans, String text, TextStyle style, double scale) {
    final buffer = StringBuffer();
    final size = (style.fontSize ?? 14) * scale;

    void flush() {
      if (buffer.isEmpty) return;
      spans.add(TextSpan(text: buffer.toString(), style: style));
      buffer.clear();
    }

    for (final grapheme in text.characters) {
      if (EmojiIndex.isEmoji(grapheme)) {
        flush();
        spans.add(WidgetSpan(
          alignment: PlaceholderAlignment.middle,
          child: EmojiGlyph(grapheme, size: size),
        ));
      } else {
        buffer.write(grapheme);
      }
    }
    flush();
  }
}

class TapGestureRecognizerHolder {
  TapGestureRecognizerHolder(VoidCallback onTap)
      : recognizer = (TapGestureRecognizer()..onTap = onTap);

  final TapGestureRecognizer recognizer;

  void dispose() => recognizer.dispose();
}

Color? _parseColor(String? hex) {
  if (hex == null || !hex.startsWith('#')) return null;
  final value = hex.substring(1);
  final normalized = value.length == 3
      ? value.split('').map((char) => '$char$char').join()
      : value.padRight(6, '0').substring(0, 6);
  final parsed = int.tryParse(normalized, radix: 16);
  return parsed == null ? null : Color(0xFF000000 | parsed);
}

/// The emoji picker used by the composer and reaction menus.
Future<String?> showEmojiPicker(
  BuildContext context, {
  String title = 'Emoji',
  bool allowSearch = true,
}) async {
  if (EmojiIndex.categories.isEmpty) await EmojiIndex.load();
  if (!context.mounted) return null;
  return showModalBottomSheet<String>(
    context: context,
    isScrollControlled: true,
    backgroundColor: context.c.panel,
    builder: (context) =>
        _EmojiPickerSheet(title: title, allowSearch: allowSearch),
  );
}

class _EmojiPickerSheet extends StatefulWidget {
  const _EmojiPickerSheet({required this.title, required this.allowSearch});

  final String title;
  final bool allowSearch;

  @override
  State<_EmojiPickerSheet> createState() => _EmojiPickerSheetState();
}

class _EmojiPickerSheetState extends State<_EmojiPickerSheet> {
  final _search = TextEditingController();
  String _category = 'recent';
  String _query = '';

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  List<EmojiEntry> get _entries {
    if (_query.trim().isNotEmpty) return EmojiIndex.search(_query);
    if (_category == 'recent') {
      final recent = EmojiIndex.recentEntries();
      if (recent.isNotEmpty) return recent;
    }
    final category = EmojiIndex.categories.firstWhere(
      (entry) => entry.id == _category,
      orElse: () => EmojiIndex.categories.isEmpty
          ? const EmojiCategory(id: '', label: '', entries: [])
          : EmojiIndex.categories.first,
    );
    return category.entries;
  }

  Future<void> _pick(EmojiEntry entry) async {
    await EmojiIndex.pushRecent(entry.char);
    if (mounted) Navigator.of(context).pop(entry.char);
  }

  @override
  Widget build(BuildContext context) {
    final c = context.c;
    final height = MediaQuery.sizeOf(context).height * 0.62;
    final entries = _entries;
    return SizedBox(
      height: height,
      child: Column(
        children: [
          const SizedBox(height: 10),
          Container(
            width: 36,
            height: 4,
            decoration: BoxDecoration(
              color: c.lineStrong,
              borderRadius: BorderRadius.circular(99),
            ),
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 12, 8, 8),
            child: Row(
              children: [
                Text(widget.title,
                    style: TextStyle(
                        fontFamily: 'Satoshi',
                        fontSize: 16,
                        fontWeight: FontWeight.w600,
                        color: c.text)),
                const Spacer(),
                IconButton(
                  onPressed: () => Navigator.of(context).pop(),
                  icon: const Icon(Icons.close_rounded, size: 19),
                ),
              ],
            ),
          ),
          if (widget.allowSearch)
            Padding(
              padding: const EdgeInsets.fromLTRB(14, 0, 14, 8),
              child: TextField(
                controller: _search,
                autocorrect: false,
                onChanged: (value) => setState(() => _query = value),
                decoration: const InputDecoration(
                  hintText: 'Search emoji',
                  prefixIcon: Icon(Icons.search_rounded, size: 18),
                  isDense: true,
                ),
              ),
            ),
          SizedBox(
            height: 36,
            child: ListView(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.symmetric(horizontal: 12),
              children: [
                _chip('recent', 'Recent'),
                for (final category in EmojiIndex.categories)
                  _chip(category.id, category.label),
              ],
            ),
          ),
          const SizedBox(height: 6),
          Expanded(
            child: entries.isEmpty
                ? Center(
                    child: Text('No emoji found',
                        style: TextStyle(color: c.muted, fontSize: 13)),
                  )
                : GridView.builder(
                    padding: const EdgeInsets.fromLTRB(12, 4, 12, 20),
                    gridDelegate:
                        const SliverGridDelegateWithMaxCrossAxisExtent(
                      maxCrossAxisExtent: 46,
                      mainAxisExtent: 42,
                    ),
                    itemCount: entries.length,
                    itemBuilder: (context, index) {
                      final entry = entries[index];
                      return Tooltip(
                        message: entry.name.replaceAll('_', ' '),
                        waitDuration: const Duration(milliseconds: 400),
                        child: InkWell(
                          borderRadius: BorderRadius.circular(8),
                          onTap: () => _pick(entry),
                          child:
                              Center(child: EmojiGlyph(entry.char, size: 24)),
                        ),
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }

  Widget _chip(String id, String label) => Padding(
        padding: const EdgeInsets.only(right: 6),
        child: ChoiceChip(
          selected: _query.isEmpty && _category == id,
          label: Text(label),
          onSelected: (_) => setState(() {
            _category = id;
            _query = '';
            _search.clear();
          }),
          visualDensity: VisualDensity.compact,
          showCheckmark: false,
        ),
      );
}

/// Compact "who reacted" summary using bundled emoji and names.
class ReactionSummary extends StatelessWidget {
  const ReactionSummary({required this.reaction, super.key});

  final Reaction reaction;

  @override
  Widget build(BuildContext context) {
    final c = context.c;
    final names = reaction.users.map((user) => user.name).toList();
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 4),
      decoration: BoxDecoration(
        color: reaction.me ? c.accentDim : c.panel2,
        borderRadius: BorderRadius.circular(9),
        border: Border.all(
            color: reaction.me ? c.accent.withValues(alpha: 0.4) : c.line),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          EmojiGlyph(reaction.emoji, size: 15),
          const SizedBox(width: 4),
          Text(
            '${reaction.count}',
            style: TextStyle(
              fontSize: 11,
              fontWeight: FontWeight.w600,
              color: reaction.me ? c.accent : c.textDim,
            ),
          ),
          if (names.isNotEmpty) ...[
            const SizedBox(width: 5),
            ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 110),
              child: Text(
                names.take(2).join(', '),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(fontSize: 10, color: c.muted),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

String formatEmojiName(String name) => name.replaceAll('_', ' ');

String emojiOnlyScale(String content) {
  final graphemes = content.characters.toList();
  if (graphemes.isEmpty || graphemes.length > 3) return 'normal';
  final allEmoji = graphemes.every(
      (grapheme) => EmojiIndex.isEmoji(grapheme) || grapheme.trim().isEmpty);
  return allEmoji ? 'jumbo' : 'normal';
}

double emojiOnlyFontSize(String content) {
  final count = content.characters.where((g) => g.trim().isNotEmpty).length;
  return math.min(44, 30 + count * 4).toDouble();
}

bool hasPermissionValue(String? value, int bit) => Permission.has(value, bit);
