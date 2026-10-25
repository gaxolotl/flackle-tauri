import { get, writable } from "svelte/store";
import * as api from "./api";
import { ALL_PERMISSIONS } from "./permissions";
import { prefs } from "./preferences";
import type {
  Active,
  ActiveChannel,
  ActiveConversation,
  Attachment,
  Bootstrap,
  ChannelBundle,
  Conversation,
  Embed,
  FriendRequests,
  GatewayEvent,
  Member,
  MemberListSection,
  Message,
  Role,
  SelfUser,
  Server,
  ServerSettingsBundle,
  ServerUserSettings,
  UnreadChannel,
  UnreadSummary,
  User,
} from "./types";
import { activeKey, targetOf } from "./types";

export const me = writable<SelfUser | null>(null);
export const servers = writable<Server[]>([]);
export const conversations = writable<Conversation[]>([]);
export const friends = writable<User[]>([]);
export const blockedUsers = writable<User[]>([]);
export const friendRequests = writable<FriendRequests>({ incoming: [], outgoing: [] });
export const viewMode = writable<"server" | "dm">("server");
export const active = writable<Active | null>(null);
export const channels = writable<ChannelBundle | null>(null);
export const messages = writable<Message[]>([]);
export const members = writable<Member[]>([]);
export const memberSections = writable<MemberListSection[]>([]);
export const memberTotal = writable(0);
export const serverRoles = writable<Role[]>([]);
export const serverPermissions = writable<string | null>(null);
export const activePermissions = writable<string | null>(null);
export const presenceStatuses = writable<Record<string, string>>({});
export const serverSettings = writable<Record<string, ServerUserSettings>>({});
export const pendingAttachments = writable<Attachment[]>([]);
export const uploadingAttachments = writable(false);
export const unread = writable<UnreadChannel[]>([]);
export const unreadCounts = writable<Record<string, number>>({});
export const mentionCounts = writable<Record<string, number>>({});
export const connection = writable<"connecting" | "online" | "offline">(
  api.inTauri ? "connecting" : "online",
);
export const typing = writable<Record<string, { name: string; expires: number }>>({});
export const newMessagesMarker = writable<{ key: string; id: string } | null>(null);
active.subscribe((value) => {
  if (typeof localStorage === "undefined" || !value) return;
  const payload =
    value.kind === "guild"
      ? { serverId: value.serverId, channelId: value.channelId }
      : { conversationId: value.conversationId };
  try {
    localStorage.setItem("flackle-last-channel", JSON.stringify(payload));
  } catch {
    // Remembering the last channel is best-effort.
  }
});
export const hasMore = writable(false);
export const loadingMessages = writable(false);
export const showMembers = writable(true);
export const toast = writable<string | null>(null);
export const modal = writable<"settings" | "server-settings" | "server-info" | "add-server" | "friends" | null>(null);

const channelCache = new Map<string, ChannelBundle>();
const memberCache = new Map<string, Member[]>();
const pingedDms = new Set<string>();
let toastTimer: ReturnType<typeof setTimeout> | undefined;
let lastShareTyping: boolean | null = null;

prefs.subscribe((value) => {
  if (lastShareTyping === null) {
    lastShareTyping = value.shareTyping;
    return;
  }
  if (lastShareTyping === value.shareTyping) return;
  lastShareTyping = value.shareTyping;
  if (!value.shareTyping) void stopTypingInChannel();
  void syncTypingIndicator();
});

export function notify(message: string) {
  toast.set(message);
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toast.set(null), 3600);
}

export async function init(): Promise<boolean> {
  try {
    const user = await api.restore();
    if (!user) return false;
    me.set(user);
    await loadWorkspace();
    return true;
  } catch (error) {
    notify(error instanceof Error ? error.message : String(error));
    return false;
  }
}

export async function loadWorkspace() {
  const data = await api.bootstrap();
  applyBootstrap(data);
  await Promise.all([loadServerSettings(), loadSocial(), syncPresence(), syncTypingIndicator()]);
  if (get(prefs).keepCurrentChannel) await restoreLastChannel();
}

async function syncPresence() {
  try {
    const status = await api.fetchStatus();
    const user = get(me);
    if (user) me.set({ ...user, status });
  } catch {
    // The profile keeps the status returned by bootstrap.
  }
}

export async function syncTypingIndicator() {
  if (!api.inTauri) return;
  const user = get(me);
  if (!get(prefs).shareTyping) {
    await api.updateTypingIndicator(false).catch(() => {});
    return;
  }
  const text =
    user?.typing_indicator_text?.trim() &&
    /^(is|are|was)\s+\S/i.test(user.typing_indicator_text.trim())
      ? user.typing_indicator_text.trim()
      : "is typing";
  const style = user?.typing_indicator_style === "emojis" ? "emojis" : "dots";
  await api
    .updateTypingIndicator(true, text, style)
    .then(() => {
      const current = get(me);
      if (current) {
        me.set({
          ...current,
          typing_indicator_enabled: true,
          typing_indicator_text: text,
          typing_indicator_style: style,
        });
      }
    })
    .catch(() => {});
}

function lastChannelKey() {
  try {
    return JSON.parse(localStorage.getItem("flackle-last-channel") ?? "null") as
      | { serverId: string; channelId: string }
      | { conversationId: string }
      | null;
  } catch {
    return null;
  }
}

async function restoreLastChannel() {
  if (get(active)) return;
  const stored = lastChannelKey();
  if (!stored) return;
  try {
    if ("conversationId" in stored) {
      const conversation = get(conversations).find((entry) => entry.id === stored.conversationId);
      if (conversation) await openConversation(conversation);
      return;
    }
    const server = get(servers).find((entry) => entry.id === stored.serverId);
    if (!server) return;
    const bundle = await ensureChannels(server.id);
    if (!bundle.channels.some((channel) => channel.id === stored.channelId && channel.type !== "voice")) {
      return;
    }
    await openGuild(server, stored.channelId);
  } catch {
    // A removed server or channel simply falls back to the empty view.
  }
}

export async function loadSocial() {
  try {
    const [friendList, blockedList, requests] = await Promise.all([
      api.fetchFriends(),
      api.fetchBlocked(),
      api.fetchFriendRequests(),
    ]);
    friends.set(friendList);
    blockedUsers.set(blockedList);
    friendRequests.set({
      incoming: requests.incoming ?? [],
      outgoing: requests.outgoing ?? [],
    });
  } catch {
    // Social data is optional; keep the last known state.
  }
}

export async function sendFriendRequest(username: string) {
  await api.addFriend(username);
  await loadSocial();
}

export async function acceptFriendRequest(requestId: string) {
  await api.acceptFriendRequest(requestId);
  await loadSocial();
}

export async function rejectFriendRequest(requestId: string) {
  await api.rejectFriendRequest(requestId);
  await loadSocial();
}

export async function cancelFriendRequest(requestId: string) {
  await api.cancelFriendRequest(requestId);
  await loadSocial();
}

export async function removeFriend(userId: string) {
  await api.removeFriend(userId);
  await loadSocial();
}

export async function blockUser(userId: string) {
  await api.blockUser(userId);
  await loadSocial();
}

export async function unblockUser(userId: string) {
  await api.unblockUser(userId);
  await loadSocial();
}

export async function messageUser(userId: string) {
  const conversation = await api.createDM(userId);
  conversations.update((list) => {
    const existing = list.find((entry) => entry.id === conversation.id);
    if (!existing) return [conversation, ...list];
    return list.map((entry) => (entry.id === conversation.id ? { ...entry, ...conversation } : entry));
  });
  await openConversation(conversation);
  return conversation;
}

export async function loadServerSettings() {
  try {
    const bundle: ServerSettingsBundle = await api.fetchServerSettings();
    const map: Record<string, ServerUserSettings> = {};
    for (const setting of bundle.server_settings) {
      map[setting.server_id] = setting;
    }
    serverSettings.set(map);
  } catch {
    // Notifications fall back to the server default.
  }
}

export async function setServerNotificationLevel(serverId: string, level: string) {
  await api.updateServerUserSettings(serverId, { notification_level: level });
  serverSettings.update((map) => ({
    ...map,
    [serverId]: {
      ...(map[serverId] ?? { server_id: serverId }),
      server_id: serverId,
      notification_level: level,
    },
  }));
}

export function notificationLevelFor(serverId: string): string {
  const setting = get(serverSettings)[serverId];
  if (setting?.notification_level) return setting.notification_level;
  const server = get(servers).find((entry) => entry.id === serverId);
  return server?.default_notification_level || "mentions";
}

export function applyBootstrap(data: Bootstrap) {
  me.set(data.me);
  servers.set(data.servers);
  conversations.set(data.conversations);
  applyUnreadSummary(data.unread);
  connection.set(api.inTauri ? "online" : "online");
}

export function applyUnreadSummary(summary: UnreadSummary) {
  unread.set(summary.channels);
  const unreadMap: Record<string, number> = {};
  const mentionMap: Record<string, number> = {};
  for (const entry of summary.channels) {
    const key = `${entry.server_id}:${entry.channel_id}`;
    unreadMap[key] = Math.max(
      0,
      entry.unread_count - (entry.blocked_unread_count ?? 0),
    );
    mentionMap[key] = Math.max(
      0,
      entry.mention_count - (entry.blocked_mention_count ?? 0),
    );
  }
  unreadCounts.set(unreadMap);
  mentionCounts.set(mentionMap);
}

export function clearUnread(serverId: string, channelId: string) {
  const key = `${serverId}:${channelId}`;
  unreadCounts.update((map) => {
    const next = { ...map };
    delete next[key];
    return next;
  });
  mentionCounts.update((map) => {
    const next = { ...map };
    delete next[key];
    return next;
  });
}

export async function ensureChannels(serverId: string): Promise<ChannelBundle> {
  const cached = channelCache.get(serverId);
  if (cached) return cached;
  const bundle = await api.fetchChannels(serverId);
  channelCache.set(serverId, bundle);
  return bundle;
}

export async function openGuild(server: Server, channelId: string) {
  const current = get(active);
  if (
    current?.kind === "guild" &&
    current.serverId === server.id &&
    current.channelId === channelId
  ) {
    return;
  }
  const bundle = await ensureChannels(server.id);
  const channel = bundle.channels.find((item) => item.id === channelId);
  if (!channel) return;

  const next: ActiveChannel = {
    kind: "guild",
    serverId: server.id,
    serverName: server.name,
    channelId,
    title: channel.name,
    topic: channel.topic,
    icon: channel.icon,
    icon_color: channel.icon_color,
  };
  active.set(next);
  viewMode.set("server");
  channels.set(bundle);
  messages.set([]);
  typing.set({});
  newMessagesMarker.set(null);
  hasMore.set(false);
  serverRoles.set([]);
  serverPermissions.set(null);
  activePermissions.set(bundle.permissions?.[channelId] ?? null);
  const unreadBefore = get(unreadCounts)[`${server.id}:${channelId}`] ?? 0;
  clearUnread(server.id, channelId);
  await Promise.all([
    loadMessagesFor(next, unreadBefore),
    loadMemberSnapshot(server.id, channelId),
    loadRoles(server.id),
    loadServerPermissions(server.id),
    loadServerDetails(server.id),
    loadChannelPermissions(server.id, channelId),
  ]);
}

export async function openConversation(conversation: Conversation) {
  const next: ActiveConversation = {
    kind: "dm",
    conversationId: conversation.id,
    title: conversationTitle(conversation),
    subtitle: conversation.type === "group" ? "Group conversation" : "Direct message",
  };
  active.set(next);
  viewMode.set("dm");
  channels.set(null);
  messages.set([]);
  typing.set({});
  newMessagesMarker.set(null);
  hasMore.set(false);
  serverRoles.set([]);
  serverPermissions.set(null);
  activePermissions.set(ALL_PERMISSIONS.toString());
  const unreadBefore = conversation.unread_count ?? 0;
  conversations.update((list) =>
    list.map((entry) => (entry.id === conversation.id ? { ...entry, unread_count: 0 } : entry)),
  );
  pingedDms.delete(conversation.id);
  await loadMessagesFor(next, unreadBefore);
}

function conversationTitle(conversation: Conversation): string {
  if (conversation.name) return conversation.name;
  const mine = get(me);
  const others = (conversation.members ?? [])
    .filter((member) => member.user.id !== mine?.id)
    .map((member) => member.user.display_name || member.user.username);
  return others.join(", ") || "Direct message";
}

export function conversationName(conversation: Conversation): string {
  return conversationTitle(conversation);
}

export async function loadMembers(serverId: string) {
  let cached = memberCache.get(serverId);
  if (!cached) {
    try {
      cached = await api.fetchMembers(serverId);
      memberCache.set(serverId, cached);
    } catch {
      cached = [];
    }
  }
  const presence = get(presenceStatuses);
  members.set(
    cached.map((member) => ({
      ...member,
      user: {
        ...member.user,
        status:
          presence[member.user.id] ?? member.presence ?? member.user.status ?? "offline",
      },
    })),
  );
  memberTotal.set(cached.length);
}

export async function loadMemberSnapshot(serverId: string, channelId: string) {
  try {
    const snapshot = await api.fetchMemberSnapshot(serverId, channelId);
    const presence = get(presenceStatuses);
    members.set(
      snapshot.members.map((member) => ({
        ...member,
        user: {
          ...member.user,
          status:
            presence[member.user.id] ?? member.presence ?? member.user.status ?? "offline",
        },
      })),
    );
    memberSections.set(snapshot.sections ?? []);
    memberTotal.set(snapshot.total || snapshot.members.length);
    if (snapshot.roles?.length) serverRoles.set(snapshot.roles);
    memberCache.set(
      serverId,
      snapshot.members.map((member) => ({
        ...member,
        user: {
          ...member.user,
          status: member.presence ?? member.user.status ?? "offline",
        },
      })),
    );
  } catch {
    memberSections.set([]);
    await loadMembers(serverId);
  }
}

export function applyPresenceMap(presences: Record<string, string>) {
  presenceStatuses.set(presences);
  members.update((list) =>
    list.map((member) => ({
      ...member,
      user: {
        ...member.user,
        status: presences[member.user.id] ?? member.user.status ?? "offline",
      },
    })),
  );
}

export function applyPresence(userId: string, status: string) {
  presenceStatuses.update((map) => ({ ...map, [userId]: status }));
  members.update((list) =>
    list.map((member) =>
      member.user.id === userId ? { ...member, user: { ...member.user, status } } : member,
    ),
  );
}

export async function loadRoles(serverId: string) {
  try {
    const response = await api.fetchRoles(serverId);
    serverRoles.set(response.roles);
  } catch {
    serverRoles.set([]);
  }
}

export function applyRoles(roles: Role[]) {
  serverRoles.set([...roles].sort((a, b) => (a.position ?? 0) - (b.position ?? 0)));
}

export async function loadServerPermissions(serverId: string) {
  try {
    serverPermissions.set(await api.fetchServerPermissions(serverId));
  } catch {
    serverPermissions.set(null);
  }
}

export async function loadServerDetails(serverId: string) {
  try {
    const server = await api.fetchServer(serverId);
    applyServerUpdate(server);
  } catch {
    // Keep the list copy.
  }
}

export async function loadChannelPermissions(serverId: string, channelId: string) {
  try {
    activePermissions.set(await api.fetchChannelPermissions(serverId, channelId));
  } catch {
    // Keep the permissions from the channel bundle when the direct lookup fails.
  }
}

export function applyServerUpdate(server: Server) {
  servers.update((list) => list.map((item) => (item.id === server.id ? { ...item, ...server } : item)));
  active.update((current) =>
    current?.kind === "guild" && current.serverId === server.id
      ? { ...current, serverName: server.name }
      : current,
  );
}

export async function reloadChannels(server: Server): Promise<ChannelBundle> {
  channelCache.delete(server.id);
  const bundle = await api.fetchChannels(server.id);
  channelCache.set(server.id, bundle);
  const current = get(active);
  if (current?.kind === "guild" && current.serverId === server.id) {
    const channel = bundle.channels.find((item) => item.id === current.channelId);
    if (channel) {
      active.set({ ...current, title: channel.name, topic: channel.topic });
      channels.set(bundle);
    } else {
      const first = bundle.channels
        .filter((item) => item.type === "text")
        .sort((a, b) => (a.position ?? 0) - (b.position ?? 0))[0];
      if (first) await openGuild(server, first.id);
      else {
        active.set(null);
        channels.set(bundle);
      }
    }
  }
  return bundle;
}

export async function moveChannelToCategory(
  server: Server,
  channelId: string,
  categoryId: string | null,
  beforeChannelId?: string | null,
) {
  const bundle = get(channels) ?? (await ensureChannels(server.id));
  const categories = [...bundle.categories].sort(
    (a, b) => (a.position ?? 0) - (b.position ?? 0),
  );
  const keyFor = (id?: string | null) => id ?? "__loose";
  const groups = new Map<string, ChannelBundle["channels"]>();
  const keys: string[] = ["__loose", ...categories.map((category) => category.id)];
  for (const key of keys) groups.set(key, []);
  for (const channel of bundle.channels) {
    const key = keyFor(channel.category_id);
    if (!groups.has(key)) {
      groups.set(key, []);
      keys.push(key);
    }
    groups.get(key)!.push({ ...channel });
  }
  for (const list of groups.values()) {
    list.sort((a, b) => (a.position ?? 0) - (b.position ?? 0));
  }

  const dragged = bundle.channels.find((channel) => channel.id === channelId);
  if (!dragged) return;

  for (const [key, list] of groups) {
    groups.set(
      key,
      list.filter((channel) => channel.id !== channelId),
    );
  }

  const targetKey = keyFor(categoryId);
  if (!groups.has(targetKey)) {
    groups.set(targetKey, []);
    keys.push(targetKey);
  }
  const target = groups.get(targetKey)!;
  const moved = { ...dragged, category_id: categoryId };
  const beforeIndex = beforeChannelId
    ? target.findIndex((channel) => channel.id === beforeChannelId)
    : -1;
  if (beforeIndex >= 0) target.splice(beforeIndex, 0, moved);
  else target.push(moved);

  // Flackle stores channel position globally across the server, not per
  // category. Rebuild the full order so a category move also produces valid
  // positions for every other channel.
  const positions: Array<{ id: string; position: number; category_id: string | null }> = [];
  let position = 0;
  for (const key of keys) {
    for (const channel of groups.get(key) ?? []) {
      positions.push({
        id: channel.id,
        position: position++,
        category_id: key === "__loose" ? null : key,
      });
    }
  }

  await api.updateChannelPositions(server.id, positions);
  await reloadChannels(server);
}

export async function setMemberRoles(userId: string, add: string[], remove: string[]) {
  const current = get(active);
  if (current?.kind !== "guild") return null;
  const updated = await api.setMemberRoles(current.serverId, userId, add, remove);
  members.update((list) =>
    list.map((member) =>
      member.user.id === userId
        ? {
            ...member,
            ...updated,
            user: updated.user?.id
              ? { ...member.user, ...updated.user }
              : member.user,
          }
        : member,
    ),
  );
  return updated;
}

export function clearMemberCache(serverId: string) {
  memberCache.delete(serverId);
}

function renderable(message: Message): boolean {
  if (message.is_deleted) return false;
  if (message.type === "system") return true;
  return Boolean(
    message.content ||
      message.attachments?.length ||
      message.embeds?.length ||
      message.reactions?.length,
  );
}

export async function loadMessagesFor(target: Active, unreadCount = 0) {
  loadingMessages.set(true);
  newMessagesMarker.set(null);
  try {
    const page = await api.fetchMessages(targetOf(target), null, 50);
    const visible = page.messages.filter(renderable);
    messages.set(visible);
    hasMore.set(page.has_more_before || page.has_more);
    if (unreadCount > 0 && visible.length > 0) {
      const index = Math.max(0, visible.length - Math.min(unreadCount, visible.length));
      newMessagesMarker.set({ key: activeKey(target), id: visible[index].id });
    }
    const last = visible[visible.length - 1];
    if (last) void api.markRead(targetOf(target), last.id).catch(() => {});
  } catch (error) {
    notify(error instanceof Error ? error.message : String(error));
  } finally {
    loadingMessages.set(false);
  }
}

export async function loadOlder() {
  const current = get(active);
  const list = get(messages);
  if (!current || list.length === 0 || !get(hasMore)) return;
  loadingMessages.set(true);
  try {
    const page = await api.fetchMessages(targetOf(current), list[0].id, 50);
    messages.set([...page.messages.filter(renderable), ...list]);
    hasMore.set(page.has_more_before || page.has_more);
  } catch (error) {
    notify(error instanceof Error ? error.message : String(error));
  } finally {
    loadingMessages.set(false);
  }
}

export async function loadMessagesAround(messageId: string) {
  const current = get(active);
  if (!current) return false;
  loadingMessages.set(true);
  try {
    const page = await api.fetchMessagesAround(targetOf(current), messageId, 50);
    messages.set(page.messages.filter(renderable));
    hasMore.set(page.has_more_before || page.has_more);
    return page.messages.some((message) => message.id === messageId);
  } catch (error) {
    notify(error instanceof Error ? error.message : String(error));
    return false;
  } finally {
    loadingMessages.set(false);
  }
}

let pendingCounter = 0;

export async function attachFiles(paths: string[]) {
  const current = get(active);
  if (!current || paths.length === 0) return;
  if (!api.inTauri) {
    notify("Attachments are available in the desktop app.");
    return;
  }
  uploadingAttachments.set(true);
  try {
    const uploaded: Attachment[] = [];
    for (const path of paths) {
      const files = await api.uploadAttachment(targetOf(current), path);
      uploaded.push(...files);
    }
    pendingAttachments.update((list) => [...list, ...uploaded]);
  } catch (error) {
    notify(error instanceof Error ? error.message : String(error));
  } finally {
    uploadingAttachments.set(false);
  }
}

export function removePendingAttachment(index: number) {
  pendingAttachments.update((list) => list.filter((_item, position) => position !== index));
}

export function clearPendingAttachments() {
  pendingAttachments.set([]);
}

export async function sendMessage(
  content: string,
  replyTo?: string | null,
  attachments: Attachment[] = [],
) {
  const current = get(active);
  if (!current) return;
  const trimmed = content.trim();
  if (!trimmed && attachments.length === 0) return;
  const user = get(me);
  const temporary: Message = {
    id: `pending-${++pendingCounter}`,
    content: trimmed,
    type: "text",
    is_deleted: false,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    sender: user,
    pending: true,
    attachments,
    reply_to: replyTo
      ? {
          id: replyTo,
          content: "",
          sender_id: "",
        }
      : null,
  };
  messages.update((list) => [...list, temporary]);
  try {
    const message = await api.sendMessage(
      targetOf(current),
      trimmed,
      attachments,
      replyTo ?? null,
    );
    messages.update((list) =>
      list.map((item) =>
        item.id === temporary.id ? { ...message, pending: false } : item,
      ),
    );
    if (get(prefs).linkPreviews && trimmed) {
      void attachLinkPreviews(current, message, trimmed);
    }
  } catch (error) {
    messages.update((list) =>
      list.map((item) =>
        item.id === temporary.id ? { ...item, pending: false, failed: true } : item,
      ),
    );
    notify(error instanceof Error ? error.message : String(error));
  }
}

function embedsFrom(value: unknown): Embed[] | null {
  if (!value) return null;
  if (Array.isArray(value)) return value as Embed[];
  if (typeof value !== "object") return null;
  const record = value as { embeds?: unknown; message?: { embeds?: unknown } };
  if (Array.isArray(record.embeds)) return record.embeds as Embed[];
  if (Array.isArray(record.message?.embeds)) return record.message.embeds as Embed[];
  return null;
}

async function attachLinkPreviews(target: Active, message: Message, content: string) {
  if (message.embeds && message.embeds.length > 0) return;
  const urls = [...new Set(content.match(/https?:\/\/[^\s<>()]+/g) ?? [])]
    .filter(
      (url) =>
        !/\.(png|jpe?g|gif|webp|avif|svg|mp4|webm|mov|mp3|wav|ogg|pdf)(\?|$)/i.test(url),
    )
    .slice(0, 5);
  if (urls.length === 0) return;
  const previews = (
    await Promise.all(
      urls.map((url) => api.createLinkPreview(url).catch(() => null)),
    )
  ).filter((entry): entry is NonNullable<typeof entry> => !!entry);
  const tokens = previews
    .map((entry) => entry.token)
    .filter((token): token is string => typeof token === "string" && token.length > 0);
  if (tokens.length === 0) return;
  try {
    const result = await api.updateMessageEmbeds(targetOf(target), message.id, tokens);
    const embeds =
      embedsFrom(result) ??
      (previews
        .flatMap((entry) =>
          Array.isArray(entry.preview) ? entry.preview : entry.preview ? [entry.preview] : [],
        ) as Embed[]);
    const current = get(active);
    if (!current || activeKey(current) !== activeKey(target)) return;
    messages.update((list) =>
      list.map((item) =>
        item.id === message.id ? { ...item, embeds } : item,
      ),
    );
  } catch {
    // Link previews are best-effort and never block the sent message.
  }
}

export async function sendFiles(paths: string[]) {
  const current = get(active);
  if (!current || paths.length === 0) return;
  if (!api.inTauri) {
    notify("Attachments are available in the desktop app.");
    return;
  }
  try {
    const uploaded: Attachment[] = [];
    for (const path of paths) {
      const files = await api.uploadAttachment(targetOf(current), path);
      uploaded.push(...files);
    }
    if (uploaded.length > 0) {
      await api.sendMessage(targetOf(current), "", uploaded, null);
      await loadMessagesFor(current);
    }
  } catch (error) {
    notify(error instanceof Error ? error.message : String(error));
  }
}

export async function editMessage(messageId: string, content: string) {
  const current = get(active);
  if (!current) return;
  const trimmed = content.trim();
  if (!trimmed) return;
  try {
    await api.editMessage(targetOf(current), messageId, trimmed);
    messages.update((list) =>
      list.map((item) =>
        item.id === messageId
          ? { ...item, content: trimmed, edited_at: new Date().toISOString() }
          : item,
      ),
    );
  } catch (error) {
    notify(error instanceof Error ? error.message : String(error));
  }
}

export async function removeMessage(messageId: string) {
  const current = get(active);
  if (!current) return;
  try {
    await api.deleteMessage(targetOf(current), messageId);
    messages.update((list) => list.filter((item) => item.id !== messageId));
  } catch (error) {
    notify(error instanceof Error ? error.message : String(error));
  }
}

export async function toggleReaction(messageId: string, emoji: string, mine: boolean) {
  const current = get(active);
  if (!current) return;
  messages.update((list) =>
    list.map((item) => {
      if (item.id !== messageId) return item;
      const reactions = [...(item.reactions ?? [])];
      const index = reactions.findIndex((reaction) => reaction.emoji === emoji);
      if (mine) {
        if (index >= 0) {
          const reaction = reactions[index];
          const count = Math.max(0, reaction.count - 1);
          if (count === 0) reactions.splice(index, 1);
          else reactions[index] = { ...reaction, count, me: false };
        }
      } else if (index >= 0) {
        reactions[index] = {
          ...reactions[index],
          count: reactions[index].count + 1,
          me: true,
        };
      } else {
        reactions.push({ emoji, count: 1, me: true });
      }
      return { ...item, reactions };
    }),
  );
  try {
    if (mine) await api.unreact(targetOf(current), messageId, emoji);
    else await api.react(targetOf(current), messageId, emoji);
  } catch (error) {
    notify(error instanceof Error ? error.message : String(error));
  }
}

export async function setMyStatus(status: string) {
  const user = get(me);
  const previous = user?.status ?? "online";
  if (user) me.set({ ...user, status });
  try {
    await api.setStatus(status);
    const current = get(me);
    if (current) me.set({ ...current, status });
  } catch (error) {
    if (user) me.set({ ...user, status: previous });
    notify(error instanceof Error ? error.message : String(error));
    throw error;
  }
}

export async function typeInChannel() {
  const current = get(active);
  if (!current || !api.inTauri || !get(prefs).shareTyping) return;
  await api.sendTyping(targetOf(current)).catch(() => {});
}

export async function stopTypingInChannel(target?: Active | null) {
  const current = target ?? get(active);
  if (!current || !api.inTauri) return;
  await api.stopTyping(targetOf(current)).catch(() => {});
}

export async function setTypingFor(channelKey: string, userId: string, isTyping: boolean) {
  const entryKey = `${channelKey}:${userId}`;
  if (!isTyping) {
    typing.update((map) => {
      const next = { ...map };
      delete next[entryKey];
      return next;
    });
    return;
  }
  if (userId === get(me)?.id) return;
  const member = get(members).find((entry) => entry.user.id === userId);
  const conversationId = channelKey.startsWith("d:") ? channelKey.slice(2) : null;
  const conversationUser: User | undefined = conversationId
    ? get(conversations)
        .find((entry) => entry.id === conversationId)
        ?.members?.find((entry) => entry.user.id === userId)?.user
    : undefined;
  const friend = get(friends).find((entry) => entry.id === userId);
  const person = member?.user ?? conversationUser ?? friend;
  const name = person?.display_name || person?.username || "Someone";
  typing.update((map) => ({
    ...map,
    [entryKey]: { name, expires: Date.now() + 6000 },
  }));
  setTimeout(() => {
    typing.update((map) => {
      if (!map[entryKey] || map[entryKey].expires > Date.now()) return map;
      const next = { ...map };
      delete next[entryKey];
      return next;
    });
  }, 6200);
}

export function handleGatewayEvent(raw: unknown) {
  const event = raw as GatewayEvent;
  switch (event.type) {
    case "connected":
      connection.set("online");
      break;
    case "disconnected":
      connection.set("offline");
      break;
    case "ready": {
      connection.set("online");
      const ready = event as {
        user?: SelfUser | null;
        presences?: Record<string, string>;
      };
      if (ready.user) me.set(ready.user);
      if (ready.presences) applyPresenceMap(ready.presences);
      void api.fetchUnread().then(applyUnreadSummary).catch(() => {});
      break;
    }
    case "error":
      notify(String((event as { message?: string }).message ?? "Gateway error"));
      break;
    case "message_create": {
      const current = get(active);
      const message = (event as { message: Message }).message;
      const serverId = (event as { server_id?: string }).server_id;
      const channelId = (event as { channel_id?: string }).channel_id;
      const conversationId = (event as { conversation_id?: string }).conversation_id;
      const inActiveGuild =
        current?.kind === "guild" &&
        current.serverId === serverId &&
        current.channelId === channelId;
      const inActiveConversation =
        current?.kind === "dm" &&
        current.conversationId === (conversationId ?? message.conversation_id);

      if (!renderable(message)) {
        messages.update((list) => list.filter((item) => item.id !== message.id));
        break;
      }

      if (inActiveGuild || inActiveConversation) {
        messages.update((list) =>
          list.some((item) => item.id === message.id)
            ? list.map((item) => (item.id === message.id ? message : item))
            : [...list, message],
        );
        void api.markRead(targetOf(current), message.id).catch(() => {});
      } else if (serverId && channelId) {
        const key = `${serverId}:${channelId}`;
        unreadCounts.update((map) => ({
          ...map,
          [key]: (map[key] ?? 0) + 1,
        }));
        const mine = get(me)?.id;
        const mentioned =
          message.sender?.id !== mine &&
          (Boolean(message.mention_everyone) ||
            Boolean(mine && message.mention_ids?.includes(mine)));
        if (mentioned) {
          mentionCounts.update((map) => ({
            ...map,
            [key]: (map[key] ?? 0) + 1,
          }));
        }
      } else if (conversationId ?? message.conversation_id) {
        const id = conversationId ?? message.conversation_id!;
        conversations.update((list) =>
          list.map((conversation) =>
            conversation.id === id
              ? { ...conversation, unread_count: (conversation.unread_count ?? 0) + 1 }
              : conversation,
          ),
        );
      }

      refreshConversationPreview(message);

      if (
        !inActiveGuild &&
        !inActiveConversation &&
        message.sender?.id !== get(me)?.id &&
        typeof document !== "undefined" &&
        (document.hidden || !document.hasFocus())
      ) {
        const notificationPrefs = get(prefs);
        const isDm = Boolean(conversationId ?? message.conversation_id);
        const mine = get(me)?.id;
        const mentioned =
          Boolean(message.mention_everyone) ||
          Boolean(mine && message.mention_ids?.includes(mine));
        const server = serverId
          ? get(servers).find((entry) => entry.id === serverId)
          : undefined;
        const userSetting = serverId ? get(serverSettings)[serverId] : undefined;
        const level =
          userSetting?.notification_level ||
          server?.default_notification_level ||
          "mentions";
        const serverAllows = level === "all" || (level === "mentions" && mentioned);
        const dmId = conversationId ?? message.conversation_id ?? null;
        let dmPingAllowed = true;
        if (isDm && notificationPrefs.onePingPerDm && dmId) {
          dmPingAllowed = !pingedDms.has(dmId);
        }
        const shouldNotify =
          notificationPrefs.notifications &&
          dmPingAllowed &&
          (isDm || (serverAllows && (!notificationPrefs.mentionsOnly || mentioned)));
        if (shouldNotify && isDm && notificationPrefs.onePingPerDm && dmId) {
          pingedDms.add(dmId);
        }
        if (shouldNotify) {
          const sender =
            message.sender?.display_name || message.sender?.username || "New message";
          const title = isDm ? sender : server?.name ?? sender;
          const body =
            message.content.trim().slice(0, 180) ||
            (message.attachments?.length ? "Sent an attachment" : "New message");
          void api.notifyNative(title, body);
        }
      }
      break;
    }
    case "message_update": {
      const update = event as { id: string; content: string; edited_at?: string };
      messages.update((list) =>
        list.map((item) =>
          item.id === update.id
            ? { ...item, content: update.content, edited_at: update.edited_at }
            : item,
        ),
      );
      break;
    }
    case "message_delete": {
      const update = event as { id: string };
      messages.update((list) => list.filter((item) => item.id !== update.id));
      break;
    }
    case "typing":
    case "channel_typing": {
      const update = event as {
        user_id: string;
        is_typing: boolean;
        server_id?: string;
        channel_id?: string;
        conversation_id?: string;
      };
      const channelKey =
        update.server_id && update.channel_id
          ? `g:${update.server_id}:${update.channel_id}`
          : update.conversation_id
            ? `d:${update.conversation_id}`
            : null;
      if (channelKey) void setTypingFor(channelKey, update.user_id, update.is_typing);
      break;
    }
    case "presence": {
      const update = event as { user_id: string; status: string };
      applyPresence(update.user_id, update.status);
      break;
    }
  }
}

function refreshConversationPreview(message: Message) {
  conversations.update((list) =>
    list.map((conversation) =>
      conversation.id === message.conversation_id
        ? { ...conversation, last_message: message, last_message_at: message.created_at }
        : conversation,
    ),
  );
}

export { targetOf, activeKey };
