<script lang="ts">
  import { onMount, tick } from "svelte";
  import { get } from "svelte/store";
  import * as api from "../api";
  import { avatarColor, initials } from "../format";
  import { hasPermission } from "../permissions";
  import { formatKeybind, matchesKeybind, prefs, type Keybinds } from "../preferences";
  import { openProfile } from "../profile";
  import { openContextMenu, type ContextMenuItem } from "../context-menu";
  import { confirmAction } from "../dialog";
  import {
    openCategoryCreate,
    openCategoryEdit,
    openChannelCreate,
    openChannelEdit,
  } from "../channel-dialog";
  import type { Category, Channel, ChannelBundle, Conversation, PaletteItemLike, Server } from "../types";
  import {
    active,
    channels,
    connection,
    conversations,
    ensureChannels,
    friendRequests,
    me,
    members,
    mentionCounts,
    modal,
    moveChannelToCategory,
    notify,
    openConversation,
    openGuild,
    reloadChannels,
    serverPermissions,
    serverSettings,
    servers,
    setServerNotificationLevel,
    setMyStatus,
    showMembers,
    unreadCounts,
    viewMode,
  } from "../workspace";
  import Avatar from "./Avatar.svelte";
  import ChannelIcon from "./ChannelIcon.svelte";
  import Chat from "./Chat.svelte";
  import EmojiText from "./EmojiText.svelte";
  import Icon from "./Icon.svelte";
  import Mark from "./Mark.svelte";
  import MemberList from "./MemberList.svelte";

  let filter = "";
  let paletteOpen = false;
  let paletteQuery = "";
  let paletteIndex = 0;
  let paletteInput: HTMLInputElement;
  let draggingChannel: string | null = null;
  let dragOverGroup: string | null = null;
  let dragOverChannel: string | null = null;
  let suppressClick = false;
  let serverTip: { server: Server; top: number; left: number } | null = null;
  let statusMenu = false;

  const statusChoices = [
    { id: "online", label: "Online", color: "var(--green)" },
    { id: "idle", label: "Idle", color: "var(--yellow)" },
    { id: "dnd", label: "Do not disturb", color: "var(--accent)" },
    { id: "invisible", label: "Invisible", color: "#6b7280" },
  ];

  function showServerTip(event: MouseEvent | FocusEvent, server: Server) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    serverTip = { server, top: rect.top + rect.height / 2, left: rect.right + 10 };
  }

  function hideServerTip() {
    serverTip = null;
  }

  async function chooseStatus(next: string) {
    statusMenu = false;
    try {
      await setMyStatus(next);
    } catch {
      // setMyStatus already reported the failure and restored the previous status.
    }
  }

  function serverVerified(server: Server): boolean {
    if (server.verified) return true;
    return (server.flags ?? "").toLowerCase().split(/[\s,]+/).includes("verified");
  }

  $: canManageChannels =
    $serverPermissions !== null && hasPermission($serverPermissions, "ManageChannels");

  $: currentServer =
    $active?.kind === "guild" ? $servers.find((server) => server.id === $active?.serverId) : undefined;

  $: incomingFriendRequests = $friendRequests.incoming.length;

  $: unreadByServer = Object.entries($unreadCounts).reduce(
    (accumulator, [key, count]) => {
      const serverId = key.split(":")[0];
      accumulator[serverId] = (accumulator[serverId] ?? 0) + count;
      return accumulator;
    },
    {} as Record<string, number>,
  );

  $: levelByServer = Object.fromEntries(
    $servers.map((server) => [
      server.id,
      $serverSettings[server.id]?.notification_level ||
        server.default_notification_level ||
        "mentions",
    ]),
  ) as Record<string, string>;

  $: pingByChannel = Object.fromEntries(
    Object.entries($unreadCounts).map(([key, count]) => {
      const serverId = key.split(":")[0];
      const level = levelByServer[serverId] ?? "mentions";
      const mentions = $mentionCounts[key] ?? 0;
      const ping = level === "nothing" ? 0 : level === "all" ? count : mentions;
      return [key, ping];
    }),
  ) as Record<string, number>;

  $: pingByServer = Object.entries(pingByChannel).reduce(
    (accumulator, [key, count]) => {
      const serverId = key.split(":")[0];
      accumulator[serverId] = (accumulator[serverId] ?? 0) + count;
      return accumulator;
    },
    {} as Record<string, number>,
  );

  $: groups = buildGroups($channels, filter);
  $: filteredConversations = $conversations.filter((conversation) =>
    conversationName(conversation).toLowerCase().includes(filter.toLowerCase()),
  );

  $: paletteItems = buildPalette($prefs.keybinds);
  $: filteredPalette = paletteItems
    .filter((item) => item.label.toLowerCase().includes(paletteQuery.toLowerCase()))
    .slice(0, 12);

  function buildGroups(bundle: ChannelBundle | null, query: string) {
    if (!bundle) return [] as Array<{ id: string; name: string; channels: ChannelBundle["channels"] }>;
    const normalized = query.trim().toLowerCase();
    const visible = bundle.channels
      .filter((channel) => channel.type !== "category")
      .filter((channel) => !normalized || channel.name.toLowerCase().includes(normalized))
      .sort((a, b) => (a.position ?? 0) - (b.position ?? 0));
    const categories = [...bundle.categories].sort(
      (a, b) => (a.position ?? 0) - (b.position ?? 0),
    );
    const result = categories.map((category) => ({
      id: category.id,
      name: category.name,
      channels: visible.filter((channel) => channel.category_id === category.id),
    }));
    const loose = visible.filter((channel) => !channel.category_id);
    if (loose.length) result.unshift({ id: "__loose", name: "", channels: loose });
    return result.filter((group) => group.id !== "__loose" || group.channels.length > 0);
  }

  function conversationName(conversation: Conversation): string {
    if (conversation.name) return conversation.name;
    return (
      (conversation.members ?? [])
        .filter((member) => member.user.id !== $me?.id)
        .map((member) => member.user.display_name || member.user.username)
        .join(", ") || "Direct message"
    );
  }

  function conversationAvatar(conversation: Conversation) {
    const other = (conversation.members ?? []).find((member) => member.user.id !== $me?.id);
    return other?.user ?? $me ?? null;
  }

  async function selectServer(serverId: string) {
    const server = $servers.find((item) => item.id === serverId);
    if (!server) return;
    viewMode.set("server");
    filter = "";
    try {
      const bundle = await ensureChannels(server.id);
      const first = bundle.channels
        .filter((channel) => channel.type === "text")
        .sort((a, b) => (a.position ?? 0) - (b.position ?? 0))[0];
      if (first) await openGuild(server, first.id);
      else notify("This server has no text channels yet.");
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    }
  }

  async function focusServer(server: (typeof $servers)[number], action: () => void) {
    if ($active?.kind !== "guild" || $active.serverId !== server.id) {
      await selectServer(server.id);
    }
    action();
  }

  async function selectConversation(conversation: Conversation) {
    viewMode.set("dm");
    filter = "";
    await openConversation(conversation);
  }

  function startChannelDrag(event: DragEvent, channelId: string) {
    if (!canManageChannels) return;
    draggingChannel = channelId;
    dragOverChannel = null;
    suppressClick = true;
    event.dataTransfer?.setData("text/plain", channelId);
    if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
  }

  function enterGroup(event: DragEvent, groupId: string) {
    if (!canManageChannels || !draggingChannel) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
    dragOverGroup = groupId;
    dragOverChannel = null;
  }

  function leaveGroup(groupId: string) {
    if (dragOverGroup === groupId) dragOverGroup = null;
  }

  function dropOnGroup(event: DragEvent, groupId: string) {
    if (!canManageChannels || !currentServer) return;
    event.preventDefault();
    const channelId =
      draggingChannel ?? event.dataTransfer?.getData("text/plain") ?? null;
    dragOverGroup = null;
    dragOverChannel = null;
    draggingChannel = null;
    if (!channelId) return;
    void moveChannelToCategory(
      currentServer,
      channelId,
      groupId === "__loose" ? null : groupId,
    ).catch((error) =>
      notify(error instanceof Error ? error.message : String(error)),
    );
  }

  function enterChannel(event: DragEvent, channelId: string) {
    if (!canManageChannels || !draggingChannel) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
    dragOverChannel = channelId;
    dragOverGroup = null;
  }

  function leaveChannel(channelId: string) {
    if (dragOverChannel === channelId) dragOverChannel = null;
  }

  function dropOnChannel(
    event: DragEvent,
    channelId: string,
    groupId: string,
  ) {
    if (!canManageChannels || !currentServer) return;
    event.preventDefault();
    event.stopPropagation();
    const dragged = draggingChannel ?? event.dataTransfer?.getData("text/plain");
    dragOverChannel = null;
    dragOverGroup = null;
    draggingChannel = null;
    if (!dragged || dragged === channelId) return;
    void moveChannelToCategory(
      currentServer,
      dragged,
      groupId === "__loose" ? null : groupId,
      channelId,
    ).catch((error) =>
      notify(error instanceof Error ? error.message : String(error)),
    );
  }

  function createMenu(event: MouseEvent) {
    if (!currentServer || !canManageChannels) return;
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const server = currentServer;
    openContextMenu(rect.left - 110, rect.bottom + 5, [
      {
        label: "Create channel",
        icon: "hash",
        run: () => openChannelCreate(server.id, server.name),
      },
      {
        label: "Create category",
        icon: "server",
        run: () => openCategoryCreate(server.id, server.name),
      },
    ]);
  }

  function categoryMenu(event: MouseEvent, category: Category) {
    if (!currentServer || !canManageChannels) return;
    const server = currentServer;
    const items: ContextMenuItem[] = [
      {
        label: "Create channel here",
        icon: "plus",
        run: () => openChannelCreate(server.id, server.name, category.id),
      },
      {
        label: "Rename category",
        icon: "pencil",
        run: () => openCategoryEdit(server.id, server.name, category),
      },
      {
        label: "Copy category ID",
        icon: "copy",
        run: () => {
          void navigator.clipboard
            .writeText(category.id)
            .then(() => notify("Category ID copied."))
            .catch(() => notify("Could not copy the category ID."));
        },
      },
      { separator: true, label: "" },
      {
        label: `Delete ${category.name}`,
        icon: "trash",
        danger: true,
        run: () => void deleteCategory(category),
      },
    ];
    openContextMenu(event.clientX, event.clientY, items);
  }

  async function deleteCategory(category: Category) {
    if (!currentServer || !canManageChannels) return;
    const confirmed = await confirmAction({
      title: `Delete ${category.name}?`,
      body: "Channels in this category will not be deleted; they will move out of the category.",
      confirmLabel: "Delete category",
      danger: true,
    });
    if (!confirmed) return;
    try {
      await api.deleteCategory(currentServer.id, category.id);
      await reloadChannels(currentServer);
      notify("Category deleted.");
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    }
  }

  function channelMenu(event: MouseEvent, channel: Channel) {
    if (!currentServer) return;
    const server = currentServer;
    const items: ContextMenuItem[] = [];
    if (canManageChannels) {
      items.push(
        {
          label: "Edit channel",
          icon: "pencil",
          run: () => openChannelEdit(server.id, server.name, channel),
        },
        {
          label: "Delete channel",
          icon: "trash",
          danger: true,
          run: () => void deleteChannel(channel),
        },
        { separator: true, label: "" },
      );
    }
    items.push(
      {
        label: "Copy channel ID",
        icon: "copy",
        run: () => {
          void navigator.clipboard
            .writeText(channel.id)
            .then(() => notify("Channel ID copied."))
            .catch(() => notify("Could not copy the channel ID."));
        },
      },
      {
        label: "Copy channel link",
        icon: "link",
        run: () => {
          const link = `https://flackle.com/app/channels/${server.id}/${channel.id}`;
          void navigator.clipboard
            .writeText(link)
            .then(() => notify("Channel link copied."))
            .catch(() => notify("Could not copy the channel link."));
        },
      },
    );
    openContextMenu(event.clientX, event.clientY, items);
  }

  async function deleteChannel(channel: Channel) {
    if (!currentServer || !canManageChannels) return;
    const confirmed = await confirmAction({
      title: `Delete #${channel.name}?`,
      body: "This cannot be undone. All messages in the channel will be removed.",
      confirmLabel: "Delete channel",
      danger: true,
    });
    if (!confirmed) return;
    try {
      await api.deleteChannel(currentServer.id, channel.id);
      await reloadChannels(currentServer);
      notify("Channel deleted.");
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    }
  }

  async function goHome() {
    viewMode.set("dm");
    filter = "";
    const first = $conversations[0];
    if (first && $active?.kind !== "dm") await openConversation(first);
  }

  function buildPalette(keybinds: Keybinds): PaletteItemLike[] {
    const items: PaletteItemLike[] = [];
    for (const server of $servers) {
      items.push({
        key: `server:${server.id}`,
        label: server.name,
        hint: "Server",
        icon: "home",
        run: () => void selectServer(server.id),
      });
    }
    for (const channel of $channels?.channels ?? []) {
      if (channel.type !== "text") continue;
      items.push({
        key: `channel:${channel.id}`,
        label: `# ${channel.name}`,
        hint: currentServer?.name ?? "Channel",
        icon: "hash",
        run: () => {
          if (currentServer) void openGuild(currentServer, channel.id);
        },
      });
    }
    for (const conversation of $conversations) {
      items.push({
        key: `dm:${conversation.id}`,
        label: conversationName(conversation),
        hint: "Direct message",
        icon: "dm",
        run: () => void openConversation(conversation),
      });
    }
    items.push(
      {
        key: "action:settings",
        label: "Open settings",
        hint: formatKeybind(keybinds.settings),
        icon: "settings",
        run: () => modal.set("settings"),
      },
      {
        key: "action:members",
        label: "Toggle member list",
        hint: formatKeybind(keybinds.toggleMembers),
        icon: "users",
        run: () => showMembers.update((value) => !value),
      },
      {
        key: "action:server-settings",
        label: "Server settings",
        hint: currentServer?.name ?? "Server",
        icon: "server",
        run: () => modal.set("server-settings"),
      },
      {
        key: "action:friends",
        label: "Friends",
        hint: "Add or message friends",
        icon: "users",
        run: () => modal.set("friends"),
      },
      {
        key: "action:add-server",
        label: "Add a server",
        hint: "Join or create",
        icon: "plus",
        run: () => modal.set("add-server"),
      },
      {
        key: "action:logout",
        label: "Log out",
        hint: "Account",
        icon: "alert",
        run: () => modal.set("settings"),
      },
    );
    return items;
  }

  async function openPalette() {
    paletteOpen = true;
    paletteQuery = "";
    paletteIndex = 0;
    await tick();
    paletteInput?.focus();
  }

  function closePalette() {
    paletteOpen = false;
  }

  function runPaletteItem(item: PaletteItemLike) {
    closePalette();
    item.run();
  }

  function onPaletteKey(event: KeyboardEvent) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      paletteIndex = Math.min(paletteIndex + 1, Math.max(filteredPalette.length - 1, 0));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      paletteIndex = Math.max(paletteIndex - 1, 0);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const item = filteredPalette[paletteIndex];
      if (item) runPaletteItem(item);
    } else if (event.key === "Escape") {
      closePalette();
    }
  }

  function onGlobalKey(event: KeyboardEvent) {
    if (matchesKeybind(event, $prefs.keybinds.quickSwitcher)) {
      event.preventDefault();
      void openPalette();
    } else if (matchesKeybind(event, $prefs.keybinds.settings)) {
      event.preventDefault();
      modal.set("settings");
    } else if (matchesKeybind(event, $prefs.keybinds.toggleMembers)) {
      event.preventDefault();
      showMembers.update((value) => !value);
    } else if (matchesKeybind(event, $prefs.keybinds.serverSettings)) {
      event.preventDefault();
      if ($active?.kind === "guild") modal.set("server-settings");
      else notify("Open a server first.");
    } else if (event.key === "Escape") {
      if (paletteOpen) closePalette();
      else modal.set(null);
    }
  }

  onMount(() => {
    if (!get(prefs).startWithMembers) showMembers.set(false);
    window.addEventListener("keydown", onGlobalKey);
    void (async () => {
      if ($active) {
        viewMode.set($active.kind === "dm" ? "dm" : "server");
        return;
      }
      if ($servers.length > 0) {
        await selectServer($servers[0].id);
      } else if ($conversations.length > 0) {
        await selectConversation($conversations[0]);
      }
    })();
    return () => window.removeEventListener("keydown", onGlobalKey);
  });

  function showServerMenu(event: MouseEvent, server: (typeof $servers)[number]) {
    const index = $servers.findIndex((entry) => entry.id === server.id);
    const currentLevel =
      $serverSettings[server.id]?.notification_level ||
      server.default_notification_level ||
      "mentions";
    const muted = currentLevel === "nothing";
    const canInvite =
      $serverPermissions !== null && hasPermission($serverPermissions, "CreateInvite");
    const isOwner = server.owner_id === $me?.id;
    const reorder = (offset: number) => {
      const target = index + offset;
      if (target < 0 || target >= $servers.length) return;
      const list = [...$servers];
      [list[index], list[target]] = [list[target], list[index]];
      servers.set(list);
      void api
        .updateServerPositions(list.map((entry, position) => ({ server_id: entry.id, position })))
        .catch(() => notify("Could not save server order."));
    };
    const items: ContextMenuItem[] = [
      {
        label: "Open server",
        icon: "server",
        run: () => void selectServer(server.id),
      },
      {
        label: "Move up",
        icon: "up",
        disabled: index <= 0,
        run: () => reorder(-1),
      },
      {
        label: "Move down",
        icon: "down",
        disabled: index < 0 || index >= $servers.length - 1,
        run: () => reorder(1),
      },
      { separator: true, label: "" },
      {
        label: "Mark as Read",
        icon: "check",
        run: () => {
          unreadCounts.update((map) =>
            Object.fromEntries(
              Object.entries(map).filter(([key]) => !key.startsWith(`${server.id}:`)),
            ),
          );
          mentionCounts.update((map) =>
            Object.fromEntries(
              Object.entries(map).filter(([key]) => !key.startsWith(`${server.id}:`)),
            ),
          );
          void api.markServerRead(server.id).catch(() => {});
        },
      },
      {
        label: muted ? "Unmute Server" : "Mute Server",
        icon: "bell",
        run: () => {
          void setServerNotificationLevel(server.id, muted ? "mentions" : "nothing")
            .then(() => notify(muted ? `${server.name} unmuted.` : `${server.name} muted.`))
            .catch(() => notify("Could not update notification settings."));
        },
      },
      {
        label: "Notification Settings",
        icon: "sliders",
        run: () => void focusServer(server, () => modal.set("server-settings")),
      },
      {
        label: "Server Info",
        icon: "alert",
        run: () => void focusServer(server, () => modal.set("server-info")),
      },
      {
        label: "Invite People",
        icon: "users",
        disabled: !canInvite,
        run: () => {
          if (!canInvite) return;
          void api
            .createServerInvite(server.id, { force_new: true })
            .then((invite) => {
              const url = invite.url ?? `https://flackle.com/invite/${invite.code}`;
              return navigator.clipboard.writeText(url).then(() => {
                notify("Invite link copied.");
              });
            })
            .catch(() => notify("Could not create an invite."));
        },
      },
      {
        label: "Copy Server ID",
        icon: "hash",
        run: () => {
          void navigator.clipboard
            .writeText(server.id)
            .then(() => notify("Server ID copied."))
            .catch(() => notify("Could not copy server ID."));
        },
      },
      { separator: true, label: "" },
      {
        label: "Leave Server",
        icon: "alert",
        danger: true,
        disabled: isOwner,
        run: () => {
          if (isOwner) {
            notify("Transfer ownership before leaving this server.");
            return;
          }
          void (async () => {
            const confirmed = await confirmAction({
              title: `Leave ${server.name}?`,
              body: "You can join again with an invite.",
              confirmLabel: "Leave server",
              danger: true,
            });
            if (!confirmed) return;
            void api
              .leaveServer(server.id)
              .then(() => {
                servers.update((list) => list.filter((entry) => entry.id !== server.id));
                if ($active?.kind === "guild" && $active.serverId === server.id) {
                  active.set(null);
                  const next = $servers[0];
                  if (next) void selectServer(next.id);
                }
                notify(`Left ${server.name}.`);
              })
              .catch(() => notify("Could not leave this server."));
          })();
        },
      },
    ];
    openContextMenu(event.clientX, event.clientY, items);
  }
</script>

<div class="shell">
  <div class="frame">
    <nav class="rail" aria-label="Servers">
      <button class="home" class:selected={$viewMode === "dm"} aria-label="Direct messages" on:click={goHome}>
        <Mark size={23} />
      </button>
      <div class="rail-sep"></div>
      <div class="server-list">
        {#each $servers as server (server.id)}
          <button
            class="server"
            class:selected={$active?.kind === "guild" && $active.serverId === server.id}
            class:has-icon={!!server.icon_url}
            style={`--sc:${avatarColor(server.name)}`}
            aria-label={server.name}
            on:mouseenter={(event) => showServerTip(event, server)}
            on:mouseleave={hideServerTip}
            on:focus={(event) => showServerTip(event, server)}
            on:blur={hideServerTip}
            on:click={() => {
              hideServerTip();
              void selectServer(server.id);
            }}
            on:contextmenu|preventDefault={(event) => showServerMenu(event, server)}
          >
            {#if server.icon_url}
              <img src={server.icon_url} alt="" loading="lazy" />
            {:else}
              <span class="server-fallback">{initials(server.name)}</span>
            {/if}
            {#if unreadByServer[server.id] && !($active?.kind === "guild" && $active.serverId === server.id)}
              <span class="read-dot" aria-hidden="true"></span>
            {/if}
            {#if pingByServer[server.id]}
              <span class="ping" aria-label={`${pingByServer[server.id]} unread mention${pingByServer[server.id] === 1 ? "" : "s"}`}>
                {pingByServer[server.id] > 99 ? "99+" : pingByServer[server.id]}
              </span>
            {/if}
          </button>
        {/each}
        <button
          class="server add"
          title="Add a server"
          aria-label="Add a server"
          on:click={() => modal.set("add-server")}
        >
          <Icon name="plus" size={18} />
        </button>
      </div>
    </nav>

    {#if serverTip}
      <div
        class="server-tip"
        role="tooltip"
        style={`top:${serverTip.top}px; left:${serverTip.left}px`}
      >
        <div class="tip-name">
          <span>{serverTip.server.name}</span>
          {#if serverVerified(serverTip.server)}
            <span class="tip-badge" title="Verified server" aria-label="Verified server">
              <Icon name="check" size={9} />
            </span>
          {/if}
        </div>
        <div class="tip-stats">
          <span><i class="tip-online"></i>{serverTip.server.approx_presence_count ?? "—"} online</span>
          <span>{serverTip.server.approx_member_count ?? "—"} members</span>
        </div>
      </div>
    {/if}

    <aside class="sidebar">
      {#if $viewMode === "dm"}
        <header class="side-head">
          <div>
            <div class="side-title">Direct messages</div>
          </div>
          <button
            class="icon-btn"
            aria-label="Friends"
            title="Friends"
            on:click={() => modal.set("friends")}
          >
            <Icon name="users" size={16} />
            {#if incomingFriendRequests > 0 && !$prefs.hidePendingCount}
              <span class="friend-count">{incomingFriendRequests > 9 ? "9+" : incomingFriendRequests}</span>
            {/if}
          </button>
        </header>
        <div class="side-search">
          <Icon name="search" size={14} />
          <input bind:value={filter} placeholder="Find a conversation" spellcheck="false" />
        </div>
        <div class="side-scroll">
          {#each filteredConversations as conversation (conversation.id)}
            <button
              class="dm-row"
              class:active={$active?.kind === "dm" && $active.conversationId === conversation.id}
              on:click={() => selectConversation(conversation)}
            >
              <Avatar user={conversationAvatar(conversation)} size={34} />
              <span class="dm-copy">
                <b><EmojiText text={conversationName(conversation)} size={13} /></b>
                <small>{conversation.last_message?.content ?? "No messages yet"}</small>
              </span>
              {#if conversation.unread_count}
                <span class="count">{conversation.unread_count}</span>
              {/if}
            </button>
          {/each}
          {#if filteredConversations.length === 0}
            <p class="side-empty">No conversations match “{filter}”.</p>
          {/if}
        </div>
      {:else}
        <header class="side-head" class:has-banner={!!currentServer?.banner_url}>
          {#if currentServer?.banner_url}
            <img class="side-banner" src={currentServer.banner_url} alt="" />
            <span class="side-banner-shade"></span>
          {/if}
          <div class="server-heading">
            {#if currentServer}
              <div class="side-title"><EmojiText text={currentServer.name} size={14} /></div>
              <div class="side-sub">
                {currentServer.approx_member_count
                  ? `${currentServer.approx_member_count} members`
                  : "Community server"}
              </div>
            {:else}
              <div class="side-title">Channels</div>
            {/if}
          </div>
          {#if currentServer}
            {#if canManageChannels}
              <button
                class="icon-btn"
                aria-label="Create channel or category"
                title="Create channel or category"
                on:click={createMenu}
              >
                <Icon name="plus" size={17} />
              </button>
            {/if}
            <button
              class="icon-btn"
              aria-label="Server settings"
              title="Server settings"
              on:click={() => modal.set("server-settings")}
            >
              <Icon name="settings" size={16} />
            </button>
          {/if}
        </header>
        <div class="side-search">
          <Icon name="search" size={14} />
          <input bind:value={filter} placeholder="Find a channel" spellcheck="false" />
        </div>
        <div class="side-scroll">
          {#each groups as group (group.id)}
            {#if group.name}
              {@const category = ($channels?.categories ?? []).find((item) => item.id === group.id)}
              <div
                class="side-label-row"
                role="group"
                aria-label={group.name}
                class:drag-over={dragOverGroup === group.id}
                on:dragenter={(event) => enterGroup(event, group.id)}
                on:dragover={(event) => enterGroup(event, group.id)}
                on:dragleave={() => leaveGroup(group.id)}
                on:drop={(event) => dropOnGroup(event, group.id)}
                on:contextmenu|preventDefault={(event) => {
                  if (category) categoryMenu(event, category);
                }}
              >
                <span class="side-label-text"><EmojiText text={group.name} size={9.5} /></span>
                {#if canManageChannels && currentServer}
                  <button
                    class="label-add"
                    title={`Create channel in ${group.name}`}
                    aria-label={`Create channel in ${group.name}`}
                    on:click={(event) => {
                      event.stopPropagation();
                      if (currentServer) {
                        openChannelCreate(currentServer.id, currentServer.name, group.id);
                      }
                    }}
                  >
                    <Icon name="plus" size={12} />
                  </button>
                {/if}
              </div>
            {/if}
            {#each group.channels as channel (channel.id)}
              {@const unreadCount = currentServer ? ($unreadCounts[`${currentServer.id}:${channel.id}`] ?? 0) : 0}
              {@const pingCount = currentServer ? (pingByChannel[`${currentServer.id}:${channel.id}`] ?? 0) : 0}
              <button
                class="channel"
                class:active={$active?.kind === "guild" && $active.channelId === channel.id}
                class:voice={channel.type === "voice"}
                class:drag-over={dragOverChannel === channel.id}
                draggable={canManageChannels}
                on:dragstart={(event) => startChannelDrag(event, channel.id)}
                on:dragend={() => {
                  draggingChannel = null;
                  dragOverGroup = null;
                  dragOverChannel = null;
                  setTimeout(() => (suppressClick = false), 100);
                }}
                on:dragenter={(event) => enterChannel(event, channel.id)}
                on:dragover={(event) => enterChannel(event, channel.id)}
                on:dragleave={() => leaveChannel(channel.id)}
                on:drop={(event) => dropOnChannel(event, channel.id, group.id)}
                on:contextmenu|preventDefault={(event) => channelMenu(event, channel)}
                on:click={() => {
                  if (suppressClick) {
                    suppressClick = false;
                    return;
                  }
                  if (channel.type === "voice") {
                    notify("Voice audio is not supported by the Flackle API yet.");
                  } else if (currentServer) {
                    void openGuild(currentServer, channel.id);
                  }
                }}
              >
                <ChannelIcon
                  name={channel.icon}
                  color={channel.icon_color}
                  size={15}
                  fallback={channel.type === "voice" ? "speaker" : "hash"}
                />
                {#if unreadCount > 0}
                  <span class="unread-dot" aria-hidden="true"></span>
                {/if}
                <span class="channel-name"><EmojiText text={channel.name} size={13.5} /></span>
                {#if pingCount > 0}
                  <span class="count">{pingCount > 99 ? "99+" : pingCount}</span>
                {/if}
              </button>
            {/each}
          {/each}
          {#if groups.length === 0}
            <p class="side-empty">No channels match “{filter}”.</p>
          {/if}
        </div>
      {/if}

      <div class="user-panel">
        <div class="user-self-wrap">
          <button
            class="user-self"
            aria-label="Your profile"
            on:click={(event) => {
              if (!$me) return;
              openProfile(
                $me,
                $members.find((member) => member.user.id === $me?.id),
                event.currentTarget as HTMLElement | null,
              );
            }}
          >
            <div class="user-avatar">
              <Avatar user={$me} size={34} status={false} />
            </div>
            <div class="user-copy">
              <b>{$me?.display_name || $me?.username}</b>
            </div>
          </button>
          <button
            class="status-trigger"
            class:online={$me?.status === "online"}
            class:idle={$me?.status === "idle"}
            class:dnd={$me?.status === "dnd"}
            title="Set status"
            aria-label="Set status"
            aria-expanded={statusMenu}
            on:click={() => (statusMenu = !statusMenu)}
          >
            <i></i>
          </button>
        </div>
        <button class="icon-btn" aria-label="Settings" on:click={() => modal.set("settings")}>
          <Icon name="settings" size={17} />
        </button>

        {#if statusMenu}
          <button
            class="status-backdrop"
            aria-label="Close status menu"
            on:click={() => (statusMenu = false)}
          ></button>
          <div class="status-menu" role="menu" aria-label="Set status">
            <span class="status-title">Set status</span>
            {#each statusChoices as choice}
              <button
                role="menuitem"
                class:active={$me?.status === choice.id}
                on:click={() => chooseStatus(choice.id)}
              >
                <i style={`background:${choice.color}`}></i>
                <span>{choice.label}</span>
                {#if $me?.status === choice.id}<Icon name="check" size={12} />{/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </aside>

    <main class="content">
      {#if $active}
        <Chat />
      {:else}
        <div class="empty">
          <div class="empty-mark"><Mark size={54} /></div>
          <h2>Pick a channel</h2>
          <p>Your servers and conversations are on the left.</p>
          <div class="empty-keys">
            <span><b class="kbd">⌘</b><b class="kbd">K</b> jump anywhere</span>
          </div>
        </div>
      {/if}
    </main>

    {#if $showMembers && $active?.kind === "guild"}
      <MemberList />
    {/if}
  </div>

  {#if paletteOpen}
    <div class="palette-backdrop" role="presentation" on:click={closePalette}>
      <div class="palette" role="dialog" aria-label="Quick switcher" on:click|stopPropagation>
        <div class="palette-input">
          <Icon name="search" size={17} />
          <input
            bind:this={paletteInput}
            bind:value={paletteQuery}
            on:input={() => (paletteIndex = 0)}
            on:keydown={onPaletteKey}
            placeholder="Jump to a server, channel, or conversation…"
            spellcheck="false"
          />
          <span class="kbd">esc</span>
        </div>
        <div class="palette-list">
          {#each filteredPalette as item, index (item.key)}
            <button
              class="palette-item"
              class:selected={index === paletteIndex}
              on:mouseenter={() => (paletteIndex = index)}
              on:click={() => runPaletteItem(item)}
            >
              <span class="p-icon"><Icon name={item.icon} size={16} /></span>
              <span class="p-label">{item.label}</span>
              <span class="p-hint">{item.hint}</span>
            </button>
          {/each}
          {#if filteredPalette.length === 0}
            <p class="palette-empty">Nothing found.</p>
          {/if}
        </div>
        <footer class="palette-foot">
          <span>↑↓ navigate</span>
          <span>↵ open</span>
          <span>{formatKeybind($prefs.keybinds.quickSwitcher)} anywhere</span>
        </footer>
      </div>
    </div>
  {/if}
</div>

<style>
  .shell {
    height: 100%;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg);
  }
  .frame {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .rail {
    width: 68px;
    flex: none;
    background: var(--rail);
    border-right: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 10px 0;
    gap: 6px;
  }
  .home {
    width: 44px;
    height: 44px;
    border-radius: 14px;
    border: 1px solid transparent;
    background: var(--panel-2);
    color: var(--accent);
    display: grid;
    place-items: center;
    position: relative;
    flex: none;
  }
  .home:hover {
    border-radius: 12px;
    background: var(--panel-3);
  }
  .home.selected {
    border-radius: 12px;
    background: var(--accent-dim);
    border-color: rgba(var(--accent-rgb), 0.26);
  }
  .home.selected::before {
    content: "";
    position: absolute;
    left: -13px;
    top: 11px;
    width: 3px;
    height: 22px;
    border-radius: 0 3px 3px 0;
    background: var(--accent);
  }
  .rail-sep {
    width: 26px;
    height: 1px;
    background: var(--line-strong);
    margin: 3px 0;
    flex: none;
  }
  .server-list {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 8px 0;
  }
  .server {
    width: 44px;
    height: 44px;
    border-radius: 14px;
    border: 1px solid var(--line);
    background: var(--panel-2);
    color: var(--text-dim);
    display: grid;
    place-items: center;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: -0.02em;
    position: relative;
    flex: none;
    overflow: visible;
    transition: border-radius 120ms ease, background 120ms ease;
  }
  .server span {
    color: color-mix(in srgb, var(--sc) 85%, white);
  }
  .server.has-icon {
    border-color: transparent;
  }
  .server.add {
    border-style: dashed;
    color: var(--muted);
    background: transparent;
  }
  .server.add:hover {
    color: var(--green);
    border-color: rgba(52, 211, 153, 0.4);
    background: rgba(52, 211, 153, 0.06);
  }
  .server img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    border-radius: inherit;
  }
  .server:hover {
    border-radius: 12px;
    background: color-mix(in srgb, var(--sc) 14%, var(--panel-2));
  }
  .server.selected {
    border-radius: 12px;
    border-color: color-mix(in srgb, var(--sc) 45%, transparent);
    background: color-mix(in srgb, var(--sc) 17%, var(--panel-2));
  }
  .server.selected::before {
    content: "";
    position: absolute;
    left: -13px;
    top: 11px;
    width: 3px;
    height: 22px;
    border-radius: 0 3px 3px 0;
    background: var(--accent);
  }
  .server .read-dot {
    position: absolute;
    left: -13px;
    top: 50%;
    transform: translateY(-50%);
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--text-dim);
  }
  .server.selected .read-dot {
    display: none;
  }
  .server .ping {
    position: absolute;
    top: -4px;
    right: -4px;
    min-width: 19px;
    height: 19px;
    padding: 0 5px;
    border-radius: 99px;
    background: var(--accent);
    color: white;
    font-size: 10px;
    font-weight: 750;
    line-height: 1;
    display: grid;
    place-items: center;
    border: 2px solid var(--rail);
    z-index: 2;
  }
  .sidebar {
    width: 252px;
    flex: none;
    background: var(--sidebar);
    border-right: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .side-head {
    height: 58px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 0 10px 0 15px;
    border-bottom: 1px solid var(--line);
    position: relative;
    overflow: hidden;
  }
  .side-banner {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0.62;
    pointer-events: none;
  }
  .side-banner-shade {
    position: absolute;
    inset: 0;
    background: rgba(8, 8, 11, 0.45);
    pointer-events: none;
  }
  .server-heading,
  .side-head > .icon-btn {
    position: relative;
    z-index: 1;
  }
  .server-heading {
    flex: 1;
    min-width: 0;
  }
  .side-head > .icon-btn {
    position: relative;
    z-index: 1;
    flex: none;
  }
  .side-title {
    font-family: "Satoshi", "Manrope", system-ui, sans-serif;
    font-size: 14px;
    font-weight: 650;
    letter-spacing: -0.02em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }
  .side-sub {
    font-size: 10.5px;
    color: var(--muted);
    margin-top: 1px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }
  .side-search {
    flex: none;
    margin: 9px 10px 2px;
    height: 32px;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 9px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--input);
    color: var(--muted);
  }
  .side-search:focus-within {
    border-color: var(--line-strong);
  }
  .side-search input {
    border: 0;
    background: transparent;
    outline: none;
    width: 100%;
    font-size: 12px;
    color: var(--text);
  }
  .side-search input::placeholder {
    color: #5a6070;
  }
  .side-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 5px 8px 12px;
    min-height: 0;
  }
  .side-label-row {
    display: flex;
    align-items: center;
    gap: 4px;
    min-height: 24px;
    padding: 8px 6px 3px 8px;
    margin-top: 6px;
    border-radius: 5px;
    color: #666d7d;
  }
  .side-label-row:hover {
    background: color-mix(in srgb, var(--hover) 55%, transparent);
  }
  .side-label-row.drag-over {
    color: var(--accent);
    background: var(--accent-dim);
  }
  .side-label-text {
    flex: 1;
    min-width: 0;
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .label-add {
    width: 20px;
    height: 20px;
    flex: none;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: #666d7d;
    display: grid;
    place-items: center;
    opacity: 0;
  }
  .side-label-row:hover .label-add,
  .label-add:focus-visible {
    opacity: 1;
  }
  .label-add:hover {
    background: var(--hover);
    color: var(--text);
  }
  .dm-row,
  .channel {
    width: 100%;
    border: 0;
    background: transparent;
    color: var(--muted);
    display: flex;
    align-items: center;
    gap: 9px;
    border-radius: 7px;
    padding: 0 8px;
    text-align: left;
  }
  .dm-row:hover,
  .channel:hover {
    background: var(--hover);
    color: var(--text);
  }
  .channel {
    height: 33px;
    font-size: 13px;
    margin: 1px 0;
  }
  .channel[draggable="true"] {
    cursor: grab;
  }
  .channel .unread-dot {
    width: 5px;
    height: 5px;
    flex: none;
    border-radius: 50%;
    background: var(--text-dim);
  }
  .channel.active .unread-dot {
    display: none;
  }
  .channel.drag-over {
    box-shadow: inset 0 2px 0 var(--accent);
  }
  .channel-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .channel.active,
  .dm-row.active {
    background: var(--active);
    color: var(--text);
  }
  .channel.active {
    font-weight: 550;
  }
  .channel.voice {
    opacity: 0.62;
  }
  .dm-row {
    height: 54px;
    gap: 10px;
    margin: 1px 0;
  }
  .dm-copy {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
    gap: 1px;
  }
  .dm-copy b {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dm-copy small {
    font-size: 10.5px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .count {
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 9px;
    background: var(--accent);
    color: white;
    font-size: 9.5px;
    font-weight: 700;
    line-height: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
  }
  .friend-count {
    position: absolute;
    top: -4px;
    right: -4px;
    min-width: 14px;
    height: 14px;
    padding: 0 3px;
    border-radius: 99px;
    background: var(--accent);
    color: white;
    border: 2px solid var(--sidebar);
    font-size: 7.5px;
    font-weight: 750;
    line-height: 1;
    display: grid;
    place-items: center;
  }
  .side-empty {
    font-size: 11.5px;
    color: var(--muted);
    padding: 14px 9px;
    line-height: 1.5;
  }
  .user-panel {
    height: 58px;
    flex: none;
    border-top: 1px solid var(--line);
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 8px 0 12px;
    position: relative;
  }
  .user-self-wrap {
    position: relative;
    display: flex;
    flex: 1;
    min-width: 0;
  }
  .user-copy {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
    gap: 1px;
  }
  .user-self {
    display: flex;
    align-items: center;
    gap: 9px;
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    padding: 3px 4px;
    margin-left: -4px;
    border-radius: 8px;
    text-align: left;
  }
  .user-self:hover {
    background: var(--hover);
  }
  .user-copy b {
    font-size: 12.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .user-avatar {
    width: 34px;
    height: 34px;
    flex: none;
  }
  .status-trigger {
    position: absolute;
    left: 20px;
    bottom: 1px;
    width: 14px;
    height: 14px;
    padding: 0;
    border-radius: 50%;
    border: 2px solid var(--sidebar);
    background: var(--panel-2);
    display: grid;
    place-items: center;
    z-index: 1;
  }
  .status-trigger i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #6b7280;
  }
  .status-trigger.online i {
    background: var(--green);
  }
  .status-trigger.idle i {
    background: var(--yellow);
  }
  .status-trigger.dnd i {
    background: var(--accent);
  }
  .status-trigger:hover {
    border-color: var(--line-strong);
    background: var(--panel-3);
  }
  .status-backdrop {
    position: fixed;
    inset: 0;
    border: 0;
    background: transparent;
    z-index: 30;
  }
  .status-menu {
    position: absolute;
    left: 10px;
    bottom: calc(100% + 7px);
    width: 188px;
    padding: 5px;
    border: 1px solid var(--line-strong);
    border-radius: 9px;
    background: var(--panel-2);
    box-shadow: 0 14px 34px rgba(0, 0, 0, 0.42);
    z-index: 31;
  }
  .status-title {
    display: block;
    padding: 5px 8px 6px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .status-menu button {
    width: 100%;
    height: 31px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--text-dim);
    font-size: 12px;
  }
  .status-menu button > span {
    flex: 1;
    text-align: left;
  }
  .status-menu button:hover,
  .status-menu button.active {
    background: var(--hover);
    color: var(--text);
  }
  .status-menu button > i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex: none;
  }
  .server-tip {
    position: fixed;
    z-index: 40;
    transform: translateY(-50%);
    min-width: 168px;
    max-width: 248px;
    padding: 9px 11px;
    border: 1px solid var(--line-strong);
    border-radius: 8px;
    background: var(--panel-2);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.4);
    pointer-events: none;
  }
  .tip-name {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12.5px;
    font-weight: 650;
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .tip-badge {
    width: 14px;
    height: 14px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: rgba(var(--accent-rgb), 0.16);
    color: #ff8f8f;
  }
  .tip-stats {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 4px;
    font-size: 10.5px;
    color: var(--muted);
  }
  .tip-stats span {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .tip-online {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--green);
  }
  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    background: var(--chat);
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    gap: 4px;
  }
  .empty-mark {
    color: var(--accent);
    opacity: 0.9;
    filter: drop-shadow(0 10px 40px rgba(var(--accent-rgb), 0.22));
    margin-bottom: 16px;
  }
  .empty h2 {
    font-size: 19px;
    letter-spacing: -0.03em;
    color: var(--text);
    margin: 0;
  }
  .empty p {
    font-size: 12.5px;
    margin: 0;
  }
  .empty-keys {
    margin-top: 22px;
    display: flex;
    gap: 8px;
    font-size: 11px;
  }
  .empty-keys span {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
  }
  .palette-backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    background: rgba(0, 0, 0, 0.52);
    backdrop-filter: blur(7px);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 11vh;
  }
  .palette {
    width: min(580px, calc(100vw - 48px));
    background: var(--card);
    border: 1px solid var(--line-strong);
    border-radius: 15px;
    box-shadow: var(--shadow);
    overflow: hidden;
    animation: pop 140ms ease-out;
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-5px) scale(0.99);
    }
  }
  .palette-input {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 14px;
    height: 52px;
    border-bottom: 1px solid var(--line);
    color: var(--muted);
  }
  .palette-input input {
    flex: 1;
    border: 0;
    outline: none;
    background: transparent;
    font-size: 14px;
    color: var(--text);
  }
  .palette-input input::placeholder {
    color: #5a6070;
  }
  .palette-list {
    max-height: 354px;
    overflow-y: auto;
    padding: 6px;
  }
  .palette-item {
    width: 100%;
    height: 44px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 9px;
    color: var(--text-dim);
    text-align: left;
  }
  .palette-item.selected {
    background: var(--active);
    color: var(--text);
  }
  .p-icon {
    width: 27px;
    height: 27px;
    border-radius: 7px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    display: grid;
    place-items: center;
    color: var(--muted);
    flex: none;
  }
  .p-label {
    flex: 1;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .p-hint {
    font-size: 10px;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .palette-empty {
    text-align: center;
    color: var(--muted);
    font-size: 12px;
    padding: 28px;
  }
  .palette-foot {
    height: 34px;
    border-top: 1px solid var(--line);
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 14px;
    font-size: 9.5px;
    color: var(--muted);
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }
</style>
