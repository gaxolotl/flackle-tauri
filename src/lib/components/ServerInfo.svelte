<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "../api";
  import { openProfile } from "../profile";
  import { showUserMenu } from "../user-menu";
  import type { Member, User } from "../types";
  import {
    active,
    applyServerUpdate,
    channels,
    me,
    memberTotal,
    members,
    modal,
    presenceStatuses,
    serverRoles,
    servers,
  } from "../workspace";
  import Avatar from "./Avatar.svelte";
  import Icon from "./Icon.svelte";

  type Tab = "info" | "friends" | "blocked";

  let tab: Tab = "info";
  let friends: User[] = [];
  let blocked: User[] = [];

  $: server =
    $active?.kind === "guild"
      ? $servers.find((entry) => entry.id === $active?.serverId) ?? null
      : null;
  $: myMember = $members.find((member) => member.user.id === $me?.id) ?? null;
  $: owner =
    $members.find((member) => member.is_owner) ??
    $members.find((member) => member.user.id === server?.owner_id) ??
    null;
  $: online = $members.filter((member) =>
    ["online", "idle", "dnd"].includes(
      $presenceStatuses[member.user.id] ?? member.presence ?? member.user.status ?? "offline",
    ),
  ).length;
  $: total = $memberTotal || $members.length;
  $: channelCount = ($channels?.channels ?? []).filter(
    (channel) =>
      channel.type === "text" || channel.type === "voice" || channel.type === "stats",
  ).length;
  $: roleCount = $serverRoles.filter((role) => !role.is_everyone).length;
  $: friendIds = new Set(friends.map((user) => user.id));
  $: blockedIds = new Set(blocked.map((user) => user.id));
  $: friendsHere = $members.filter((member) => friendIds.has(member.user.id));
  $: blockedHere = $members.filter((member) => blockedIds.has(member.user.id));

  onMount(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") modal.set(null);
    };
    window.addEventListener("keydown", onKey);
    if (server) {
      void api
        .fetchServer(server.id)
        .then((value) => {
          applyServerUpdate(value);
        })
        .catch(() => {});
    }
    void Promise.all([api.fetchFriends(), api.fetchBlocked()])
      .then(([friendList, blockedList]) => {
        friends = friendList;
        blocked = blockedList;
      })
      .catch(() => {});
    return () => window.removeEventListener("keydown", onKey);
  });

  function formatDate(value?: string | null): string {
    if (!value) return "—";
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return "—";
    return date.toLocaleString("en-GB", {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function ownerName(member: Member | null): string {
    if (!member) return "—";
    return member.user.display_name || member.user.username;
  }

  function displayName(member: Member): string {
    return member.nickname || member.user.display_name || member.user.username;
  }
</script>

<div class="backdrop" role="presentation" on:click={() => modal.set(null)}>
  <div class="modal" role="dialog" aria-label="Server info" tabindex="-1" on:click|stopPropagation>
    <header>
      <span class="server-mark">
        {#if server?.icon_url}
          <img src={server.icon_url} alt="" />
        {:else}
          <Icon name="server" size={18} />
        {/if}
      </span>
      <div class="head-copy">
        <h2>{server?.name ?? "Server"}</h2>
        <p>Server info</p>
      </div>
      <button class="icon-btn" aria-label="Close" on:click={() => modal.set(null)}>
        <Icon name="close" size={18} />
      </button>
    </header>

    <div class="banner">
      {#if server?.banner_url}
        <img src={server.banner_url} alt="" />
      {:else}
        <div class="banner-fallback">
          <span>{server?.name ?? "Server"}</span>
          {#if server?.description}<small>{server.description}</small>{/if}
        </div>
      {/if}
    </div>

    <div class="tabs">
      <button class:active={tab === "info"} on:click={() => (tab = "info")}>Server Info</button>
      <button class:active={tab === "friends"} on:click={() => (tab = "friends")}>
        Friends ({friendsHere.length})
      </button>
      <button class:active={tab === "blocked"} on:click={() => (tab = "blocked")}>
        Blocked Users ({blockedHere.length})
      </button>
    </div>

    {#if tab === "info"}
      <div class="info">
        <div class="fact owner-fact">
          <span>Server Owner</span>
          {#if owner}
            <button
              class="owner"
              on:click={(event) => openProfile(owner.user, owner, event.currentTarget as HTMLElement | null)}
              on:contextmenu|preventDefault={(event) => showUserMenu(event, owner.user, owner)}
            >
              <Avatar user={owner.user} size={20} status={false} />
              <b>{ownerName(owner)}</b>
            </button>
          {:else}
            <b>—</b>
          {/if}
        </div>

        <div class="fact">
          <span>Created At</span>
          <b>{formatDate(server?.created_at)}</b>
        </div>

        <div class="fact">
          <span>Joined At</span>
          <b>{formatDate(myMember?.joined_at)}</b>
        </div>

        <div class="fact">
          <span>Members</span>
          <b>{total} <small>({online} online)</small></b>
        </div>

        <div class="fact">
          <span>Channels</span>
          <b>{channelCount}</b>
        </div>

        <div class="fact">
          <span>Roles</span>
          <b>{roleCount}</b>
        </div>
      </div>

      {#if server?.description}
        <p class="description">{server.description}</p>
      {/if}
    {:else if tab === "friends"}
      <div class="people">
        {#each friendsHere as member (member.user.id)}
          <button
            class="person"
            on:click={(event) => openProfile(member.user, member, event.currentTarget as HTMLElement | null)}
            on:contextmenu|preventDefault={(event) => showUserMenu(event, member.user, member)}
          >
            <Avatar user={member.user} size={28} status={false} />
            <span>{displayName(member)}</span>
          </button>
        {/each}
        {#if friendsHere.length === 0}<p class="empty">None of your friends are in this server.</p>{/if}
      </div>
    {:else}
      <div class="people">
        {#each blockedHere as member (member.user.id)}
          <button
            class="person"
            on:click={(event) => openProfile(member.user, member, event.currentTarget as HTMLElement | null)}
            on:contextmenu|preventDefault={(event) => showUserMenu(event, member.user, member)}
          >
            <Avatar user={member.user} size={28} status={false} />
            <span>{displayName(member)}</span>
          </button>
        {/each}
        {#if blockedHere.length === 0}<p class="empty">No blocked users are in this server.</p>{/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 127;
    background: rgba(0, 0, 0, 0.58);
    backdrop-filter: blur(8px);
    display: grid;
    place-items: center;
    padding: 24px;
  }
  .modal {
    width: min(570px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    overflow-y: auto;
    background: var(--card);
    border: 1px solid var(--line-strong);
    border-radius: 15px;
    box-shadow: var(--shadow);
    animation: rise 150ms ease-out;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(7px);
    }
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 13px 12px 13px 16px;
    border-bottom: 1px solid var(--line);
  }
  .server-mark {
    width: 34px;
    height: 34px;
    border-radius: 9px;
    overflow: hidden;
    background: var(--panel-2);
    border: 1px solid var(--line);
    display: grid;
    place-items: center;
    color: var(--muted);
    flex: none;
  }
  .server-mark img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .head-copy {
    flex: 1;
  }
  h2 {
    margin: 0;
    font-size: 14px;
    letter-spacing: -0.02em;
  }
  .head-copy p {
    margin: 1px 0 0;
    font-size: 10px;
    color: var(--muted);
  }
  .banner {
    margin: 14px;
    height: 168px;
    border-radius: 10px;
    overflow: hidden;
    border: 1px solid var(--line);
    background: var(--panel-2);
  }
  .banner img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .banner-fallback {
    height: 100%;
    padding: 20px;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 6px;
    background: var(--panel-3);
  }
  .banner-fallback span {
    font-family: "Satoshi", "Manrope", system-ui, sans-serif;
    font-size: 24px;
    font-weight: 650;
    letter-spacing: -0.035em;
  }
  .banner-fallback small {
    font-size: 11px;
    color: var(--text-dim);
    max-width: 400px;
  }
  .tabs {
    display: flex;
    gap: 16px;
    padding: 0 16px;
    border-bottom: 1px solid var(--line);
    overflow-x: auto;
  }
  .tabs button {
    height: 35px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
    padding: 0;
    white-space: nowrap;
    flex: none;
  }
  .tabs button.active {
    color: var(--text);
    border-bottom-color: var(--accent);
  }
  .info {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 18px 20px;
    padding: 18px 16px;
  }
  .fact {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }
  .fact > span {
    font-size: 9.5px;
    font-weight: 650;
    letter-spacing: 0.02em;
    color: var(--text-dim);
  }
  .fact > b {
    font-size: 11px;
    font-weight: 500;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fact small {
    font-size: 10px;
    color: var(--muted);
    font-weight: 400;
  }
  .owner {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    border: 0;
    background: transparent;
    padding: 0;
    text-align: left;
    color: inherit;
  }
  .owner:hover b {
    text-decoration: underline;
  }
  .owner b {
    font-size: 11px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .description {
    margin: 0;
    padding: 0 16px 18px;
    color: var(--muted);
    font-size: 10.5px;
    line-height: 1.55;
  }
  .people {
    padding: 10px 16px 16px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 120px;
  }
  .person {
    display: flex;
    align-items: center;
    gap: 9px;
    height: 38px;
    padding: 0 6px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    font-size: 11.5px;
    color: var(--text-dim);
    text-align: left;
  }
  .person:hover {
    background: var(--hover);
    color: var(--text);
  }
  .empty {
    margin: 20px 0;
    color: var(--muted);
    font-size: 11px;
  }
</style>
