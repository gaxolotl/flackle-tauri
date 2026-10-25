import { emojiCategories, emojiPath, isBundledEmoji } from "./emoji";
import { getTimeFormat, type TimeFormat } from "./preferences";
import type { Channel, Member, Role, User } from "./types";

export interface RenderContext {
  roles?: Role[];
  members?: Member[];
  channels?: Channel[];
  users?: User[];
  mentionAvatars?: boolean;
}

function safeColor(value: string | null | undefined): string | null {
  if (!value) return null;
  return /^#[0-9a-f]{3,8}$/i.test(value) ? value : null;
}

function findUser(id: string, context?: RenderContext): User | undefined {
  const member = context?.members?.find(
    (entry) => entry.user.id === id || entry.user_id === id,
  );
  return member?.user ?? context?.users?.find((user) => user.id === id);
}

function roleChip(id: string, context?: RenderContext): string {
  const role = context?.roles?.find((entry) => entry.id === id);
  const name = escapeHtml(role?.name ?? "role");
  const color = safeColor(role?.color);
  const icon = role?.icon_url
    ? `<img class="mention-role-icon" src="${escapeHtml(role.icon_url)}" alt="" />`
    : role?.icon_emoji && isBundledEmoji(role.icon_emoji)
      ? `<img class="mention-role-icon" src="${emojiPath(role.icon_emoji)}" alt="" />`
      : "";
  return `<span class="mention role-mention" data-role-id="${escapeHtml(id)}"${
    color ? ` style="--role:${color}"` : ""
  }>${icon}@${name}</span>`;
}

function userChip(id: string, context?: RenderContext): string {
  const user = findUser(id, context);
  const name = escapeHtml(user?.display_name || user?.username || "user");
  const avatar =
    context?.mentionAvatars && user?.avatar_url
      ? `<img class="mention-avatar" src="${escapeHtml(user.avatar_url)}" alt="" draggable="false" />`
      : "";
  return `<span class="mention user-mention" data-user-id="${escapeHtml(id)}">${avatar}@${name}</span>`;
}

function channelChip(id: string, context?: RenderContext): string {
  const channel = context?.channels?.find((entry) => entry.id === id);
  const name = escapeHtml(channel?.name ?? "channel");
  return `<span class="mention channel-mention" data-channel-id="${escapeHtml(id)}">#${name}</span>`;
}

function messageLink(
  url: string,
  serverId: string,
  channelId: string,
  messageId: string,
  context?: RenderContext,
): string {
  const channel = context?.channels?.find((entry) => entry.id === channelId);
  const label = channel ? `#${escapeHtml(channel.name)}` : "message";
  return `<a class="message-link" href="${escapeHtml(url)}" rel="noreferrer" data-message-link="true" data-server-id="${escapeHtml(serverId)}" data-channel-id="${escapeHtml(channelId)}" data-message-id="${escapeHtml(messageId)}"><span class="message-link-channel">${label}</span><span class="message-link-action">Jump to message</span></a>`;
}


export function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

const emojiCharacters = [
  ...new Set(
    emojiCategories.flatMap((category) =>
      category.entries.flatMap((entry) => {
        const char = entry.char;
        const normalized = char.replace(/[\uFE0E\uFE0F]/g, "");
        const variants = [char, normalized];
        if (!/[\uFE0E\uFE0F]/.test(char) && [...char].length === 1) {
          variants.push(`${char}\uFE0F`);
        }
        return variants;
      }),
    ),
  ),
]
  .filter(Boolean)
  .sort((a, b) => b.length - a.length);
const emojiPattern = new RegExp(
  `(${emojiCharacters.map((char) => char.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|")})`,
  "g",
);

function replaceEmoji(segment: string): string {
  return segment.replace(
    emojiPattern,
    (char) =>
      `<img class="message-emoji" src="${emojiPath(char)}" alt="${char}" draggable="false" />`,
  );
}

export function renderContent(content: string, context?: RenderContext): string {
  const tokens: string[] = [];
  const stash = (html: string) => `\uE000${tokens.push(html) - 1}\uE001`;

  let source = content.replace(/<@&(\d+)>/g, (_match, id: string) =>
    stash(roleChip(id, context)),
  );
  source = source.replace(/<@!?(\d+)>/g, (_match, id: string) =>
    stash(userChip(id, context)),
  );
  source = source.replace(/<#(\d+)>/g, (_match, id: string) =>
    stash(channelChip(id, context)),
  );
  source = source.replace(
    /https?:\/\/(?:www\.)?flackle\.com\/app\/channels\/(\d+)\/(\d+)#message-(\d+)/g,
    (match, serverId: string, channelId: string, messageId: string) =>
      stash(messageLink(match, serverId, channelId, messageId, context)),
  );

  let html = escapeHtml(source);
  html = html.replace(/```([\s\S]*?)```/g, (_match, code) => `<pre><code>${code.trim()}</code></pre>`);
  html = html.replace(/`([^`\n]+)`/g, "<code>$1</code>");
  html = html.replace(/\*\*([^*\n]+)\*\*/g, "<strong>$1</strong>");
  html = html.replace(/__([^_\n]+)__/g, "<u>$1</u>");
  html = html.replace(/~~([^~\n]+)~~/g, "<s>$1</s>");
  html = html.replace(/(^|[^*])\*([^*\n]+)\*(?!\*)/g, "$1<em>$2</em>");
  html = html.replace(/(^|[^_\w])_([^_\n]+)_(?!_)/g, "$1<em>$2</em>");
  html = html.replace(
    /(https?:\/\/[^\s<]+)/g,
    '<a href="$1" data-external="true" rel="noreferrer">$1</a>',
  );
  html = html.replace(/(^|\s)@([a-zA-Z0-9_.-]{2,32})/g, '$1<span class="mention">@$2</span>');
  html = html.replace(/\uE000(\d+)\uE001/g, (_match, index: string) => tokens[Number(index)] ?? "");
  html = html.replace(/(<[^>]*>)|([^<]+)/g, (_match, tag: string | undefined, text: string | undefined) =>
    tag ? tag : replaceEmoji(text ?? ""),
  );
  html = html.replace(/\n/g, "<br />");
  return html;
}

export function formatTime(iso: string, mode: TimeFormat = getTimeFormat()): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  return date.toLocaleTimeString([], {
    hour: "numeric",
    minute: "2-digit",
    hour12: mode === "12h",
  });
}

export function formatDay(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  const today = new Date();
  const yesterday = new Date();
  yesterday.setDate(today.getDate() - 1);
  if (date.toDateString() === today.toDateString()) return "Today";
  if (date.toDateString() === yesterday.toDateString()) return "Yesterday";
  return date.toLocaleDateString([], { month: "long", day: "numeric", year: "numeric" });
}

export function formatSize(bytes?: number | null): string {
  if (!bytes) return "";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function sameDay(a: string, b: string): boolean {
  const left = new Date(a);
  const right = new Date(b);
  return left.toDateString() === right.toDateString();
}

export function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? "")
    .join("");
}

export function avatarColor(name: string): string {
  const palette = ["#ff6b5e", "#7c5cff", "#2ec4a6", "#f2a33c", "#4aa8ff", "#e05ea8", "#9ad14b"];
  return palette[(name.charCodeAt(0) || 0) % palette.length];
}
