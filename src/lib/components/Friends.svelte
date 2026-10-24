<script lang="ts">
  import { onMount } from "svelte";
  import { openProfile } from "../profile";
  import type { FriendRequest, User } from "../types";
  import {
    acceptFriendRequest,
    blockedUsers,
    cancelFriendRequest,
    friendRequests,
    friends,
    loadSocial,
    messageUser,
    modal,
    notify,
    rejectFriendRequest,
    removeFriend,
    sendFriendRequest,
    unblockUser,
  } from "../workspace";
  import Avatar from "./Avatar.svelte";
  import Icon from "./Icon.svelte";

  type Tab = "all" | "incoming" | "outgoing" | "blocked";

  let tab: Tab = "all";
  let username = "";
  let adding = false;
  let busy: string | null = null;

  $: incoming = $friendRequests.incoming;
  $: outgoing = $friendRequests.outgoing;

  onMount(() => {
    void loadSocial();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") modal.set(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  function name(user?: User | null) {
    return user?.display_name || user?.username || "Unknown user";
  }

  async function add() {
    const value = username.trim().replace(/^@/, "");
    if (!value || adding) return;
    adding = true;
    try {
      await sendFriendRequest(value);
      username = "";
      notify("Friend request sent.");
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    } finally {
      adding = false;
    }
  }

  async function accept(request: FriendRequest) {
    if (!request.sender || busy) return;
    busy = request.id;
    try {
      await acceptFriendRequest(request.id);
      notify("Friend request accepted.");
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    } finally {
      busy = null;
    }
  }

  async function reject(request: FriendRequest) {
    if (busy) return;
    busy = request.id;
    try {
      await rejectFriendRequest(request.id);
      notify("Friend request rejected.");
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    } finally {
      busy = null;
    }
  }

  async function cancel(request: FriendRequest) {
    if (busy) return;
    busy = request.id;
    try {
      await cancelFriendRequest(request.id);
      notify("Friend request cancelled.");
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    } finally {
      busy = null;
    }
  }

  async function remove(user: User) {
    if (busy) return;
    busy = user.id;
    try {
      await removeFriend(user.id);
      notify("Friend removed.");
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    } finally {
      busy = null;
    }
  }

  async function unblock(user: User) {
    if (busy) return;
    busy = user.id;
    try {
      await unblockUser(user.id);
      notify("User unblocked.");
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    } finally {
      busy = null;
    }
  }

  async function message(user: User) {
    try {
      await messageUser(user.id);
      modal.set(null);
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    }
  }
</script>

<div class="backdrop" role="presentation" on:click={() => modal.set(null)}>
  <div
    class="modal"
    role="dialog"
    aria-label="Friends"
    tabindex="-1"
    on:click|stopPropagation
    on:keydown|stopPropagation
  >
    <header>
      <div>
        <h2>Friends</h2>
        <p>{$friends.length} friend{$friends.length === 1 ? "" : "s"} · {incoming.length} incoming</p>
      </div>
      <button class="icon-btn" aria-label="Close friends" on:click={() => modal.set(null)}>
        <Icon name="close" size={18} />
      </button>
    </header>

    <div class="add-row">
      <input
        bind:value={username}
        placeholder="Add a friend by username"
        spellcheck="false"
        on:keydown={(event) => {
          if (event.key === "Enter") void add();
        }}
      />
      <button class="primary" disabled={!username.trim() || adding} on:click={add}>
        <Icon name="userPlus" size={14} />
        {adding ? "Sending…" : "Add Friend"}
      </button>
    </div>

    <div class="tabs">
      <button class:active={tab === "all"} on:click={() => (tab = "all")}>
        All <span>{$friends.length}</span>
      </button>
      <button class:active={tab === "incoming"} on:click={() => (tab = "incoming")}>
        Incoming <span>{incoming.length}</span>
      </button>
      <button class:active={tab === "outgoing"} on:click={() => (tab = "outgoing")}>
        Outgoing <span>{outgoing.length}</span>
      </button>
      <button class:active={tab === "blocked"} on:click={() => (tab = "blocked")}>
        Blocked <span>{$blockedUsers.length}</span>
      </button>
    </div>

    <div class="list">
      {#if tab === "all"}
        {#each $friends as friend (friend.id)}
          <div class="row">
            <button class="person" on:click={(event) => openProfile(friend, null, event.currentTarget as HTMLElement | null)}>
              <Avatar user={friend} size={32} />
              <span class="person-copy"><b>{name(friend)}</b><small>@{friend.username}</small></span>
            </button>
            <div class="row-actions">
              <button class="secondary" on:click={() => message(friend)}>
                <Icon name="mail" size={13} /> Message
              </button>
              <button class="danger-outline" disabled={busy === friend.id} on:click={() => remove(friend)}>
                Remove
              </button>
            </div>
          </div>
        {/each}
        {#if $friends.length === 0}<p class="empty">No friends yet. Add someone by username.</p>{/if}
      {:else if tab === "incoming"}
        {#each incoming as request (request.id)}
          {#if request.sender}
            {@const sender = request.sender}
            <div class="row">
              <button class="person" on:click={(event) => openProfile(sender, null, event.currentTarget as HTMLElement | null)}>
                <Avatar user={sender} size={32} />
                <span class="person-copy"><b>{name(sender)}</b><small>@{sender.username}</small></span>
              </button>
              <div class="row-actions">
                <button class="primary small" disabled={busy === request.id} on:click={() => accept(request)}>
                  Accept
                </button>
                <button class="secondary small" disabled={busy === request.id} on:click={() => reject(request)}>
                  Reject
                </button>
              </div>
            </div>
          {/if}
        {/each}
        {#if incoming.length === 0}<p class="empty">No incoming friend requests.</p>{/if}
      {:else if tab === "outgoing"}
        {#each outgoing as request (request.id)}
          {#if request.receiver}
            {@const receiver = request.receiver}
            <div class="row">
              <button class="person" on:click={(event) => openProfile(receiver, null, event.currentTarget as HTMLElement | null)}>
                <Avatar user={receiver} size={32} />
                <span class="person-copy"><b>{name(receiver)}</b><small>@{receiver.username}</small></span>
              </button>
              <div class="row-actions">
                <button class="secondary small" disabled={busy === request.id} on:click={() => cancel(request)}>
                  Cancel request
                </button>
              </div>
            </div>
          {/if}
        {/each}
        {#if outgoing.length === 0}<p class="empty">No outgoing friend requests.</p>{/if}
      {:else}
        {#each $blockedUsers as user (user.id)}
          <div class="row">
            <button class="person" on:click={(event) => openProfile(user, null, event.currentTarget as HTMLElement | null)}>
              <Avatar user={user} size={32} />
              <span class="person-copy"><b>{name(user)}</b><small>@{user.username}</small></span>
            </button>
            <div class="row-actions">
              <button class="secondary small" disabled={busy === user.id} on:click={() => unblock(user)}>
                Unblock
              </button>
            </div>
          </div>
        {/each}
        {#if $blockedUsers.length === 0}<p class="empty">No blocked users.</p>{/if}
      {/if}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 126;
    background: rgba(0, 0, 0, 0.56);
    backdrop-filter: blur(8px);
    display: grid;
    place-items: center;
    padding: 26px;
  }
  .modal {
    width: min(620px, calc(100vw - 52px));
    height: min(610px, calc(100vh - 52px));
    background: var(--card);
    border: 1px solid var(--line-strong);
    border-radius: 16px;
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: rise 150ms ease-out;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(7px) scale(0.995);
    }
  }
  header {
    flex: none;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 14px;
    padding: 15px 14px 13px 19px;
    border-bottom: 1px solid var(--line);
  }
  h2 {
    margin: 0;
    font-size: 17px;
    letter-spacing: -0.03em;
  }
  header p {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 10.5px;
  }
  .add-row {
    flex: none;
    display: flex;
    gap: 7px;
    padding: 11px 13px;
    border-bottom: 1px solid var(--line);
    background: var(--panel-2);
  }
  .add-row input {
    flex: 1;
    min-width: 0;
    height: 34px;
    border: 1px solid var(--line-strong);
    border-radius: 8px;
    background: var(--input);
    color: var(--text);
    padding: 0 10px;
    font-size: 11.5px;
    outline: none;
  }
  .add-row input:focus {
    border-color: rgba(var(--accent-rgb), 0.4);
    box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.06);
  }
  .primary,
  .secondary,
  .danger-outline {
    height: 31px;
    border-radius: 7px;
    padding: 0 11px;
    font-size: 10px;
    font-weight: 650;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    flex: none;
  }
  .primary {
    border: 0;
    background: var(--accent);
    color: white;
  }
  .primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 86%, white);
  }
  .secondary {
    border: 1px solid var(--line);
    background: var(--panel-3);
    color: var(--text-dim);
  }
  .secondary:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  .danger-outline {
    border: 1px solid rgba(var(--accent-rgb), 0.25);
    background: transparent;
    color: #ff9a9a;
  }
  .danger-outline:hover:not(:disabled) {
    background: rgba(var(--accent-rgb), 0.12);
  }
  .primary:disabled,
  .secondary:disabled,
  .danger-outline:disabled {
    opacity: 0.38;
  }
  .primary.small,
  .secondary.small {
    height: 27px;
    font-size: 9px;
    padding: 0 8px;
  }
  .tabs {
    flex: none;
    display: flex;
    gap: 3px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--line);
  }
  .tabs button {
    height: 28px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    font-size: 9.5px;
    padding: 0 9px;
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .tabs button span {
    font-size: 8px;
    border: 1px solid var(--line);
    border-radius: 99px;
    padding: 0 4px;
    color: var(--muted);
  }
  .tabs button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .tabs button.active {
    background: var(--active);
    color: var(--text);
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 51px;
    padding: 7px 13px;
    border-bottom: 1px solid var(--line);
  }
  .row:hover {
    background: color-mix(in srgb, var(--panel-2) 60%, transparent);
  }
  .person {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    display: flex;
    align-items: center;
    gap: 9px;
    text-align: left;
    padding: 0;
  }
  .person-copy {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: 1.2;
  }
  .person-copy b {
    font-size: 11.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .person-copy small {
    margin-top: 2px;
    font-size: 9px;
    color: var(--muted);
  }
  .row-actions {
    display: flex;
    gap: 5px;
    flex: none;
  }
  .empty {
    margin: 0;
    padding: 18px 14px;
    color: var(--muted);
    font-size: 10.5px;
    text-align: center;
  }
</style>
