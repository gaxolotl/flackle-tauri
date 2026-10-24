<script lang="ts">
  import * as api from "../../api";
  import { confirmAction } from "../../dialog";
  import { hasPermission } from "../../permissions";
  import type { Member, Server, ServerBan } from "../../types";
  import { notify, serverPermissions } from "../../workspace";
  import Avatar from "../Avatar.svelte";
  import Icon from "../Icon.svelte";

  export let server: Server;

  let bans: ServerBan[] = [];
  let userInput = "";
  let reason = "";
  let loading = true;
  let banning = false;
  let busy: string | null = null;
  let error = "";
  let lastServerId = "";
  let memberCache: Member[] | null = null;

  $: canBan =
    $serverPermissions !== null && hasPermission($serverPermissions, "BanMembers");

  $: if (server.id !== lastServerId) {
    lastServerId = server.id;
    memberCache = null;
    void load();
  }

  async function load() {
    loading = true;
    error = "";
    try {
      bans = await api.fetchBans(server.id);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
    }
  }

  async function resolveUser(value: string): Promise<string> {
    const needle = value.trim();
    if (/^\d{15,}$/.test(needle)) return needle;
    if (!needle) throw new Error("Enter a username or user ID.");
    if (!memberCache) memberCache = await api.fetchMembers(server.id);
    const lower = needle.toLowerCase().replace(/^@/, "");
    const match = memberCache.find((member) => {
      const user = member.user;
      return (
        user.id === needle ||
        user.username.toLowerCase() === lower ||
        (user.display_name ?? "").toLowerCase() === lower
      );
    });
    if (!match) throw new Error("No member matches that username.");
    return match.user.id;
  }

  async function submitBan() {
    if (!canBan || banning) return;
    banning = true;
    try {
      const userId = await resolveUser(userInput);
      const confirmed = await confirmAction({
        title: "Ban this user?",
        body: `${userInput.trim()} will be removed and cannot rejoin until unbanned.`,
        confirmLabel: "Ban user",
        danger: true,
      });
      if (!confirmed) return;
      await api.banMember(server.id, userId, reason.trim() || undefined);
      userInput = "";
      reason = "";
      await load();
      notify("User banned.");
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      banning = false;
    }
  }

  async function unban(ban: ServerBan) {
    if (!canBan) return;
    const label = ban.user?.display_name || ban.user?.username || ban.user_id;
    const confirmed = await confirmAction({
      title: `Unban ${label}?`,
      body: "They will be able to join this server again.",
      confirmLabel: "Unban user",
    });
    if (!confirmed) return;
    busy = ban.user_id;
    try {
      await api.unbanMember(server.id, ban.user_id);
      bans = bans.filter((entry) => entry.user_id !== ban.user_id);
      notify(`${label} was unbanned.`);
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      busy = null;
    }
  }

  function banName(ban: ServerBan) {
    return ban.user?.display_name || ban.user?.username || ban.user_id;
  }

  function bannedAt(ban: ServerBan) {
    if (!ban.created_at) return "Unknown";
    return new Intl.DateTimeFormat(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    }).format(new Date(ban.created_at));
  }
</script>

<div class="bans">
  <div class="head">
    <div>
      <h3>Bans</h3>
      <p>Banned users cannot rejoin until they are unbanned.</p>
    </div>
  </div>

  {#if canBan}
    <div class="ban-form">
      <label class="field">
        <span>Username or user ID</span>
        <input bind:value={userInput} placeholder="Username or user ID to ban" spellcheck="false" />
      </label>
      <label class="field reason-field">
        <span>Reason</span>
        <input bind:value={reason} placeholder="Reason for the ban" maxlength="512" />
      </label>
      <button
        class="danger-btn"
        disabled={!userInput.trim() || banning}
        on:click={submitBan}
      >
        {banning ? "Banning…" : "Ban user"}
      </button>
    </div>
  {:else}
    <p class="denied">You do not have permission to ban members.</p>
  {/if}

  {#if error}
    <p class="denied">{error}</p>
  {:else if loading}
    <p class="empty">Loading bans…</p>
  {:else}
    <div class="table">
      <div class="table-head">
        <span>User</span>
        <span>Reason</span>
        <span>Banned</span>
        <span>Actions</span>
      </div>
      {#each bans as ban (ban.user_id)}
        <div class="row">
          <div class="user">
            <Avatar
              user={ban.user ?? { username: ban.user_id }}
              size={28}
              status={false}
            />
            <span class="user-copy">
              <b>{banName(ban)}</b>
              <small>{ban.user ? `@${ban.user.username}` : ban.user_id}</small>
            </span>
          </div>
          <span class="reason">{ban.reason || "No reason provided"}</span>
          <span class="muted">{bannedAt(ban)}</span>
          <div class="actions">
            <button
              class="unban"
              disabled={!canBan || busy === ban.user_id}
              on:click={() => unban(ban)}
            >
              Unban
            </button>
          </div>
        </div>
      {/each}
      {#if bans.length === 0}
        <div class="empty-state">
          <Icon name="ban" size={24} />
          <b>No banned users</b>
          <span>Bans will appear here with the reason and date.</span>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .bans {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 16px 18px 18px;
  }
  .head {
    flex: none;
    margin-bottom: 13px;
  }
  h3 {
    margin: 0;
    font-size: 16px;
    letter-spacing: -0.03em;
  }
  .head p {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 10.5px;
  }
  .ban-form {
    flex: none;
    display: flex;
    align-items: flex-end;
    gap: 7px;
    margin-bottom: 13px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    flex: 1;
    min-width: 0;
  }
  .reason-field {
    flex: 1.3;
  }
  .field > span {
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .field input {
    height: 34px;
    border: 1px solid var(--line-strong);
    border-radius: 8px;
    background: var(--input);
    color: var(--text);
    font-size: 11.5px;
    padding: 0 9px;
    outline: none;
  }
  .field input:focus {
    border-color: rgba(var(--accent-rgb), 0.4);
    box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.06);
  }
  .danger-btn {
    height: 34px;
    flex: none;
    border: 1px solid rgba(var(--accent-rgb), 0.35);
    border-radius: 8px;
    background: rgba(var(--accent-rgb), 0.12);
    color: #ff9a9a;
    padding: 0 12px;
    font-size: 10.5px;
    font-weight: 650;
  }
  .danger-btn:hover:not(:disabled) {
    background: rgba(var(--accent-rgb), 0.2);
  }
  .danger-btn:disabled {
    opacity: 0.35;
  }
  .table {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    border: 1px solid var(--line);
    border-radius: 10px;
  }
  .table-head,
  .row {
    display: grid;
    grid-template-columns: minmax(160px, 1.2fr) minmax(160px, 1.8fr) minmax(105px, 0.9fr) 68px;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
  }
  .table-head {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--panel-2);
    border-bottom: 1px solid var(--line);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .row {
    min-height: 50px;
    border-bottom: 1px solid var(--line);
  }
  .row:last-of-type {
    border-bottom: 0;
  }
  .row:hover {
    background: color-mix(in srgb, var(--panel-2) 65%, transparent);
  }
  .user {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .user-copy {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: 1.2;
  }
  .user-copy b {
    font-size: 11.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .user-copy small {
    margin-top: 2px;
    color: var(--muted);
    font-size: 9px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .reason {
    color: var(--text-dim);
    font-size: 10px;
    line-height: 1.4;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .muted {
    color: var(--muted);
    font-size: 9.5px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
  }
  .unban {
    height: 25px;
    border: 1px solid var(--line-strong);
    border-radius: 6px;
    background: var(--panel-2);
    color: var(--text-dim);
    padding: 0 9px;
    font-size: 9.5px;
    font-weight: 600;
  }
  .unban:hover:not(:disabled) {
    background: var(--panel-3);
    color: var(--text);
  }
  .unban:disabled {
    opacity: 0.35;
  }
  .empty-state {
    min-height: 140px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    color: var(--muted);
    text-align: center;
    padding: 20px;
  }
  .empty-state b {
    margin-top: 4px;
    color: var(--text-dim);
    font-size: 12px;
  }
  .empty-state span {
    max-width: 250px;
    font-size: 10px;
    line-height: 1.5;
  }
  .empty,
  .denied {
    padding: 12px 2px;
    margin: 0;
    color: var(--muted);
    font-size: 10.5px;
  }
  .denied {
    color: #d68a8a;
  }
</style>
