import type {
  Active,
  Attachment,
  AuthSession,
  BackupCodes,
  Bootstrap,
  Channel,
  ChannelBundle,
  ChannelUpdate,
  Conversation,
  CreateChannel,
  LoginOutcome,
  Member,
  MemberListSnapshot,
  AcceptInviteResult,
  AuditLogPage,
  CreateServerResult,
  FriendRequests,
  Invite,
  LiftsInfo,
  Message,
  MessagePage,
  Role,
  RoleBatchPayload,
  RoleBatchResponse,
  RolesResponse,
  SavedMediaPage,
  SecurityEvent,
  SelfUser,
  Server,
  ServerBan,
  ServerInvite,
  ServerSettingsBundle,
  ServerUpdate,
  Target,
  TwoFactorSetup,
  TwoFactorStatus,
  UnreadSummary,
  User,
} from "./types";
import { ALL_PERMISSIONS, Permission } from "./permissions";
import { demoBootstrap, demoChannels, demoMemberSnapshot, demoMembers, demoMessages, demoRoles } from "./demo";

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export { inTauri };

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}

export async function login(email: string, password: string): Promise<LoginOutcome> {
  if (!inTauri) {
    await wait(700);
    if (password.length < 4) throw new Error("Incorrect email or password.");
    return { status: "success", user: demoBootstrap.me };
  }
  return call("login", { email, password });
}

export async function loginMfa(
  mfaToken: string,
  code: string,
  backupCode = false,
): Promise<SelfUser> {
  if (!inTauri) {
    await wait(500);
    return demoBootstrap.me;
  }
  return call("login_mfa", { mfaToken, code, backupCode });
}

export async function restore(): Promise<SelfUser | null> {
  if (!inTauri) return demoBootstrap.me;
  return call("restore");
}

export async function logout(): Promise<void> {
  if (!inTauri) return;
  return call("logout");
}

export async function bootstrap(): Promise<Bootstrap> {
  if (!inTauri) {
    await wait(250);
    return demoBootstrap;
  }
  return call("bootstrap");
}

export async function fetchServers(): Promise<Server[]> {
  if (!inTauri) return demoBootstrap.servers;
  return call("fetch_servers");
}

export async function fetchServer(serverId: string): Promise<Server> {
  if (!inTauri) {
    const server = demoBootstrap.servers.find((entry) => entry.id === serverId);
    if (!server) throw new Error("Server not found.");
    return server;
  }
  return call("fetch_server", { serverId });
}

export async function fetchChannels(serverId: string): Promise<ChannelBundle> {
  if (!inTauri) {
    await wait(120);
    return demoChannels(serverId);
  }
  return call("fetch_channels", { serverId });
}

export async function fetchConversations(): Promise<Conversation[]> {
  if (!inTauri) return demoBootstrap.conversations;
  return call("fetch_conversations");
}

export async function fetchFriends(): Promise<User[]> {
  if (!inTauri) return demoMembers.slice(1, 3).map((member) => member.user);
  return call("fetch_friends");
}

export async function fetchBlocked(): Promise<User[]> {
  if (!inTauri) return [];
  return call("fetch_blocked");
}

export async function fetchFriendRequests(): Promise<FriendRequests> {
  if (!inTauri) return { incoming: [], outgoing: [] };
  return call("fetch_friend_requests");
}

export async function addFriend(username: string): Promise<void> {
  if (!inTauri) return;
  return call("add_friend", { username });
}

export async function acceptFriendRequest(requestId: string): Promise<void> {
  if (!inTauri) return;
  return call("accept_friend_request", { requestId });
}

export async function rejectFriendRequest(requestId: string): Promise<void> {
  if (!inTauri) return;
  return call("reject_friend_request", { requestId });
}

export async function cancelFriendRequest(requestId: string): Promise<void> {
  if (!inTauri) return;
  return call("cancel_friend_request", { requestId });
}

export async function removeFriend(userId: string): Promise<void> {
  if (!inTauri) return;
  return call("remove_friend", { userId });
}

export async function createDM(userId: string): Promise<Conversation> {
  if (!inTauri) {
    await wait(200);
    return {
      id: `demo-dm-${userId}`,
      type: "direct",
      members: [{ user: demoBootstrap.me }],
    };
  }
  return call("create_dm", { userId });
}

export async function fetchSessions(): Promise<AuthSession[]> {
  if (!inTauri) return [];
  const data = await call<{ sessions?: AuthSession[] }>("fetch_sessions");
  return data.sessions ?? [];
}

export async function revokeSession(sessionId: string, code?: string): Promise<void> {
  if (!inTauri) return;
  return call("revoke_session", { sessionId, code: code ?? null });
}

export async function revokeAllSessions(code?: string): Promise<void> {
  if (!inTauri) return;
  return call("revoke_all_sessions", { code: code ?? null });
}

export async function fetchSecurityEvents(): Promise<SecurityEvent[]> {
  if (!inTauri) return [];
  const data = await call<{ events?: SecurityEvent[] }>("fetch_security_events");
  return data.events ?? [];
}

export async function clearSecurityEvents(code?: string): Promise<void> {
  if (!inTauri) return;
  return call("clear_security_events", { code: code ?? null });
}

export async function fetch2FAStatus(): Promise<TwoFactorStatus> {
  if (!inTauri) {
    return { enabled: false, backup_codes_remaining: 0, pending: false };
  }
  return call("fetch_2fa_status");
}

export async function setup2FA(password: string): Promise<TwoFactorSetup> {
  if (!inTauri) {
    return {
      secret: "DEMO2FASECRET",
      otpauth_url: "otpauth://totp/Flackle:demo?secret=DEMO2FASECRET",
    };
  }
  return call("setup_2fa", { password });
}

export async function confirm2FA(code: string): Promise<unknown> {
  if (!inTauri) return {};
  return call("confirm_2fa", { code });
}

export async function disable2FA(
  password: string,
  code?: string,
  backupCode?: string,
): Promise<unknown> {
  if (!inTauri) return {};
  return call("disable_2fa", {
    password,
    code: code ?? null,
    backupCode: backupCode ?? null,
  });
}

export async function createBackupCodes(
  password: string,
  code: string,
): Promise<BackupCodes> {
  if (!inTauri) return { codes: ["DEMO-CODE-1", "DEMO-CODE-2"] };
  return call("create_backup_codes", { password, code });
}

export async function updateServer(serverId: string, patch: ServerUpdate): Promise<Server> {
  if (!inTauri) {
    await wait(200);
    const server = demoBootstrap.servers.find((item) => item.id === serverId);
    if (!server) throw new Error("Server not found.");
    Object.assign(server, patch);
    return server;
  }
  return call("update_server", { serverId, patch });
}

export async function uploadServerIcon(serverId: string, path: string): Promise<string> {
  if (!inTauri) {
    await wait(500);
    return "/brand/flackle-icon.png";
  }
  return call("upload_server_icon", { serverId, path });
}

export async function createChannel(serverId: string, options: CreateChannel): Promise<Channel> {
  if (!inTauri) {
    await wait(200);
    return {
      id: `demo-${Date.now()}`,
      name: options.name,
      type: options.type ?? "text",
      topic: options.topic ?? null,
      category_id: options.category_id ?? null,
      position: 0,
    };
  }
  return call("create_channel", { serverId, options });
}

export async function createCategory(serverId: string, name: string) {
  if (!inTauri) {
    await wait(200);
    return { id: `demo-cat-${Date.now()}`, name, position: 0 };
  }
  return call("create_category", { serverId, name });
}

export async function updateCategory(serverId: string, categoryId: string, name: string) {
  if (!inTauri) {
    await wait(200);
    return { id: categoryId, name, position: 0 };
  }
  return call("update_category", { serverId, categoryId, patch: { name } });
}

export async function deleteCategory(serverId: string, categoryId: string): Promise<void> {
  if (!inTauri) return;
  return call("delete_category", { serverId, categoryId });
}

export async function updateChannel(
  serverId: string,
  channelId: string,
  patch: ChannelUpdate,
) {
  if (!inTauri) {
    await wait(200);
    return { id: channelId, name: patch.name ?? "channel", type: "text", ...patch };
  }
  return call("update_channel", { serverId, channelId, patch });
}

export async function updateChannelPositions(
  serverId: string,
  channels: Array<{ id: string; position: number; category_id?: string | null }>,
): Promise<void> {
  if (!inTauri) return;
  return call("update_channel_positions", { serverId, channels });
}

export async function deleteChannel(serverId: string, channelId: string): Promise<void> {
  if (!inTauri) {
    await wait(200);
    return;
  }
  return call("delete_channel", { serverId, channelId });
}

export async function fetchRoles(serverId: string): Promise<RolesResponse> {
  if (!inTauri) return { roles: demoRoles, can_view_role_permissions: true };
  return call("fetch_roles", { serverId });
}

export async function batchRoles(
  serverId: string,
  batch: RoleBatchPayload,
): Promise<RoleBatchResponse> {
  if (!inTauri) {
    await wait(200);
    return { roles: demoRoles, created_role_ids: {} };
  }
  return call("update_roles", { serverId, batch });
}

export async function uploadRoleIcon(
  serverId: string,
  roleId: string,
  path: string,
): Promise<RoleBatchResponse> {
  if (!inTauri) {
    await wait(300);
    return { roles: demoRoles };
  }
  return call("upload_role_icon", { serverId, roleId, path });
}

export async function fetchServerInvites(serverId: string): Promise<ServerInvite[]> {
  if (!inTauri) return [];
  const data = await call<{ invites?: ServerInvite[] }>("fetch_server_invites", { serverId });
  return data.invites ?? [];
}

export async function deleteServerInvite(serverId: string, code: string): Promise<void> {
  if (!inTauri) return;
  return call("delete_server_invite", { serverId, code });
}

export async function fetchBans(serverId: string): Promise<ServerBan[]> {
  if (!inTauri) return [];
  const data = await call<{ bans?: ServerBan[] }>("fetch_bans", { serverId });
  return data.bans ?? [];
}

export async function kickMember(
  serverId: string,
  userId: string,
  reason?: string,
): Promise<void> {
  if (!inTauri) return;
  return call("kick_member", { serverId, userId, reason: reason ?? null });
}

export async function banMember(
  serverId: string,
  userId: string,
  reason?: string,
): Promise<void> {
  if (!inTauri) return;
  return call("ban_member", { serverId, userId, reason: reason ?? null });
}

export async function unbanMember(serverId: string, userId: string): Promise<void> {
  if (!inTauri) return;
  return call("unban_member", { serverId, userId });
}

export async function fetchAuditLog(
  serverId: string,
  limit = 50,
  before?: string | null,
): Promise<AuditLogPage> {
  if (!inTauri) return { entries: [], has_more: false };
  return call("fetch_audit_log", { serverId, limit, before: before ?? null });
}

export async function fetchLifts(serverId: string): Promise<LiftsInfo> {
  if (!inTauri) {
    return {
      server_id: serverId,
      level: 0,
      count: 0,
      next_level_at: 2,
      stream_fps: { "720p": 30 },
      max_upload_bytes: 52428800,
      saved_budget_bytes: 1073741824,
      retention_multiplier: 1,
      levels: [],
    };
  }
  return call("fetch_lifts", { serverId });
}

export async function fetchLiftSupporters(serverId: string): Promise<User[]> {
  if (!inTauri) return [];
  const data = await call<{ supporters?: User[] }>("fetch_lift_supporters", { serverId });
  return data.supporters ?? [];
}

export async function fetchSavedMedia(serverId: string): Promise<SavedMediaPage> {
  if (!inTauri) return { items: [], count: 0, used: 0, budget: 1073741824 };
  return call("fetch_saved_media", { serverId });
}

export async function fetchServerPermissions(serverId: string): Promise<string> {
  if (!inTauri) return (ALL_PERMISSIONS & ~Permission.Administrator).toString();
  return call("fetch_server_permissions", { serverId });
}

export async function fetchServerSettings(): Promise<ServerSettingsBundle> {
  if (!inTauri) {
    return {
      server_settings: demoBootstrap.servers.map((server) => ({
        server_id: server.id,
        notification_level: "mentions",
        last_viewed_channel_id: null,
        collapsed_categories: [],
      })),
    };
  }
  return call("fetch_server_settings");
}

export async function fetchChannelPermissions(
  serverId: string,
  channelId: string,
): Promise<string> {
  if (!inTauri) {
    const base = ALL_PERMISSIONS & ~Permission.Administrator;
    if (channelId === "300000000000000005") {
      return (base & ~(Permission.SendMessages | Permission.AttachFiles)).toString();
    }
    return base.toString();
  }
  return call("fetch_channel_permissions", { serverId, channelId });
}

export async function fetchUnread(): Promise<UnreadSummary> {
  if (!inTauri) return demoBootstrap.unread;
  return call("fetch_unread");
}

export async function fetchMessages(
  target: Target,
  before?: string | null,
  limit = 50,
): Promise<MessagePage> {
  if (!inTauri) return demoMessages(target, before);
  return call("fetch_messages", { target, before: before ?? null, limit });
}

export async function fetchMessagesAround(
  target: Target,
  around: string,
  limit = 50,
): Promise<MessagePage> {
  if (!inTauri) return demoMessages(target, around);
  return call("fetch_messages_around", { target, around, limit });
}

export async function sendMessage(
  target: Target,
  content: string,
  attachments: Attachment[] = [],
  replyTo?: string | null,
): Promise<Message> {
  if (!inTauri) {
    await wait(220);
    return {
      id: `local-${Date.now()}`,
      content,
      type: "text",
      is_deleted: false,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      sender: demoBootstrap.me,
      attachments,
      reply_to: replyTo
        ? {
            id: replyTo,
            content: "",
            sender_id: demoBootstrap.me.id,
            sender_username: demoBootstrap.me.username,
          }
        : null,
    };
  }
  return call("send_message", {
    target,
    content,
    attachments,
    replyTo: replyTo ?? null,
  });
}

export async function editMessage(
  target: Target,
  messageId: string,
  content: string,
): Promise<void> {
  if (!inTauri) return;
  return call("edit_message", { target, messageId, content });
}

export async function deleteMessage(target: Target, messageId: string): Promise<void> {
  if (!inTauri) return;
  return call("delete_message", { target, messageId });
}

export async function sendTyping(target: Target): Promise<void> {
  if (!inTauri) return;
  return call("send_typing", { target });
}

export async function markRead(target: Target, messageId: string): Promise<void> {
  if (!inTauri) return;
  return call("mark_read", { target, messageId });
}

export async function react(
  target: Target,
  messageId: string,
  emoji: string,
): Promise<void> {
  if (!inTauri) return;
  return call("react", { target, messageId, emoji });
}

export async function unreact(
  target: Target,
  messageId: string,
  emoji: string,
): Promise<void> {
  if (!inTauri) return;
  return call("unreact", { target, messageId, emoji });
}

export async function fetchMembers(serverId: string): Promise<Member[]> {
  if (!inTauri) return demoMembers;
  return call("fetch_members", { serverId });
}

export async function setMemberRoles(
  serverId: string,
  userId: string,
  add: string[],
  remove: string[],
): Promise<Member> {
  if (!inTauri) {
    await wait(200);
    const member = demoMembers.find((entry) => entry.user.id === userId) ?? demoMembers[0];
    const ids = new Set(member.role_ids ?? []);
    add.forEach((id) => ids.add(id));
    remove.forEach((id) => ids.delete(id));
    member.role_ids = [...ids];
    return { ...member };
  }
  return call("set_member_roles", { serverId, userId, add, remove });
}

export async function fetchMemberSnapshot(
  serverId: string,
  channelId: string,
): Promise<MemberListSnapshot> {
  if (!inTauri) {
    return demoMemberSnapshot();
  }
  return call("fetch_member_snapshot", { serverId, channelId });
}

export async function acceptInvite(code: string): Promise<AcceptInviteResult> {
  if (!inTauri) {
    await wait(400);
    throw new Error("Invite links need a live account.");
  }
  return call("accept_invite", { code });
}

export async function createServer(name: string): Promise<CreateServerResult> {
  if (!inTauri) {
    await wait(400);
    throw new Error("Creating servers needs a live account.");
  }
  return call("create_server", { name });
}

export async function markServerRead(serverId: string): Promise<void> {
  if (!inTauri) return;
  return call("mark_server_read", { serverId });
}

export async function updateServerUserSettings(
  serverId: string,
  patch: Record<string, unknown>,
): Promise<void> {
  if (!inTauri) return;
  return call("update_server_user_settings", { serverId, patch });
}

export async function updateServerPositions(
  servers: Array<{ server_id: string; position: number }>,
): Promise<void> {
  if (!inTauri) return;
  return call("update_server_positions", { servers });
}

export async function leaveServer(serverId: string): Promise<void> {
  if (!inTauri) return;
  return call("leave_server", { serverId });
}

export async function createServerInvite(
  serverId: string,
  options: Record<string, unknown> = {},
): Promise<Invite> {
  if (!inTauri) {
    await wait(300);
    return { code: "DEMO1234", url: "https://flackle.com/invite/DEMO1234" };
  }
  return call("create_server_invite", { serverId, options });
}

export async function blockUser(userId: string): Promise<void> {
  if (!inTauri) return;
  return call("block_user", { userId });
}

export async function unblockUser(userId: string): Promise<void> {
  if (!inTauri) return;
  return call("unblock_user", { userId });
}

export async function reportUser(
  userId: string,
  category: string,
  additionalInfo: string,
): Promise<void> {
  if (!inTauri) return;
  return call("report_user", { userId, category, additionalInfo });
}

export async function uploadAttachment(
  target: Target,
  path: string,
): Promise<Attachment[]> {
  if (!inTauri) return [];
  return call("upload_attachment", { target, path });
}

export async function setStatus(status: string): Promise<void> {
  if (!inTauri) return;
  return call("set_status", { status });
}

export async function notifyNative(title: string, body: string): Promise<void> {
  if (!inTauri) return;
  await call("notify_native", { title, body }).catch(() => {});
}

export async function pickFile(): Promise<string | null> {
  if (!inTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({ multiple: false, directory: false });
  return typeof selected === "string" ? selected : null;
}

export async function pickImage(): Promise<string | null> {
  if (!inTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: "Images",
        extensions: ["png", "jpg", "jpeg", "gif", "webp"],
      },
    ],
  });
  return typeof selected === "string" ? selected : null;
}

export async function pickFiles(): Promise<string[]> {
  if (!inTauri) return [];
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({ multiple: true, directory: false });
  if (Array.isArray(selected)) return selected;
  return typeof selected === "string" ? [selected] : [];
}

export async function listenGateway(
  handler: (event: unknown) => void,
): Promise<() => void> {
  if (!inTauri) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  const unlisten = await listen("flackle://gateway", (event) => handler(event.payload));
  return unlisten;
}

export async function openExternal(url: string): Promise<void> {
  if (!inTauri) {
    window.open(url, "_blank", "noopener");
    return;
  }
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(url);
}

export function activeToTarget(active: Active): Target {
  if (active.kind === "guild") {
    return { server_id: active.serverId, channel_id: active.channelId };
  }
  return { conversation_id: active.conversationId };
}

function wait(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
