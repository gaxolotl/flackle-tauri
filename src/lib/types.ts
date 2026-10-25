export type Snowflake = string;

export interface User {
  id: Snowflake;
  username: string;
  display_name?: string | null;
  avatar_url?: string | null;
  banner_url?: string | null;
  status?: string | null;
}

export interface SelfUser extends User {
  email: string;
  is_admin: boolean;
  typing_indicator_enabled?: boolean | null;
  typing_indicator_text?: string | null;
  typing_indicator_style?: string | null;
  typing_indicator_emojis?: string[] | null;
}

export interface Attachment {
  url: string;
  file_name?: string | null;
  file_size?: number | null;
  content_type?: string | null;
  width?: number | null;
  height?: number | null;
  expires_at?: string | null;
  saved?: boolean | null;
}

export interface EmbedField {
  name?: string;
  value?: string;
  inline?: boolean;
}

export interface Embed {
  url?: string;
  title?: string;
  description?: string;
  type?: string;
  color?: string | number;
  site_name?: string;
  siteName?: string;
  provider_name?: string;
  providerName?: string;
  image_url?: string;
  imageUrl?: string;
  thumbnail_url?: string;
  thumbnailUrl?: string;
  video_url?: string;
  videoUrl?: string;
  video_hls_url?: string;
  videoHlsUrl?: string;
  favicon_url?: string;
  faviconUrl?: string;
  fields?: EmbedField[];
  [key: string]: unknown;
}

export interface Reaction {
  emoji: string;
  count: number;
  me?: boolean;
  users?: User[];
}

export interface MessageReply {
  id: Snowflake;
  content: string;
  sender_id: Snowflake;
  sender_username?: string | null;
  sender_display_name?: string | null;
  sender_avatar_url?: string | null;
  deleted?: boolean;
}

export interface Message {
  id: Snowflake;
  conversation_id?: Snowflake | null;
  content: string;
  type: string;
  is_deleted: boolean;
  edited_at?: string | null;
  created_at: string;
  updated_at: string;
  nonce?: string | null;
  sender?: User | null;
  sender_id?: Snowflake | null;
  sender_username?: string | null;
  sender_display_name?: string | null;
  sender_avatar_url?: string | null;
  author?: User | null;
  attachments?: Attachment[];
  embeds?: Embed[];
  reply_to?: MessageReply | null;
  reactions?: Reaction[];
  mention_ids?: string[];
  mention_everyone?: boolean;
  pending?: boolean;
  failed?: boolean;
}

export interface Server {
  id: Snowflake;
  name: string;
  description?: string | null;
  icon_url?: string | null;
  banner_url?: string | null;
  owner_id?: Snowflake | null;
  default_notification_level?: string | null;
  position?: number | null;
  approx_member_count?: number | null;
  approx_presence_count?: number | null;
  flags?: string | null;
  verified?: boolean | null;
  created_at?: string | null;
}

export interface Channel {
  id: Snowflake;
  name: string;
  topic?: string | null;
  type: string;
  position?: number | null;
  category_id?: Snowflake | null;
  slowmode_seconds?: number | null;
  icon?: string | null;
  icon_color?: string | null;
}

export interface Category {
  id: Snowflake;
  name: string;
  position?: number | null;
}

export interface ChannelBundle {
  categories: Category[];
  channels: Channel[];
  permissions: Record<string, string>;
}

export interface ConversationMember {
  user: User;
  role?: string | null;
}

export interface Conversation {
  id: Snowflake;
  type: string;
  name?: string | null;
  icon_url?: string | null;
  is_muted?: boolean;
  last_message_at?: string | null;
  last_message?: Message | null;
  members?: ConversationMember[];
  unread_count?: number;
}

export interface FriendRequest {
  id: Snowflake;
  status: string;
  created_at: string;
  updated_at?: string;
  sender?: User | null;
  receiver?: User | null;
}

export interface FriendRequests {
  incoming: FriendRequest[];
  outgoing: FriendRequest[];
}

export interface Member {
  user: User;
  user_id?: Snowflake | null;
  nickname?: string | null;
  role_ids?: string[];
  roles?: string[];
  presence?: string | null;
  is_owner?: boolean;
  joined_at?: string | null;
}

export interface MemberListSection {
  key: string;
  label: string;
  role_id?: Snowflake | null;
  count: number;
  member_ids: string[];
}

export interface MemberListSnapshot {
  members: Member[];
  total: number;
  sections: MemberListSection[];
  roles: Role[];
}

export interface AcceptInviteResult {
  server?: Server | null;
  landing_channel_id?: Snowflake | null;
}

export interface CreateServerResult {
  server: Server;
  channel: Channel;
}

export interface Invite {
  code: string;
  url?: string | null;
  server_id?: string | null;
  channel_id?: string | null;
  max_uses?: number | null;
  uses?: number;
  expires_at?: string | null;
  created_at?: string | null;
  creator_id?: string | null;
  creator_username?: string | null;
  creator_name?: string | null;
  creator_avatar_url?: string | null;
}

export interface ServerInvite extends Invite {
  max_uses?: number | null;
  uses?: number;
  expires_at?: string | null;
  created_at?: string | null;
  creator_id?: string | null;
  creator_username?: string | null;
  creator_name?: string | null;
  creator_avatar_url?: string | null;
}

export interface ServerBan {
  user_id: string;
  user?: User | null;
  reason?: string | null;
  moderator_id?: string | null;
  created_at?: string | null;
}

export interface AuditChange {
  key?: string;
  old?: unknown;
  new?: unknown;
}

export interface AuditLogEntry {
  id: string;
  action_type: string;
  actor?: User | null;
  actor_id?: string | null;
  target_id?: string | null;
  target_kind?: string | null;
  target_user?: User | null;
  changes?: AuditChange[];
  options?: Record<string, unknown>;
  created_at?: string | null;
}

export interface AuditLogPage {
  entries: AuditLogEntry[];
  has_more?: boolean;
}

export interface LiftLevel {
  level: number;
  lifts_needed: number;
  stream_fps?: Record<string, number>;
  max_upload_bytes: number;
  saved_budget_bytes: number;
  retention_multiplier: number;
}

export interface LiftsInfo {
  server_id: string;
  level: number;
  count: number;
  next_level_at: number;
  stream_fps?: Record<string, number>;
  max_upload_bytes: number;
  saved_budget_bytes: number;
  retention_multiplier: number;
  levels: LiftLevel[];
}

export interface SavedMediaPage {
  items: Array<{
    url: string;
    file_name?: string | null;
    file_size?: number | null;
    content_type?: string | null;
    expires_at?: string | null;
    saved_at?: string | null;
  }>;
  count: number;
  used?: number;
  budget?: number;
  next_cursor?: string | null;
}

export interface RoleBatchUpdate {
  id: string;
  name?: string;
  permissions?: string;
  color?: string | null;
  hoist?: boolean;
  mentionable?: boolean;
  icon_emoji?: string | null;
  clear_icon?: boolean;
  upload_icon?: boolean;
  expected_updated_at?: string | null;
}

export interface RoleBatchCreate {
  id?: string;
  name: string;
  permissions?: string;
  color?: string | null;
  hoist?: boolean;
  mentionable?: boolean;
  icon_emoji?: string | null;
}

export interface RoleBatchPayload {
  updates?: RoleBatchUpdate[];
  creates?: RoleBatchCreate[];
  deletes?: Array<{ id: string }>;
  order?: string[];
}

export interface RoleBatchResponse {
  roles?: Role[];
  created_role_ids?: Record<string, string>;
}

export interface ServerPositionUpdate {
  server_id: string;
  position: number;
  folder_id?: string | null;
  clear_folder?: boolean;
}

export interface ServerUserSettings {
  server_id: string;
  notification_level?: string | null;
  last_viewed_channel_id?: string | null;
  collapsed_categories?: string[];
}

export interface ServerSettingsBundle {
  server_settings: ServerUserSettings[];
}

export interface Role {
  id: Snowflake;
  name: string;
  permissions: string;
  position?: number | null;
  color?: string | null;
  hoist?: boolean;
  mentionable?: boolean;
  is_everyone?: boolean;
  icon_url?: string | null;
  icon_emoji?: string | null;
  member_count?: number | null;
  updated_at?: string | null;
}

export interface RolesResponse {
  roles: Role[];
  can_view_role_permissions?: boolean;
}

export interface ServerUpdate {
  name?: string;
  description?: string;
  default_notification_level?: "all" | "mentions" | "nothing";
}

export interface ChannelUpdate {
  name?: string;
  topic?: string;
  slowmode_seconds?: number;
  category_id?: string | null;
  position?: number;
  icon_color?: string;
  visibility_status?: string;
}

export interface CreateChannel {
  name: string;
  type?: string;
  category_id?: string | null;
  topic?: string | null;
  slowmode_seconds?: number;
}

export interface MessagePage {
  messages: Message[];
  has_more: boolean;
  has_more_before: boolean;
  has_more_after: boolean;
}

export interface UnreadChannel {
  server_id: Snowflake;
  channel_id: Snowflake;
  unread_count: number;
  mention_count: number;
  blocked_unread_count?: number;
  blocked_mention_count?: number;
}

export interface UnreadSummary {
  channels: UnreadChannel[];
}

export interface AuthSession {
  id: string;
  ip_address?: string | null;
  user_agent?: string | null;
  location?: string | null;
  created_at?: string | null;
  last_used_at?: string | null;
  current?: boolean;
}

export interface SecurityEvent {
  event_type: string;
  user_agent?: string | null;
  created_at?: string | null;
  unusual?: boolean;
  unusual_reason?: string | null;
  origin_location?: string | null;
  [key: string]: unknown;
}

export interface TwoFactorStatus {
  enabled: boolean;
  backup_codes_remaining?: number;
  pending?: boolean;
}

export interface TwoFactorSetup {
  secret: string;
  otpauth_url?: string | null;
  qr_code?: string | null;
}

export interface BackupCodes {
  codes: string[];
}

export interface Bootstrap {
  me: SelfUser;
  servers: Server[];
  conversations: Conversation[];
  unread: UnreadSummary;
}

export type Target =
  | { server_id: Snowflake; channel_id: Snowflake }
  | { conversation_id: Snowflake };

export type LoginOutcome =
  | { status: "success"; user: SelfUser }
  | { status: "mfa_required"; mfa_token: string; methods: string[] };

export type GatewayEvent =
  | { type: "ready"; session_id: string; user?: SelfUser | null; presences?: Record<string, string> }
  | { type: "connected" }
  | { type: "disconnected"; code: number; reason: string }
  | {
      type: "message_create";
      message: Message;
      server_id?: string | null;
      channel_id?: string | null;
      conversation_id?: string | null;
    }
  | {
      type: "message_update";
      id: string;
      content: string;
      edited_at?: string | null;
      server_id?: string | null;
      channel_id?: string | null;
      conversation_id?: string | null;
    }
  | {
      type: "message_delete";
      id: string;
      server_id?: string | null;
      channel_id?: string | null;
      conversation_id?: string | null;
    }
  | {
      type: "typing";
      user_id: string;
      is_typing: boolean;
      server_id?: string | null;
      channel_id?: string | null;
      conversation_id?: string | null;
    }
  | {
      type: "presence";
      user_id: string;
      status: string;
    }
  | { type: "error"; message: string }
  | { type: string; [key: string]: unknown };

export interface PaletteItemLike {
  key: string;
  label: string;
  hint: string;
  icon: string;
  run: () => void;
}

export interface ActiveChannel {
  kind: "guild";
  serverId: string;
  serverName: string;
  channelId: string;
  title: string;
  topic?: string | null;
  icon?: string | null;
  icon_color?: string | null;
}

export interface ActiveConversation {
  kind: "dm";
  conversationId: string;
  title: string;
  subtitle?: string;
}

export type Active = ActiveChannel | ActiveConversation;

export function isGuild(active: Active | null): active is ActiveChannel {
  return !!active && active.kind === "guild";
}

export function targetOf(active: Active): Target {
  if (active.kind === "guild") {
    return { server_id: active.serverId, channel_id: active.channelId };
  }
  return { conversation_id: active.conversationId };
}

export function activeKey(active: Active): string {
  return active.kind === "guild"
    ? `g:${active.serverId}:${active.channelId}`
    : `d:${active.conversationId}`;
}
