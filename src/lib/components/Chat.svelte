<script lang="ts">
  import { afterUpdate, onMount, tick } from "svelte";
  import * as api from "../api";
  import { formatDay, formatSize, formatTime, renderContent, sameDay, truncateText } from "../format";
  import { prefs } from "../preferences";
  import { getRecentEmoji, pushRecentEmoji, searchEmoji } from "../emoji";
  import type { Active, Attachment, Channel, Embed, Message, Role, User } from "../types";
  import { activeKey } from "../types";
  import { hasPermission } from "../permissions";
  import { openImage } from "../image-viewer";
  import { openProfile } from "../profile";
  import { showUserMenu } from "../user-menu";
  import { openContextMenu, type ContextMenuItem } from "../context-menu";
  import { confirmAction } from "../dialog";
  import {
    active,
    activePermissions,
    attachFiles,
    channels,
    clearPendingAttachments,
    conversations,
    editMessage,
    hasMore,
    loadOlder,
    loadMessagesAround,
    loadingMessages,
    me,
    members,
    messages,
    newMessagesMarker,
    notify,
    pendingAttachments,
    presenceStatuses,
    removeMessage,
    removePendingAttachment,
    sendMessage,
    serverRoles,
    servers,
    showMembers,
    stopTypingInChannel,
    toggleReaction,
    typeInChannel,
    typing,
    openGuild,
    uploadingAttachments,
  } from "../workspace";
  import Avatar from "./Avatar.svelte";
  import ChannelIcon from "./ChannelIcon.svelte";
  import Emoji from "./Emoji.svelte";
  import EmojiPicker from "./EmojiPicker.svelte";
  import EmojiText from "./EmojiText.svelte";
  import Icon from "./Icon.svelte";
  import StatsChannel from "./StatsChannel.svelte";
  import VideoPlayer from "./VideoPlayer.svelte";
  import { statsElevatedVisible } from "../stats";

  let scroller: HTMLDivElement;
  let composer: HTMLTextAreaElement;
  let text = "";
  let replyTarget: Message | null = null;
  let editingTarget: Message | null = null;
  let atBottom = true;
  let showJump = false;
  let picker: { x: number; y: number; message?: Message } | null = null;
  let lastKey = "";
  let lastCount = 0;
  let lastTypingSent = 0;
  let dragOver = false;
  let highlightId: string | null = null;
  let forceScroll = false;
  let typingStopTimer: ReturnType<typeof setTimeout> | undefined;
  let previousActive: Active | null = null;

  type Suggestion = {
    id: string;
    kind: "user" | "channel" | "emoji" | "role";
    label: string;
    detail: string;
    insert: string;
    user?: User | null;
    channel?: Channel;
    role?: Role;
    char?: string;
  };
  let suggestions: Suggestion[] = [];
  let suggestionIndex = 0;
  let suggestionStart = 0;
  let suggestionEnd = 0;
  let recentEmoji = getRecentEmoji();

  const messageSkeletons = [
    { name: 92, lines: [94, 58] },
    { name: 74, lines: [88] },
    { name: 108, lines: [96, 72] },
    { name: 82, lines: [66] },
    { name: 96, lines: [91, 44] },
    { name: 70, lines: [84] },
  ];

  $: currentKey = $active ? activeKey($active) : "";
  $: if (currentKey !== lastKey) {
    if (previousActive) void stopTypingInChannel(previousActive);
    if (typingStopTimer) clearTimeout(typingStopTimer);
    previousActive = $active;
    lastKey = currentKey;
    replyTarget = null;
    editingTarget = null;
    picker = null;
    text = "";
    atBottom = true;
    showJump = false;
    forceScroll = true;
    lastTypingSent = 0;
    suggestions = [];
    clearPendingAttachments();
    void tick().then(() => {
      resize();
      scrollToBottom(false);
      if (!isStats && canSendMessages) composer?.focus();
    });
  }
  $: typingEntries = $prefs.showTyping
    ? Object.entries($typing)
        .filter(([key]) => key.startsWith(`${currentKey}:`))
        .map(([, entry]) => entry)
    : [];
  $: singleTyping = typingEntries.length === 1 ? typingEntries[0] : null;
  $: singleTypingUser = singleTyping
    ? ($members.find((member) => member.user.id === singleTyping?.userId)?.user ??
      $conversations
        .flatMap((conversation) => conversation.members ?? [])
        .find((member) => member.user.id === singleTyping?.userId)?.user)
    : undefined;
  $: singleTypingIndicator = singleTypingUser ? typingIndicator(singleTypingUser) : null;
  $: quickReactions = recentEmoji.length > 0 ? recentEmoji.slice(0, 3) : ["👍", "🔥", "😂"];
  $: typingLabel =
    typingEntries.length === 0
      ? ""
      : typingEntries.length === 1
        ? singleTypingIndicator
          ? `${typingEntries[0].name} ${singleTypingIndicator.text}`
          : `${typingEntries[0].name} is typing…`
        : typingEntries.length === 2
          ? `${typingEntries[0].name} and ${typingEntries[1].name} are typing…`
          : "Several people are typing…";
  $: channelTitle = $active?.title ?? "Conversation";
  $: isStats =
    $active?.kind === "guild" && $active.channelType === "stats";
  $: activeTopic =
    $active?.kind === "guild" && !isStats
      ? $active.topic || $active.serverName
      : "";
  $: canViewElevatedStats =
    $activePermissions !== null &&
    hasPermission($activePermissions, "ViewElevatedStats");
  $: canSendMessages =
    $active?.kind === "dm" ||
    $activePermissions === null ||
    hasPermission($activePermissions, "SendMessages");
  $: canAttach =
    $active?.kind === "dm" ||
    $activePermissions === null ||
    hasPermission($activePermissions, "AttachFiles");
  $: canReact =
    $active?.kind === "dm" ||
    $activePermissions === null ||
    hasPermission($activePermissions, "AddReactions");
  $: canSend =
    canSendMessages &&
    !$uploadingAttachments &&
    (text.trim().length > 0 || $pendingAttachments.length > 0);
  $: roster = $members;
  $: presence = $presenceStatuses;
  $: roleList = $serverRoles;
  $: channelList = $channels?.channels ?? [];
  $: dmUsers =
    $active?.kind === "dm"
      ? ($conversations.find((conversation) => conversation.id === $active?.conversationId)
          ?.members ?? []
        ).map((member) => member.user)
      : [];
  $: renderContext = {
    roles: roleList,
    members: roster,
    channels: channelList,
    users: dmUsers,
    mentionAvatars: $prefs.mentionAvatars,
  };
  $: suggestionHeading =
    suggestions.length === 0
      ? ""
      : suggestions[0].kind === "emoji"
        ? "Emoji"
        : suggestions[0].kind === "channel"
          ? "Channels"
          : suggestions.some((suggestion) => suggestion.kind === "role") &&
              suggestions.some((suggestion) => suggestion.kind === "user")
            ? "Members & roles"
            : suggestions.some((suggestion) => suggestion.kind === "role")
              ? "Roles"
              : "Members";

  afterUpdate(() => {
    if (forceScroll && !$loadingMessages) {
      forceScroll = false;
      void tick().then(() => scrollToBottom(false));
    }
    if ($messages.length !== lastCount) {
      const grew = $messages.length > lastCount;
      lastCount = $messages.length;
      if (grew && atBottom) {
        void tick().then(() => scrollToBottom(true));
      }
    }
  });

  onMount(() => {
    void tick().then(() => {
      scrollToBottom(false);
      if (!isStats && canSendMessages) composer?.focus();
    });
  });

  function scrollToBottom(smooth: boolean) {
    if (!scroller) return;
    scroller.scrollTo({ top: scroller.scrollHeight, behavior: smooth ? "smooth" : "auto" });
    atBottom = true;
    showJump = false;
  }

  async function onScroll() {
    if (!scroller) return;
    const distance = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight;
    atBottom = distance < 90;
    showJump = !atBottom && $messages.length > 0;
    if (scroller.scrollTop < 60 && $hasMore && !$loadingMessages) {
      const oldHeight = scroller.scrollHeight;
      const oldTop = scroller.scrollTop;
      await loadOlder();
      await tick();
      scroller.scrollTop = oldTop + (scroller.scrollHeight - oldHeight);
    }
  }

  function resize() {
    if (!composer) return;
    composer.style.height = "auto";
    composer.style.height = `${Math.min(composer.scrollHeight, 176)}px`;
  }

  function resizeSoon() {
    void tick().then(resize);
  }

  function clearComposer() {
    text = "";
    replyTarget = null;
    editingTarget = null;
    suggestions = [];
    resizeSoon();
  }

  function onInput() {
    resize();
    updateSuggestions();
    if (!canSendMessages) return;
    if (!$active) return;
    if (typingStopTimer) clearTimeout(typingStopTimer);
    if (!text.trim()) {
      void stopTypingInChannel();
      return;
    }
    const now = Date.now();
    if (now - lastTypingSent > 2000) {
      lastTypingSent = now;
      void typeInChannel();
    }
    typingStopTimer = setTimeout(() => {
      void stopTypingInChannel();
    }, 4500);
  }

  function tokenAtCaret():
    | { trigger: string; query: string; start: number; end: number }
    | null {
    if (!composer) return null;
    const caret = composer.selectionStart ?? text.length;
    const before = text.slice(0, caret);
    const match = /(^|\s)([@#:])([^\s@#:]*)$/.exec(before);
    if (!match) return null;
    const trigger = match[2];
    const query = match[3];
    if (trigger === ":" && query.length === 0) return null;
    return {
      trigger,
      query,
      start: caret - query.length - 1,
      end: caret,
    };
  }

  function updateSuggestions() {
    const token = tokenAtCaret();
    if (!token) {
      suggestions = [];
      return;
    }
    const query = token.query.toLowerCase();
    let next: Suggestion[] = [];
    if (token.trigger === "@") {
      const memberMatches = $members
        .filter((member) => {
          const name = (member.nickname || member.user.display_name || member.user.username).toLowerCase();
          return !query || name.includes(query) || member.user.username.toLowerCase().includes(query);
        })
        .slice(0, 8)
        .map((member) => ({
          id: `user:${member.user.id}`,
          kind: "user" as const,
          label: member.nickname || member.user.display_name || member.user.username,
          detail: `@${member.user.username}`,
          insert: `<@${member.user.id}>`,
          user: member.user,
        }));
      const roleMatches = roleList
        .filter(
          (role) =>
            !role.is_everyone &&
            role.mentionable !== false &&
            (!query || role.name.toLowerCase().includes(query)),
        )
        .slice(0, 4)
        .map((role) => ({
          id: `role:${role.id}`,
          kind: "role" as const,
          label: `@${role.name}`,
          detail: "Notify role members",
          insert: `<@&${role.id}>`,
          role,
        }));
      next = [...memberMatches, ...roleMatches];
    } else if (token.trigger === "#") {
      next = channelList
        .filter((channel) => !query || channel.name.toLowerCase().includes(query))
        .slice(0, 8)
        .map((channel) => ({
          id: `channel:${channel.id}`,
          kind: "channel" as const,
          label: channel.name,
          detail:
            channel.type === "voice"
              ? "Voice channel"
              : channel.type === "stats"
                ? "Stats channel"
                : "Text channel",
          insert: `<#${channel.id}>`,
          channel,
        }));
    } else {
      next = searchEmoji(token.query)
        .slice(0, 8)
        .map((entry) => ({
          id: `emoji:${entry.char}`,
          kind: "emoji" as const,
          label: `:${entry.name}:`,
          detail: entry.name,
          insert: entry.char,
          char: entry.char,
        }));
    }
    suggestions = next;
    suggestionStart = token.start;
    suggestionEnd = token.end;
    suggestionIndex = 0;
  }

  function applySuggestion(suggestion: Suggestion) {
    if (!composer) return;
    const start = suggestionStart;
    const end = suggestionEnd;
    const before = text.slice(0, start);
    const after = text.slice(end);
    text = before + suggestion.insert + after;
    suggestions = [];
    if (suggestion.kind === "emoji" && suggestion.char) {
      recentEmoji = pushRecentEmoji(suggestion.char);
    }
    void tick().then(() => {
      composer?.focus();
      const position = before.length + suggestion.insert.length;
      composer?.setSelectionRange(position, position);
      resize();
    });
  }

  async function submit() {
    if (!canSendMessages) return;
    suggestions = [];
    if (typingStopTimer) clearTimeout(typingStopTimer);
    void stopTypingInChannel();
    if (editingTarget) {
      if (!text.trim()) return;
      const target = editingTarget;
      editingTarget = null;
      const content = text;
      text = "";
      resizeSoon();
      await editMessage(target.id, content);
      composer?.focus();
      return;
    }
    if (!text.trim() && $pendingAttachments.length === 0) return;
    const content = text;
    const reply = replyTarget;
    const attachments = [...$pendingAttachments];
    text = "";
    replyTarget = null;
    clearPendingAttachments();
    resizeSoon();
    await sendMessage(content, reply, attachments);
    composer?.focus();
  }

  function onComposerKey(event: KeyboardEvent) {
    if (suggestions.length > 0) {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        suggestionIndex = (suggestionIndex + 1) % suggestions.length;
        return;
      }
      if (event.key === "ArrowUp") {
        event.preventDefault();
        suggestionIndex = (suggestionIndex - 1 + suggestions.length) % suggestions.length;
        return;
      }
      if (event.key === "Tab" || (event.key === "Enter" && !event.shiftKey && !event.isComposing)) {
        event.preventDefault();
        applySuggestion(suggestions[suggestionIndex]);
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        suggestions = [];
        return;
      }
    }
    const wantsSend =
      $prefs.sendKey === "enter"
        ? event.key === "Enter" && !event.shiftKey && !event.isComposing
        : event.key === "Enter" && (event.metaKey || event.ctrlKey) && !event.isComposing;
    if (wantsSend) {
      event.preventDefault();
      void submit();
    } else if (event.key === "Escape") {
      clearComposer();
    }
  }

  function beginReply(message: Message) {
    editingTarget = null;
    replyTarget = message;
    suggestions = [];
    composer?.focus();
    resizeSoon();
  }

  function beginEdit(message: Message) {
    replyTarget = null;
    editingTarget = message;
    suggestions = [];
    text = message.content;
    void tick().then(() => {
      composer?.focus();
      composer?.setSelectionRange(text.length, text.length);
      resize();
    });
  }

  async function confirmDelete(message: Message) {
    if ($prefs.quickDelete) {
      await removeMessage(message.id);
      return;
    }
    const confirmed = await confirmAction({
      title: "Delete message?",
      body: "This cannot be undone.",
      confirmLabel: "Delete",
      danger: true,
    });
    if (!confirmed) return;
    await removeMessage(message.id);
  }

  function toggle(message: Message, emoji: string) {
    if (!canReact) return;
    recentEmoji = pushRecentEmoji(emoji);
    const mine = message.reactions?.find((reaction) => reaction.emoji === emoji)?.me ?? false;
    void toggleReaction(message.id, emoji, mine);
  }

  function addEmoji(emoji: string) {
    text += emoji;
    recentEmoji = pushRecentEmoji(emoji);
    composer?.focus();
    resizeSoon();
  }

  function openComposerPicker(event: MouseEvent) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    picker = { x: rect.left, y: rect.top };
  }

  function openReactionPicker(event: MouseEvent, message: Message) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    picker = { x: rect.left, y: rect.bottom, message };
  }

  function onEmojiSelect(emoji: string) {
    if (picker?.message) toggle(picker.message, emoji);
    else addEmoji(emoji);
    picker = null;
  }

  async function copyText(value: string, label: string) {
    try {
      await navigator.clipboard.writeText(value);
      notify(`${label} copied.`);
    } catch {
      notify(`Could not copy ${label.toLowerCase()}.`);
    }
  }

  function showMessageMenu(event: MouseEvent, message: Message) {
    const items: ContextMenuItem[] = [
      {
        label: "Add reaction",
        icon: "smile",
        run: () => {
          picker = { x: event.clientX, y: event.clientY, message };
        },
      },
      {
        label: "Reply",
        icon: "reply",
        run: () => beginReply(message),
      },
    ];
    if (isMine(message)) {
      items.push({
        label: "Edit message",
        icon: "pencil",
        run: () => beginEdit(message),
      });
      items.push({
        label: "Delete message",
        icon: "trash",
        danger: true,
        run: () => void confirmDelete(message),
      });
    }
    items.push({ separator: true, label: "" });
    if (message.content) {
      items.push({
        label: "Copy text",
        icon: "copy",
        run: () => void copyText(message.content, "Message"),
      });
    }
    if ($active?.kind === "guild") {
      const link = `https://flackle.com/app/channels/${$active.serverId}/${$active.channelId}#message-${message.id}`;
      items.push({
        label: "Copy message link",
        icon: "link",
        run: () => void copyText(link, "Message link"),
      });
    }
    items.push({
      label: "Copy message ID",
      icon: "hash",
      run: () => void copyText(message.id, "Message ID"),
    });
    openContextMenu(event.clientX, event.clientY, items);
  }

  async function attach() {
    if (!canAttach) {
      notify("You do not have permission to attach files here.");
      return;
    }
    const paths = await api.pickFiles();
    if (paths.length) await attachFiles(paths);
  }

  function grouped(index: number): boolean {
    if (index <= 0) return false;
    const current = $messages[index];
    const previous = $messages[index - 1];
    if (current.type === "system" || previous.type === "system") return false;
    const currentSender = senderOf(current);
    const previousSender = senderOf(previous);
    if (!currentSender || !previousSender) return false;
    if (currentSender.id !== previousSender.id) return false;
    if (!sameDay(current.created_at, previous.created_at)) return false;
    return (
      new Date(current.created_at).getTime() - new Date(previous.created_at).getTime() <
      5 * 60 * 1000
    );
  }

  function newDay(index: number): boolean {
    if ($messages[index]?.type === "system") return false;
    if (index === 0) return true;
    return !sameDay($messages[index - 1].created_at, $messages[index].created_at);
  }

  function replyPreview(message: Message): { name: string; content: string } | null {
    const reply = message.reply_to;
    if (!reply) return null;
    const found = $messages.find((item) => item.id === reply.id);
    return {
      name:
        senderOf(found ?? null)?.display_name ||
        senderOf(found ?? null)?.username ||
        reply.sender_display_name ||
        reply.sender_username ||
        "a message",
      content: found?.content || reply.content || "Original message",
    };
  }

  function replyName(message: Message): string {
    return replyPreview(message)?.name ?? "a message";
  }

  function replyContent(message: Message): string {
    return truncateText(replyPreview(message)?.content ?? "");
  }

  function senderOf(message: Message | null) {
    if (!message) return null;
    if (message.sender) return message.sender;
    if (message.author) return message.author;
    const id = message.sender_id;
    const member = id ? roster.find((entry) => entry.user.id === id) : undefined;
    if (member) return member.user;
    if (!id && !message.sender_username) return null;
    return {
      id: id ?? "",
      username: message.sender_username ?? "deleted-user",
      display_name: message.sender_display_name ?? null,
      avatar_url: message.sender_avatar_url ?? null,
      status: id ? presence[id] ?? "offline" : "offline",
    };
  }

  function authorName(message: Message | null): string {
    const sender = senderOf(message);
    return sender?.display_name || sender?.username || "Deleted user";
  }

  function authorColor(message: Message | null): string {
    const sender = senderOf(message);
    if (!sender) return "";
    const member = roster.find((entry) => entry.user.id === sender.id);
    const ids = member?.role_ids ?? member?.roles ?? [];
    const role = roleList
      .filter(
        (entry) =>
          !entry.is_everyone &&
          ids.includes(entry.id) &&
          /^#[0-9a-f]{3,8}$/i.test(entry.color ?? ""),
      )
      .sort((a, b) => (b.position ?? 0) - (a.position ?? 0))[0];
    return role?.color ?? "";
  }

  function typingIndicator(user: User) {
    if (!user.typing_indicator_enabled) return null;
    const text = user.typing_indicator_text?.trim() ?? "";
    if (text.length < 2 || text.length > 40 || !/^(is|are|was)\s+\S/i.test(text)) {
      return null;
    }
    const style = user.typing_indicator_style === "emojis" ? "emojis" : "dots";
    const emojis = (user.typing_indicator_emojis ?? [])
      .map((emoji) => emoji.trim())
      .filter(Boolean);
    if (style === "emojis" && emojis.length !== 3) return null;
    return { text, style, emojis };
  }

  function isMine(message: Message): boolean {
    const sender = senderOf(message);
    return !!sender && sender.id === $me?.id;
  }

  function openSenderProfile(event: MouseEvent, message: Message) {
    const user = senderOf(message);
    if (!user) return;
    const member = roster.find((entry) => entry.user.id === user.id);
    openProfile(user, member, event.currentTarget as HTMLElement | null);
  }

  function openSenderMenu(event: MouseEvent, message: Message) {
    const user = senderOf(message);
    if (!user) return;
    const member = roster.find((entry) => entry.user.id === user.id);
    showUserMenu(event, user, member);
  }

  async function focusMessage(messageId: string) {
    await tick();
    const find = () =>
      scroller?.querySelector(
        `[data-message-id="${CSS.escape(messageId)}"]`,
      ) as HTMLElement | null;
    let element = find();
    if (!element) {
      const loaded = await loadMessagesAround(messageId);
      await tick();
      element = loaded ? find() : null;
    }
    if (!element) {
      notify("That message is outside the loaded history.");
      return;
    }
    highlightId = messageId;
    element.scrollIntoView({ block: "center", behavior: "smooth" });
    setTimeout(() => {
      if (highlightId === messageId) highlightId = null;
    }, 1800);
  }

  async function jumpToMessage(serverId: string, channelId: string, messageId: string) {
    if ($active?.kind !== "guild" || $active.serverId !== serverId || $active.channelId !== channelId) {
      const server = $servers.find((entry) => entry.id === serverId);
      if (!server) {
        notify("That message link points at a server you cannot open.");
        return;
      }
      await openGuild(server, channelId);
    }
    await focusMessage(messageId);
  }

  async function jumpToReply(event: Event, message: Message) {
    const reply = message.reply_to;
    if (!reply) return;
    const target = event.target as HTMLElement;
    if (target.closest("a, button")) return;
    if ($active?.kind === "guild") {
      await jumpToMessage($active.serverId, $active.channelId, reply.id);
    } else {
      await focusMessage(reply.id);
    }
  }

  function onContentClick(event: MouseEvent) {
    const target = event.target as HTMLElement;

    const userChip = target.closest("[data-user-id]") as HTMLElement | null;
    if (userChip) {
      const id = userChip.dataset.userId;
      if (!id) return;
      const member = roster.find((entry) => entry.user.id === id);
      const user =
        member?.user ?? $conversations.flatMap((conversation) => conversation.members ?? [])
          .find((entry) => entry.user.id === id)?.user;
      if (user) openProfile(user, member, userChip);
      return;
    }

    const messageLinkElement = target.closest("[data-message-link]") as HTMLElement | null;
    if (messageLinkElement) {
      event.preventDefault();
      const { serverId, channelId, messageId } = messageLinkElement.dataset;
      if (serverId && channelId && messageId) {
        void jumpToMessage(serverId, channelId, messageId);
      }
      return;
    }

    const channelChip = target.closest("[data-channel-id]") as HTMLElement | null;
    if (channelChip && $active?.kind === "guild") {
      const id = channelChip.dataset.channelId;
      const channel = channelList.find((entry) => entry.id === id);
      const server = $servers.find((entry) => entry.id === $active?.serverId);
      if (channel && server) {
        void openGuild(server, channel.id);
      }
      return;
    }

    const anchor = target.closest("a[data-external]") as HTMLAnchorElement | null;
    if (anchor) {
      event.preventDefault();
      void api.openExternal(anchor.href);
    }
  }

  function isImageAttachment(attachment: Attachment): boolean {
    if (attachment.content_type?.startsWith("image/")) return true;
    return /\.(png|jpe?g|gif|webp|avif|svg)(\?|$)/i.test(
      attachment.file_name ?? attachment.url ?? "",
    );
  }

  function isImage(message: Message): boolean {
    return message.attachments?.some(
      (attachment) => isImageAttachment(attachment) && Boolean(attachment.url),
    ) ?? false;
  }

  function isVideo(attachment: Attachment): boolean {
    if (attachment.content_type?.startsWith("video/")) return true;
    return /\.(mp4|webm|mov|m4v)(\?|$)/i.test(attachment.file_name ?? attachment.url ?? "");
  }

  function expiryText(attachment: Attachment): string {
    if (attachment.saved) return "Saved";
    if (!attachment.expires_at) return "";
    const time = new Date(attachment.expires_at).getTime();
    if (!Number.isFinite(time)) return "";
    const remaining = time - Date.now();
    if (remaining <= 0) return "Expired";
    const minutes = Math.ceil(remaining / 60000);
    if (minutes <= 1) return "Expires in less than a minute";
    if (minutes < 60) return `Expires in ${minutes} minutes`;
    const hours = Math.round(minutes / 60);
    if (hours < 48) return `Expires in ${hours} hour${hours === 1 ? "" : "s"}`;
    const days = Math.round(hours / 24);
    if (days < 14) return `Expires in ${days} day${days === 1 ? "" : "s"}`;
    return `Expires ${new Date(time).toLocaleString()}`;
  }

  function flattenEmbeds(embeds: Embed[] | undefined): Embed[] {
    if (!embeds) return [];
    const result: Embed[] = [];
    for (const embed of embeds) {
      if (!embed || typeof embed !== "object") continue;
      const nested = (embed as { embeds?: unknown }).embeds;
      if (Array.isArray(nested)) result.push(...(nested as Embed[]));
      else result.push(embed);
    }
    return result;
  }

  function embedSite(embed: Embed): string {
    return embed.site_name || embed.siteName || embed.provider_name || embed.providerName || "";
  }

  function embedTitle(embed: Embed): string {
    return embed.title ?? "";
  }

  function embedDescription(embed: Embed): string {
    return embed.description ?? "";
  }

  function embedImage(embed: Embed): string | null {
    const image = embed.image_url || embed.imageUrl || embed.thumbnail_url || embed.thumbnailUrl;
    return typeof image === "string" ? image : null;
  }

  function embedVideo(embed: Embed): string | null {
    const video = embed.video_url || embed.videoUrl;
    return typeof video === "string" ? video : null;
  }

  function hasEmbedBody(embed: Embed): boolean {
    return Boolean(
      embedTitle(embed) ||
        embedDescription(embed) ||
        embedImage(embed) ||
        embedVideo(embed) ||
        (embed.fields?.length ?? 0) > 0,
    );
  }

  function reactionTitle(message: Message, emoji: string, count: number): string {
    if ($prefs.whoReacted) {
      const reaction = message.reactions?.find((entry) => entry.emoji === emoji);
      const names = (reaction?.users ?? [])
        .map((user) => user.display_name || user.username)
        .filter(Boolean);
      if (names.length > 0) {
        return `${names.slice(0, 8).join(", ")}${names.length > 8 ? ` and ${names.length - 8} more` : ""}`;
      }
    }
    return `${count} reaction${count === 1 ? "" : "s"}`;
  }

  function senderUsername(message: Message | null): string {
    return senderOf(message)?.username ?? "";
  }

  function onMediaLoad() {
    if (atBottom) {
      void tick().then(() => scrollToBottom(false));
    }
  }

  function onDrop(event: DragEvent) {
    event.preventDefault();
    dragOver = false;
    if (!canAttach) {
      notify("You do not have permission to attach files here.");
      return;
    }
    const files = Array.from(event.dataTransfer?.files ?? []);
    if (files.length) {
      notify("Drop files from your file manager in the desktop window to upload them.");
    }
  }

  function onDragOver(event: DragEvent) {
    event.preventDefault();
    dragOver = true;
  }

  function onDragLeave() {
    dragOver = false;
  }
</script>

<section class="chat">
  <header class="chat-head">
    <div class="head-main">
      <span class="head-icon">
        {#if $active?.kind === "guild"}
          <ChannelIcon
            name={$active.icon}
            color={$active.icon_color}
            size={19}
            fallback={isStats ? "chart" : "hash"}
          />
        {:else}
          <Icon name="dm" size={19} />
        {/if}
      </span>
      <div class="head-copy">
        <h1><EmojiText text={channelTitle} size={15.5} /></h1>
        {#if activeTopic}<p>{activeTopic}</p>{/if}
      </div>
    </div>
    <div class="head-actions">
      {#if isStats && canViewElevatedStats}
        <button
          class="icon-btn"
          class:active={$statsElevatedVisible}
          aria-label={$statsElevatedVisible ? "Hide elevated stats" : "Show elevated stats"}
          title={$statsElevatedVisible ? "Hide elevated stats" : "Show elevated stats"}
          aria-pressed={$statsElevatedVisible}
          on:click={() => statsElevatedVisible.update((value) => !value)}
        >
          <Icon name="chart" size={18} />
        </button>
      {/if}
      {#if $active?.kind === "guild"}
        <button
          class="icon-btn"
          class:active={$showMembers}
          aria-label="Toggle member list"
          title="Toggle member list"
          on:click={() => showMembers.update((value) => !value)}
        >
          <Icon name="users" size={18} />
        </button>
      {/if}
    </div>
  </header>

  {#if isStats && $active?.kind === "guild"}
    <StatsChannel
      serverId={$active.serverId}
      channelId={$active.channelId}
      permissionBits={$activePermissions}
    />
  {:else}
  <div class="messages" bind:this={scroller} on:scroll={onScroll}>
    <div class="messages-inner" class:empty={$messages.length === 0 && !$loadingMessages}>
      {#if $loadingMessages && $messages.length === 0}
        <div class="message-skeleton" role="status" aria-label="Loading messages">
          {#each messageSkeletons as row, index}
            <div class="skeleton-row">
              <span class="skeleton-avatar" style={`animation-delay: ${index * 70}ms`}></span>
              <div class="skeleton-body">
                <div class="skeleton-meta">
                  <span
                    class="skeleton-name"
                    style={`width: ${row.name}px; animation-delay: ${index * 70}ms`}
                  ></span>
                  <span
                    class="skeleton-time"
                    style={`animation-delay: ${index * 70}ms`}
                  ></span>
                </div>
                {#each row.lines as width, lineIndex}
                  <span
                    class="skeleton-line"
                    style={`width: ${width}%; animation-delay: ${(index + lineIndex) * 70}ms`}
                  ></span>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {:else if $messages.length === 0}
        <div class="channel-start">
          <div class="start-icon">
            <Icon name={$active?.kind === "dm" ? "dm" : "hash"} size={24} />
          </div>
          <h2>{$active?.kind === "dm" ? channelTitle : `Welcome to #${channelTitle}`}</h2>
          <p>
            {$active?.kind === "dm"
              ? "This is the beginning of your conversation."
              : "This is the start of the channel. Say something worth pinning."}
          </p>
        </div>
      {/if}

      {#each $messages as message, index (message.id)}
        {#if newDay(index)}
          <div class="day"><span>{formatDay(message.created_at)}</span></div>
        {/if}

        {#if $newMessagesMarker?.key === currentKey && $newMessagesMarker.id === message.id}
          <div class="new-divider" role="separator" aria-label="New messages"><span>New messages</span></div>
        {/if}

        {#if message.type === "system"}
          <div class="system">
            <span>{message.content}</span>
          </div>
        {:else}
          <article
            class="message"
            class:grouped={grouped(index)}
            class:pending={message.pending}
            class:failed={message.failed}
            class:mine={isMine(message)}
            class:highlight={highlightId === message.id}
            data-message-id={message.id}
            on:contextmenu|preventDefault={(event) => showMessageMenu(event, message)}
            on:dblclick={() => {
              if ($prefs.messageClickActions) beginReply(message);
            }}
          >
            {#if !grouped(index)}
              <button
                class="avatar-hit"
                aria-label={`Open ${authorName(message)} profile`}
                on:click={(event) => openSenderProfile(event, message)}
                on:contextmenu|preventDefault={(event) => openSenderMenu(event, message)}
              >
                <Avatar user={senderOf(message)} size={38} status={false} />
              </button>
            {:else}
              <span class="gutter-time">{formatTime(message.created_at, $prefs.timeFormat)}</span>
            {/if}

            <div class="body">
              {#if replyPreview(message)}
                <div
                  class="reply clickable"
                  role="button"
                  tabindex="0"
                  title={`Jump to ${replyName(message)}'s message`}
                  on:click={(event) => {
                    event.stopPropagation();
                    void jumpToReply(event, message);
                  }}
                  on:keydown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      void jumpToReply(event, message);
                    }
                  }}
                >
                  <span class="reply-line"></span>
                  <Icon name="reply" size={12} />
                  <b>{replyName(message)}</b>
                  <span class="reply-content">{@html renderContent(replyContent(message), renderContext)}</span>
                </div>
              {/if}

              {#if !grouped(index)}
                <div class="meta">
                  <button
                    class="author"
                    style={authorColor(message) ? `color:${authorColor(message)}` : ""}
                    on:click={(event) => openSenderProfile(event, message)}
                    on:contextmenu|preventDefault={(event) => openSenderMenu(event, message)}
                  >{authorName(message)}{#if $prefs.showMeYourName && senderUsername(message) && senderUsername(message) !== authorName(message)}<span class="username">@{senderUsername(message)}</span>{/if}</button>
                  <span class="time">{formatTime(message.created_at, $prefs.timeFormat)}</span>
                </div>
              {/if}

              {#if message.content}
                <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
                <div class="content" on:click={onContentClick}>
                  {@html renderContent(message.content, renderContext)}
                </div>
              {/if}

              {#if message.edited_at}
                <span class="edited">(edited)</span>
              {/if}

              {#if message.attachments && message.attachments.length > 0}
                <div class="attachments">
                  {#each message.attachments as attachment}
                    {#if isImageAttachment(attachment)}
                      {#if attachment.url}
                        <button
                          class="attachment image"
                          title={attachment.file_name ?? "View image"}
                          on:click={() =>
                            openImage(
                              attachment.url,
                              attachment.file_name ?? "Image",
                            )}
                        >
                          <img
                            src={attachment.url}
                            alt={attachment.file_name ?? ""}
                            loading="lazy"
                            on:load={onMediaLoad}
                          />
                          {#if $prefs.showMediaExpiry && expiryText(attachment)}
                            <span class="media-note">{expiryText(attachment)}</span>
                          {/if}
                        </button>
                      {:else}
                        <div class="attachment placeholder">
                          <Icon name="image" size={22} />
                          <span>{attachment.file_name ?? "image"}</span>
                          <small>{formatSize(attachment.file_size)}</small>
                        </div>
                      {/if}
                    {:else if isVideo(attachment) && attachment.url}
                      <div class="attachment video">
                        <VideoPlayer
                          src={attachment.url}
                          showPip={$prefs.pictureInPicture}
                          onLoaded={onMediaLoad}
                        />
                        {#if $prefs.showMediaExpiry && expiryText(attachment)}
                          <span class="media-note video-note">{expiryText(attachment)}</span>
                        {/if}
                      </div>
                    {:else}
                      <button
                        class="attachment file"
                        on:click={() => attachment.url && api.openExternal(attachment.url)}
                      >
                        <span class="file-icon"><Icon name="paperclip" size={17} /></span>
                        <span class="file-copy">
                          <b>{attachment.file_name ?? "file"}</b>
                          <small>{formatSize(attachment.file_size)}</small>
                          {#if $prefs.showMediaExpiry && expiryText(attachment)}
                            <small class="expiry">{expiryText(attachment)}</small>
                          {/if}
                        </span>
                        <Icon name="download" size={15} />
                      </button>
                    {/if}
                  {/each}
                </div>
              {/if}

              {#each flattenEmbeds(message.embeds) as embed}
                {#if hasEmbedBody(embed)}
                  <div class="embed">
                    <div class="embed-main">
                      {#if embedSite(embed)}<span class="embed-site">{embedSite(embed)}</span>{/if}
                      {#if embedTitle(embed)}
                        {#if embed.url}
                          <button class="embed-title" on:click={() => embed.url && api.openExternal(embed.url)}>
                            {embedTitle(embed)}
                          </button>
                        {:else}
                          <span class="embed-title static">{embedTitle(embed)}</span>
                        {/if}
                      {/if}
                      {#if embedDescription(embed)}
                        <p class="embed-description">{embedDescription(embed)}</p>
                      {/if}
                      {#if embed.fields && embed.fields.length > 0}
                        <div class="embed-fields">
                          {#each embed.fields as field}
                            <div class:inline={field.inline}>
                              <span>{field.name ?? ""}</span>
                              <p>{field.value ?? ""}</p>
                            </div>
                          {/each}
                        </div>
                      {/if}
                      {#if embedVideo(embed)}
                        <VideoPlayer
                          src={embedVideo(embed) ?? ""}
                          showPip={$prefs.pictureInPicture}
                          onLoaded={onMediaLoad}
                        />
                      {:else if embedImage(embed)}
                        <button
                          class="embed-image"
                          title="View image"
                          on:click={() =>
                            openImage(
                              embedImage(embed),
                              embedTitle(embed) || "Embed image",
                            )}
                        >
                          <img src={embedImage(embed) ?? ""} alt="" loading="lazy" on:load={onMediaLoad} />
                        </button>
                      {/if}
                    </div>
                  </div>
                {/if}
              {/each}

              {#if message.reactions && message.reactions.length > 0}
                <div class="reactions">
                  {#each message.reactions as reaction}
                    <button
                      class="reaction"
                      class:me={reaction.me}
                      title={reactionTitle(message, reaction.emoji, reaction.count)}
                      on:click={() => toggle(message, reaction.emoji)}
                    >
                      <Emoji char={reaction.emoji} size={15} title={reaction.emoji} />
                      <b>{reaction.count}</b>
                    </button>
                  {/each}
                  {#if canReact}
                    <button
                      class="reaction add"
                      title="Add reaction"
                      aria-label="Add reaction"
                      on:click={(event) => openReactionPicker(event, message)}
                    >
                      <Icon name="smile" size={14} />
                    </button>
                  {/if}
                </div>
              {/if}
            </div>

            <div class="actions">
              {#if canReact}
                <button
                  title="Add reaction"
                  aria-label="Add reaction"
                  on:click={(event) => openReactionPicker(event, message)}
                >
                  <Icon name="smile" size={15} />
                </button>
                {#if $prefs.quickReactions}
                  {#each quickReactions as quick}
                    <button
                      class="quick"
                      title={`React with ${quick}`}
                      aria-label={`React with ${quick}`}
                      on:click={() => toggle(message, quick)}
                    >
                      <Emoji char={quick} size={14} />
                    </button>
                  {/each}
                {/if}
              {/if}
              <button title="Reply" aria-label="Reply" on:click={() => beginReply(message)}>
                <Icon name="reply" size={15} />
              </button>
              {#if isMine(message) && !message.pending}
                <button title="Edit" aria-label="Edit" on:click={() => beginEdit(message)}>
                  <Icon name="pencil" size={14} />
                </button>
                <button
                  title="Delete"
                  aria-label="Delete"
                  class="danger"
                  on:click={() => confirmDelete(message)}
                >
                  <Icon name="trash" size={14} />
                </button>
              {/if}
            </div>

            {#if message.pending}
              <span class="delivery" title="Sending"><Icon name="clock" size={12} /></span>
            {:else if message.failed}
              <span class="delivery failed" title="Failed to send"><Icon name="alert" size={12} /></span>
            {/if}
          </article>
        {/if}
      {/each}
    </div>
  </div>

  {#if showJump}
    <button class="jump" on:click={() => scrollToBottom(true)}>
      <Icon name="chevron" size={14} /> Jump to present
    </button>
  {/if}

  <div
    class="composer-zone"
    role="region"
    aria-label="Message composer"
    class:drag={dragOver}
    on:dragover={onDragOver}
    on:dragleave={onDragLeave}
    on:drop={onDrop}
  >
    <div class="typing-row">
      {#if typingLabel}
        <span class="typing" class:static={$prefs.noTypingAnimation}>
          {#if singleTypingIndicator && singleTypingIndicator.style === "emojis"}
            <span class="typing-emojis">
              {#each singleTypingIndicator.emojis as emoji}
                <Emoji char={emoji} size={14} />
              {/each}
            </span>
          {:else}
            <i></i><i></i><i></i>
          {/if}
          {#if singleTyping && singleTypingUser && singleTypingIndicator?.style !== "emojis"}
            <Avatar user={singleTypingUser} size={16} status={false} />
          {/if}
          {typingLabel}
        </span>
      {/if}
    </div>

    {#if $pendingAttachments.length > 0 || $uploadingAttachments}
      <div class="pending-files">
        {#if $uploadingAttachments}
          <span class="uploading"><span class="spin"></span> Uploading…</span>
        {/if}
        {#each $pendingAttachments as attachment, index}
          <div class="pending-file" title={attachment.file_name ?? "Attachment"}>
            <Icon name="paperclip" size={13} />
            <span>{attachment.file_name ?? "file"}</span>
            <small>{formatSize(attachment.file_size)}</small>
            {#if $prefs.showMediaExpiry && expiryText(attachment)}
              <small class="expiry">{expiryText(attachment)}</small>
            {/if}
            <button
              aria-label={`Remove ${attachment.file_name ?? "attachment"}`}
              on:click={() => removePendingAttachment(index)}
            >
              <Icon name="close" size={12} />
            </button>
          </div>
        {/each}
      </div>
    {/if}

    {#if suggestions.length > 0}
      <div class="suggestions" role="listbox" aria-label="Suggestions">
        <div class="sug-head">
          {suggestionHeading}
          <span>↑↓ · Tab to insert</span>
        </div>
        {#each suggestions as suggestion, index (suggestion.id)}
          <button
            class="suggestion"
            class:selected={index === suggestionIndex}
            role="option"
            aria-selected={index === suggestionIndex}
            on:mousedown|preventDefault
            on:click={() => applySuggestion(suggestion)}
          >
            {#if suggestion.kind === "user"}
              <Avatar user={suggestion.user ?? null} size={22} status={false} />
            {:else if suggestion.kind === "channel" && suggestion.channel}
              <span class="sug-icon"><ChannelIcon name={suggestion.channel.icon} color={suggestion.channel.icon_color} size={14} fallback={suggestion.channel.type === "voice" ? "speaker" : suggestion.channel.type === "stats" ? "chart" : "hash"} /></span>
            {:else if suggestion.kind === "role" && suggestion.role}
              <span
                class="sug-role"
                style={suggestion.role.color ? `--role-color:${suggestion.role.color}` : ""}
              >
                {#if suggestion.role.icon_url}
                  <img src={suggestion.role.icon_url} alt="" />
                {:else if suggestion.role.icon_emoji}
                  <Emoji char={suggestion.role.icon_emoji} size={14} />
                {:else}
                  <i></i>
                {/if}
              </span>
            {:else if suggestion.char}
              <Emoji char={suggestion.char} size={18} />
            {/if}
            <span
              class="sug-label"
              class:role-label={suggestion.kind === "role"}
              style={suggestion.role?.color ? `color:${suggestion.role.color}` : ""}
            >{suggestion.label}</span>
            <span class="sug-detail">{suggestion.detail}</span>
          </button>
        {/each}
      </div>
    {/if}

    <div class="composer-shell" class:with-banner={!!replyTarget || !!editingTarget}>
      {#if replyTarget || editingTarget}
      <div class="composer-banner">
        <Icon name={editingTarget ? "pencil" : "reply"} size={13} />
        <span>
          {#if editingTarget}
            Editing your message
          {:else if replyTarget}
            Replying to <b>{authorName(replyTarget)}</b>
          {/if}
        </span>
        <button
          class="banner-close"
          aria-label="Cancel"
          on:click={() => {
            const wasEditing = !!editingTarget;
            editingTarget = null;
            replyTarget = null;
            suggestions = [];
            if (wasEditing) clearComposer();
            else resizeSoon();
          }}
        >
          <Icon name="close" size={13} />
        </button>
      </div>
    {/if}

    <div class="composer">
      <button
        class="composer-btn"
        title={canAttach ? "Attach a file" : "You cannot attach files here"}
        aria-label="Attach a file"
        disabled={!canAttach}
        on:click={attach}
      >
        <Icon name="paperclip" size={18} />
      </button>
      <textarea
        bind:this={composer}
        bind:value={text}
        on:input={onInput}
        on:keydown={onComposerKey}
        rows="1"
        spellcheck="false"
        disabled={!canSendMessages}
        placeholder={!canSendMessages
          ? "You do not have permission to send messages here"
          : $active?.kind === "dm"
            ? `Message ${channelTitle}`
            : `Message #${channelTitle}`}
      ></textarea>
      <button
        class="composer-btn"
        class:active={picker !== null && !picker.message}
        title="Emoji"
        aria-label="Emoji"
        disabled={!canSendMessages}
        on:click={openComposerPicker}
      >
        <Icon name="smile" size={18} />
      </button>
      <button
        class="send"
        class:ready={canSend}
        title={canSendMessages ? "Send" : "You cannot send messages here"}
        aria-label="Send"
        disabled={!canSend}
        on:click={() => submit()}
      >
        <Icon name="send" size={17} />
      </button>
    </div>

    {#if $prefs.previewMessage && text.trim() && !editingTarget}
      <div class="message-preview">
        <span class="preview-label">Preview</span>
        <div class="preview-body">{@html renderContent(text, renderContext)}</div>
      </div>
    {/if}
    {#if $prefs.characterCounter}
      <div class="composer-meta">
        <span class:limit={text.length > 2000}>{text.length}/2000</span>
      </div>
    {/if}
    </div>
  </div>
  {/if}
</section>

{#if picker}
  <EmojiPicker
    x={picker.x}
    y={picker.y}
    onSelect={onEmojiSelect}
    onClose={() => (picker = null)}
  />
{/if}

<style>
  .chat {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--chat);
    position: relative;
  }
  .chat-head {
    height: 58px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 0 16px 0 20px;
    border-bottom: 1px solid var(--line);
    background: color-mix(in srgb, var(--chat) 90%, transparent);
    backdrop-filter: blur(12px);
    z-index: 4;
  }
  .head-main {
    display: flex;
    align-items: center;
    gap: 11px;
    min-width: 0;
  }
  .head-icon {
    width: 32px;
    height: 32px;
    border-radius: 9px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    display: grid;
    place-items: center;
    color: var(--muted);
    flex: none;
  }
  .head-copy {
    min-width: 0;
  }
  .head-copy h1 {
    font-size: 15.5px;
    font-weight: 650;
    letter-spacing: -0.025em;
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .head-copy p {
    margin: 1px 0 0;
    font-size: 11.5px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 62vw;
  }
  .head-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex: none;
  }
  .messages {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    min-height: 0;
    scroll-behavior: auto;
  }
  .messages-inner {
    width: 100%;
    max-width: 1000px;
    margin: 0 auto;
    padding: 22px 24px 10px;
    min-height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
  }
  .messages-inner > :global(*) {
    flex: none;
  }
  .messages-inner.empty {
    justify-content: center;
  }
  .message-skeleton {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px 4px 8px;
  }
  .skeleton-row {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    min-width: 0;
  }
  .skeleton-avatar {
    width: 38px;
    height: 38px;
    border-radius: 50%;
    background: var(--panel-3);
    flex: none;
    animation: skeleton-pulse 1.35s ease-in-out infinite;
  }
  .skeleton-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding-top: 2px;
  }
  .skeleton-meta {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .skeleton-name,
  .skeleton-time,
  .skeleton-line {
    display: block;
    background: var(--panel-3);
    border-radius: 99px;
    animation: skeleton-pulse 1.35s ease-in-out infinite;
  }
  .skeleton-name {
    height: 9px;
  }
  .skeleton-time {
    width: 32px;
    height: 7px;
    opacity: 0.7;
  }
  .skeleton-line {
    height: 10px;
    max-width: 100%;
  }
  @keyframes skeleton-pulse {
    0%,
    100% {
      opacity: 0.38;
    }
    50% {
      opacity: 0.78;
    }
  }
  .channel-start {
    padding: 28px 0 34px;
    max-width: 460px;
  }
  .start-icon {
    width: 48px;
    height: 48px;
    border-radius: 14px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    display: grid;
    place-items: center;
    color: var(--muted);
    margin-bottom: 14px;
  }
  .channel-start h2 {
    font-size: 23px;
    letter-spacing: -0.035em;
    margin: 0 0 5px;
  }
  .channel-start p {
    color: var(--muted);
    font-size: 12.5px;
    margin: 0;
    line-height: 1.55;
  }
  .day {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 18px 0 12px;
    color: var(--muted);
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .day::before,
  .day::after {
    content: "";
    height: 1px;
    background: var(--line);
    flex: 1;
  }
  .new-divider {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 10px 0 6px;
    color: #ff6b6b;
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .new-divider::before,
  .new-divider::after {
    content: "";
    height: 1px;
    flex: 1;
    background: rgba(var(--accent-rgb), 0.55);
  }
  .message {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: var(--message-pad-y) 8px var(--message-pad-y) 4px;
    margin: 0 -8px 0 -4px;
    border-radius: 8px;
    position: relative;
  }
  .message:hover {
    background: color-mix(in srgb, var(--hover) 70%, transparent);
  }
  .message.grouped {
    padding-top: 1px;
    padding-bottom: 1px;
  }
  .message.failed .content {
    color: #ff9a9a;
  }
  .body {
    flex: 1;
    min-width: 0;
    padding-top: 1px;
  }
  .meta {
    display: flex;
    align-items: baseline;
    gap: 7px;
    margin-bottom: 1px;
  }
  .author {
    font-size: calc(var(--chat-font-size) - 0.9px);
    font-weight: 650;
    letter-spacing: -0.012em;
    color: var(--text);
    border: 0;
    background: transparent;
    padding: 0;
    font-family: inherit;
  }
  .author:hover {
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .author .username {
    margin-left: 5px;
    font-size: 10px;
    font-weight: 500;
    color: #737a8b;
  }
  .avatar-hit {
    border: 0;
    background: transparent;
    padding: 0;
    margin: 0;
    flex: none;
    border-radius: 50%;
    line-height: 0;
  }
  .avatar-hit:hover {
    filter: brightness(1.08);
  }
  .time {
    font-size: 10.5px;
    color: #636a7b;
  }
  .gutter-time {
    width: 38px;
    flex: none;
    text-align: right;
    font-size: 9px;
    color: #5f6677;
    opacity: 0;
    padding-top: 4px;
    user-select: none;
  }
  .message:hover .gutter-time {
    opacity: 1;
  }
  .content {
    font-size: var(--chat-font-size);
    line-height: 1.57;
    color: var(--text-dim);
    overflow-wrap: anywhere;
  }
  .content :global(strong) {
    color: var(--text);
    font-weight: 650;
  }
  .content :global(em) {
    color: var(--text);
  }
  .content :global(a) {
    color: #ff6b6b;
    text-decoration: none;
    border-bottom: 1px solid rgba(255, 107, 107, 0.3);
  }
  .content :global(a:hover) {
    border-bottom-color: #ff6b6b;
  }
  .content :global(code) {
    font-family: "Cascadia Code", "SF Mono", "JetBrains Mono", ui-monospace, monospace;
    font-size: 11.5px;
    background: var(--input);
    border: 1px solid var(--line);
    padding: 1px 4px;
    border-radius: 4px;
    color: #e6b3b3;
  }
  .content :global(pre) {
    background: var(--input);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 10px 12px;
    overflow-x: auto;
    margin: 5px 0;
  }
  .content :global(pre code) {
    background: transparent;
    border: 0;
    padding: 0;
    color: var(--text-dim);
  }
  .content :global(.mention) {
    color: #ffb3b3;
    background: var(--accent-dim);
    border-radius: 4px;
    padding: 0 3px;
    font-weight: 550;
  }
  .content :global(.role-mention) {
    color: color-mix(in srgb, var(--role, var(--accent)) 78%, white);
    background: color-mix(in srgb, var(--role, var(--accent)) 16%, transparent);
    cursor: default;
  }
  .content :global(.mention-role-icon) {
    width: 1.15em;
    height: 1.15em;
    object-fit: contain;
    display: inline-block;
    vertical-align: -0.2em;
    margin-right: 2px;
  }
  .content :global(.mention-avatar) {
    width: 1.2em;
    height: 1.2em;
    border-radius: 50%;
    object-fit: cover;
    display: inline-block;
    vertical-align: -0.25em;
    margin-right: 2px;
  }
  .content :global(.user-mention),
  .content :global(.channel-mention) {
    color: color-mix(in srgb, var(--accent) 72%, white);
    background: var(--accent-dim);
    cursor: pointer;
  }
  .content :global(.user-mention:hover),
  .content :global(.channel-mention:hover) {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
  }
  .content :global(.message-link) {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    max-width: 100%;
    margin: 1px 0;
    padding: 3px 7px;
    border: 1px solid var(--line-strong);
    border-left: 2px solid var(--accent);
    border-radius: 5px;
    background: var(--panel-2);
    color: var(--text-dim);
    text-decoration: none;
    font-size: 0.92em;
    vertical-align: 1px;
  }
  .content :global(.message-link:hover) {
    background: var(--panel-3);
    color: var(--text);
  }
  .content :global(.message-link-channel) {
    font-weight: 650;
    color: var(--text);
  }
  .content :global(.message-link-action) {
    color: var(--muted);
  }
  .content :global(img.message-emoji) {
    width: 1.3em;
    height: 1.3em;
    display: inline-block;
    vertical-align: -0.26em;
    user-select: none;
  }
  .message.highlight {
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    animation: highlight 1.8s ease-out;
  }
  @keyframes highlight {
    0% {
      background: color-mix(in srgb, var(--accent) 22%, transparent);
    }
    100% {
      background: color-mix(in srgb, var(--accent) 10%, transparent);
    }
  }
  .edited {
    font-size: 9px;
    color: var(--muted);
    margin-left: 3px;
  }
  .reply {
    display: flex;
    align-items: center;
    flex-wrap: nowrap;
    gap: 5px;
    font-size: 11.5px;
    color: var(--muted);
    margin: 2px 0 4px;
    max-width: 100%;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    border-radius: 5px;
  }
  .reply.clickable {
    cursor: pointer;
  }
  .reply.clickable:hover {
    background: var(--hover);
  }
  .reply.clickable:focus-visible {
    outline: 1px solid rgba(var(--accent-rgb), 0.5);
    outline-offset: 2px;
  }
  .reply b {
    color: var(--text-dim);
    font-weight: 600;
    flex: none;
    max-width: 45%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .reply-content {
    flex: 1;
    min-width: 0;
    opacity: 0.82;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
  }
  .reply :global(.mention) {
    padding: 0 2px;
    border-radius: 3px;
    background: var(--accent-dim);
  }
  .reply :global(.role-mention) {
    background: color-mix(in srgb, var(--role, var(--accent)) 16%, transparent);
  }
  .reply :global(img.message-emoji) {
    width: 1.1em;
    height: 1.1em;
    display: inline-block;
    vertical-align: -0.2em;
  }
  .reply-line {
    width: 18px;
    height: 9px;
    border-left: 1px solid var(--line-strong);
    border-top: 1px solid var(--line-strong);
    border-radius: 4px 0 0 0;
    margin-left: 4px;
    margin-top: 7px;
    flex: none;
  }
  .system {
    display: flex;
    justify-content: center;
    padding: 7px 0;
  }
  .system span {
    font-size: 10.5px;
    color: var(--muted);
    background: var(--panel-2);
    border: 1px solid var(--line);
    border-radius: 99px;
    padding: 4px 11px;
  }
  .attachments {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 6px;
    max-width: 520px;
  }
  .attachment {
    border: 1px solid var(--line);
    background: var(--panel-2);
    border-radius: 9px;
    overflow: hidden;
    padding: 0;
  }
  .attachment.image {
    max-width: 420px;
    display: block;
    position: relative;
  }
  .attachment.image img {
    display: block;
    max-width: 100%;
    max-height: 300px;
    object-fit: cover;
  }
  .attachment.video {
    position: relative;
    max-width: 400px;
    display: block;
    background: #000;
  }
  .attachment.video :global(.player) {
    width: 100%;
  }
  .attachment.video .media-note {
    top: 6px;
    bottom: auto;
  }
  .media-note {
    position: absolute;
    left: 6px;
    bottom: 6px;
    padding: 2px 6px;
    border-radius: 4px;
    background: rgba(12, 13, 16, 0.74);
    color: #d9dce3;
    font-size: 9.5px;
    pointer-events: none;
  }
  .file-copy small.expiry {
    color: #8a8f9c;
  }
  .embed {
    margin-top: 6px;
    max-width: 480px;
    border-left: 3px solid rgba(var(--accent-rgb), 0.55);
    border-radius: 4px 8px 8px 4px;
    background: var(--panel-2);
    padding: 9px 12px;
  }
  .embed-main {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .embed-site {
    font-size: 10px;
    color: var(--muted);
    text-transform: none;
  }
  .embed-title {
    padding: 0;
    border: 0;
    background: transparent;
    color: #ff8f8f;
    font-size: 12.5px;
    font-weight: 650;
    text-align: left;
  }
  .embed-title.static {
    cursor: default;
  }
  .embed-title:hover {
    text-decoration: underline;
  }
  .embed-title.static:hover {
    text-decoration: none;
  }
  .embed-description {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.45;
    color: var(--text-dim);
    white-space: pre-wrap;
  }
  .embed-fields {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 16px;
    margin-top: 2px;
  }
  .embed-fields > div {
    min-width: 120px;
    flex: 1 1 100%;
  }
  .embed-fields > div.inline {
    flex: 1 1 30%;
  }
  .embed-fields span {
    display: block;
    font-size: 10px;
    color: var(--muted);
    margin-bottom: 2px;
  }
  .embed-fields p {
    margin: 0;
    font-size: 11px;
    color: var(--text-dim);
    white-space: pre-wrap;
  }
  .embed-image {
    padding: 0;
    border: 0;
    border-radius: 6px;
    overflow: hidden;
    background: transparent;
    max-width: 400px;
    margin-top: 3px;
  }
  .embed-image img {
    display: block;
    max-width: 100%;
    max-height: 260px;
    object-fit: cover;
  }
  .embed :global(.player) {
    margin-top: 3px;
  }
  .attachment.placeholder {
    width: 260px;
    height: 90px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 14px;
    color: var(--muted);
    background: var(--panel-3);
  }
  .attachment.placeholder span {
    font-size: 12px;
    color: var(--text-dim);
    flex: 1;
  }
  .attachment.placeholder small {
    font-size: 10px;
  }
  .attachment.file {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 11px;
    min-width: 230px;
    max-width: 340px;
    color: var(--muted);
  }
  .attachment.file:hover {
    border-color: var(--line-strong);
    background: var(--panel-3);
  }
  .file-icon {
    width: 31px;
    height: 31px;
    border-radius: 7px;
    background: var(--input);
    border: 1px solid var(--line);
    display: grid;
    place-items: center;
    flex: none;
  }
  .file-copy {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    flex: 1;
    min-width: 0;
  }
  .file-copy b {
    font-size: 11.5px;
    color: var(--text);
    font-weight: 600;
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file-copy small {
    font-size: 9.5px;
    color: var(--muted);
  }
  .reactions {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 6px;
  }
  .reaction {
    height: 23px;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 7px;
    border-radius: 6px;
    border: 1px solid var(--line);
    background: var(--panel-2);
    font-size: 12px;
    color: var(--muted);
    line-height: 1;
  }
  .reaction b {
    font-size: 10px;
    font-weight: 650;
  }
  .reaction:hover {
    border-color: var(--line-strong);
  }
  .reaction.me {
    background: var(--accent-dim);
    border-color: rgba(var(--accent-rgb), 0.34);
    color: #ff8f8f;
  }
  .reaction.add {
    padding: 0 6px;
    opacity: 0;
  }
  .message:hover .reaction.add {
    opacity: 1;
  }
  .actions {
    position: absolute;
    top: -13px;
    right: 8px;
    height: 30px;
    display: flex;
    align-items: center;
    gap: 1px;
    padding: 2px;
    background: var(--panel-2);
    border: 1px solid var(--line-strong);
    border-radius: 8px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.28);
    opacity: 0;
    pointer-events: none;
    transform: translateY(3px);
    transition: opacity 100ms ease, transform 100ms ease;
    z-index: 3;
  }
  .message:hover .actions,
  .message:focus-within .actions {
    opacity: 1;
    pointer-events: auto;
    transform: none;
  }
  .actions button {
    width: 25px;
    height: 24px;
    border: 0;
    background: transparent;
    border-radius: 5px;
    display: grid;
    place-items: center;
    color: var(--muted);
  }
  .actions button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .actions button.danger:hover {
    background: rgba(var(--accent-rgb), 0.12);
    color: #ff7a7a;
  }
  .actions button.quick {
    padding: 0;
  }
  .delivery {
    position: absolute;
    left: -14px;
    bottom: 8px;
    color: var(--muted);
    display: flex;
  }
  .delivery.failed {
    color: var(--accent);
  }
  .jump {
    position: absolute;
    left: 50%;
    bottom: 108px;
    transform: translateX(-50%);
    height: 30px;
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 12px;
    border-radius: 99px;
    border: 1px solid var(--line-strong);
    background: var(--panel-2);
    color: var(--text-dim);
    font-size: 11px;
    box-shadow: var(--shadow);
    z-index: 6;
  }
  .jump :global(svg) {
    transform: rotate(180deg);
  }
  .jump:hover {
    color: var(--text);
  }
  .composer-zone {
    flex: none;
    padding: 0 20px 12px;
    position: relative;
  }
  .suggestions {
    position: absolute;
    left: 20px;
    right: 20px;
    bottom: calc(100% + 6px);
    max-height: 262px;
    overflow-y: auto;
    padding: 5px;
    border: 1px solid var(--line-strong);
    border-radius: 10px;
    background: var(--panel-2);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.38);
    z-index: 8;
  }
  .sug-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 7px 6px;
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .sug-head span {
    font-size: 9px;
    font-weight: 500;
    letter-spacing: 0;
    text-transform: none;
    color: #636a7b;
  }
  .suggestion {
    width: 100%;
    height: 32px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 7px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--text-dim);
    text-align: left;
  }
  .suggestion.selected {
    background: var(--hover);
    color: var(--text);
  }
  .sug-icon {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    flex: none;
  }
  .sug-role {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    flex: none;
  }
  .sug-role img {
    width: 18px;
    height: 18px;
    border-radius: 4px;
    object-fit: cover;
  }
  .sug-role i {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--role-color, var(--muted));
  }
  .sug-label.role-label {
    font-weight: 600;
  }
  .sug-label {
    font-size: 12px;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sug-detail {
    margin-left: auto;
    font-size: 10px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .typing-row {
    height: 18px;
    display: flex;
    align-items: center;
    padding: 0 4px;
  }
  .typing {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10.5px;
    color: var(--muted);
  }
  .typing i {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--muted);
    animation: blink 1.2s infinite ease-in-out;
  }
  .typing i:nth-child(2) {
    animation-delay: 0.15s;
  }
  .typing i:nth-child(3) {
    animation-delay: 0.3s;
    margin-right: 3px;
  }
  .typing-emojis {
    display: inline-flex;
    align-items: center;
    gap: 1px;
    margin-right: 3px;
  }
  .typing.static i {
    animation: none;
    opacity: 0.7;
  }
  @keyframes blink {
    0%,
    60%,
    100% {
      opacity: 0.35;
    }
    30% {
      opacity: 1;
    }
  }
  .pending-files {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    padding: 0 2px 7px;
  }
  .pending-file {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 27px;
    max-width: 260px;
    padding: 0 5px 0 8px;
    border: 1px solid var(--line-strong);
    border-radius: 7px;
    background: var(--panel-2);
    color: var(--muted);
    font-size: 10.5px;
  }
  .pending-file > span {
    max-width: 150px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-dim);
  }
  .pending-file small {
    font-size: 9px;
    color: var(--muted);
  }
  .pending-file button {
    width: 19px;
    height: 19px;
    flex: none;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--muted);
    display: grid;
    place-items: center;
  }
  .pending-file button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .uploading {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 27px;
    padding: 0 8px;
    font-size: 10.5px;
    color: var(--muted);
  }
  .uploading .spin {
    width: 12px;
    height: 12px;
    border-width: 2px;
  }
  .composer-shell {
    background: var(--input);
    border: 1px solid var(--line-strong);
    border-radius: 12px;
    overflow: hidden;
    transition: border-color 120ms ease, box-shadow 120ms ease;
  }
  .composer-shell:focus-within {
    border-color: rgba(var(--accent-rgb), 0.38);
    box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.06);
  }
  .composer-banner {
    min-height: 32px;
    display: flex;
    align-items: center;
    gap: 7px;
    background: var(--accent-dim);
    border-bottom: 1px solid rgba(var(--accent-rgb), 0.16);
    padding: 5px 10px;
    font-size: 10.5px;
    color: var(--text-dim);
  }
  .composer-banner b {
    color: var(--text);
  }
  .banner-close {
    margin-left: auto;
    width: 20px;
    height: 20px;
    border: 0;
    background: transparent;
    border-radius: 5px;
    display: grid;
    place-items: center;
    color: var(--muted);
    flex: none;
  }
  .banner-close:hover {
    background: var(--hover);
    color: var(--text);
  }
  .composer {
    display: flex;
    align-items: flex-end;
    gap: 4px;
    background: transparent;
    border: 0;
    border-radius: 0;
    padding: 6px;
  }
  .composer textarea {
    flex: 1;
    background: transparent;
    border: 0;
    outline: none;
    resize: none;
    color: var(--text);
    font-size: 14.4px;
    line-height: 1.5;
    padding: 7px 2px;
    max-height: 176px;
    min-height: 32px;
  }
  .composer textarea::placeholder {
    color: #565c6b;
  }
  .composer-btn {
    width: 32px;
    height: 32px;
    flex: none;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    display: grid;
    place-items: center;
  }
  .composer-btn:hover,
  .composer-btn.active {
    background: var(--hover);
    color: var(--text);
  }
  .composer-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .composer textarea:disabled {
    cursor: default;
    color: var(--muted);
  }
  .message-preview {
    border-top: 1px solid var(--line);
    padding: 8px 12px 9px;
    background: color-mix(in srgb, var(--panel-2) 55%, transparent);
  }
  .preview-label {
    display: block;
    margin-bottom: 4px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .preview-body {
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-dim);
    overflow-wrap: anywhere;
  }
  .composer-meta {
    display: flex;
    justify-content: flex-end;
    padding: 0 12px 6px;
    font-size: 9.5px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .composer-meta .limit {
    color: #ff7a7a;
  }
  .composer:has(textarea:disabled) {
    opacity: 0.72;
  }
  .send {
    width: 34px;
    height: 34px;
    flex: none;
    border: 0;
    border-radius: 8px;
    background: var(--panel-3);
    color: var(--muted);
    display: grid;
    place-items: center;
    transition: background 110ms ease, color 110ms ease;
  }
  .send.ready {
    background: var(--accent);
    color: white;
  }
  .send.ready:hover {
    background: color-mix(in srgb, var(--accent) 86%, white);
  }
  .send:disabled {
    cursor: default;
  }
  .composer-zone.drag::after {
    content: "Drop to upload";
    position: absolute;
    inset: 8px 20px 10px;
    border: 1px dashed rgba(var(--accent-rgb), 0.5);
    border-radius: 12px;
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    display: grid;
    place-items: center;
    font-size: 12px;
    color: #ff9a9a;
    pointer-events: none;
    z-index: 9;
  }
  @media (max-width: 1100px) {
    .messages-inner {
      padding-left: 18px;
      padding-right: 18px;
    }
    .composer-zone {
      padding-left: 14px;
      padding-right: 14px;
    }
  }
</style>
