<script lang="ts">
  import * as api from "../../api";
  import { confirmAction } from "../../dialog";
  import { hasPermission } from "../../permissions";
  import type { Server, ServerInvite } from "../../types";
  import { channels, notify, serverPermissions } from "../../workspace";
  import Avatar from "../Avatar.svelte";
  import Icon from "../Icon.svelte";

  export let server: Server;

  let invites: ServerInvite[] = [];
  let loading = true;
  let creating = false;
  let busy: string | null = null;
  let error = "";
  let lastServerId = "";

  $: canManage =
    $serverPermissions !== null && hasPermission($serverPermissions, "ManageServer");
  $: canInvite =
    $serverPermissions !== null && hasPermission($serverPermissions, "CreateInvite");
  $: channelNames = new Map(($channels?.channels ?? []).map((channel) => [channel.id, channel.name]));

  $: if (server.id !== lastServerId) {
    lastServerId = server.id;
    void load();
  }

  async function load() {
    loading = true;
    error = "";
    try {
      invites = await api.fetchServerInvites(server.id);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
    }
  }

  async function create() {
    if (!canInvite || creating) return;
    creating = true;
    try {
      const invite = await api.createServerInvite(server.id, { max_uses: 0 });
      await load();
      const url = invite.url ?? `https://flackle.com/invite/${invite.code}`;
      await navigator.clipboard.writeText(url).catch(() => {});
      notify("Invite created and copied.");
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      creating = false;
    }
  }

  async function revoke(invite: ServerInvite) {
    if (!canManage) return;
    const confirmed = await confirmAction({
      title: `Revoke invite ${invite.code}?`,
      body: "Anyone using this link will no longer be able to join with it.",
      confirmLabel: "Revoke invite",
      danger: true,
    });
    if (!confirmed) return;
    busy = invite.code;
    try {
      await api.deleteServerInvite(server.id, invite.code);
      invites = invites.filter((entry) => entry.code !== invite.code);
      notify("Invite revoked.");
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      busy = null;
    }
  }

  function copy(invite: ServerInvite) {
    const url = invite.url ?? `https://flackle.com/invite/${invite.code}`;
    void navigator.clipboard
      .writeText(url)
      .then(() => notify("Invite link copied."))
      .catch(() => notify("Could not copy the invite link."));
  }

  function creatorName(invite: ServerInvite) {
    return invite.creator_name || invite.creator_username || "Unknown";
  }

  function uses(invite: ServerInvite) {
    if (invite.max_uses && invite.max_uses > 0) {
      return `${invite.uses ?? 0} / ${invite.max_uses}`;
    }
    return `${invite.uses ?? 0} / Unlimited`;
  }

  function expires(invite: ServerInvite) {
    if (!invite.expires_at) return "Never";
    return new Intl.DateTimeFormat(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    }).format(new Date(invite.expires_at));
  }

  function channelName(invite: ServerInvite) {
    return invite.channel_id ? channelNames.get(invite.channel_id) ?? invite.channel_id : "Server";
  }
</script>

<div class="invites">
  <div class="head">
    <div>
      <h3>Invites</h3>
      <p>Share a link to let people join {server.name}.</p>
    </div>
    <button class="primary" disabled={!canInvite || creating} on:click={create}>
      <Icon name="plus" size={14} />
      {creating ? "Creating…" : "Create invite"}
    </button>
  </div>

  {#if error}
    <p class="denied">{error}</p>
  {:else if loading}
    <p class="empty">Loading invites…</p>
  {:else}
    <div class="table">
      <div class="table-head">
        <span>Invite</span>
        <span>Created by</span>
        <span>Uses</span>
        <span>Expires</span>
        <span>Actions</span>
      </div>
      {#each invites as invite (invite.code)}
        <div class="row">
          <button class="invite-code" on:click={() => copy(invite)} title="Copy invite link">
            <b>{invite.code}</b>
            <small>#{channelName(invite)}</small>
          </button>
          <div class="creator">
            <Avatar
              user={{
                username: creatorName(invite),
                display_name: invite.creator_name,
                avatar_url: invite.creator_avatar_url,
              }}
              size={26}
              status={false}
            />
            <span>{creatorName(invite)}</span>
          </div>
          <span class="muted">{uses(invite)}</span>
          <span class="muted">{expires(invite)}</span>
          <div class="actions">
            <button class="icon-btn" title="Copy link" aria-label="Copy invite link" on:click={() => copy(invite)}>
              <Icon name="copy" size={14} />
            </button>
            <button
              class="icon-btn danger"
              title="Revoke invite"
              aria-label={`Revoke invite ${invite.code}`}
              disabled={!canManage || busy === invite.code}
              on:click={() => revoke(invite)}
            >
              <Icon name="trash" size={14} />
            </button>
          </div>
        </div>
      {/each}
      {#if invites.length === 0}
        <div class="empty-state">
          <Icon name="link" size={24} />
          <b>No active invites</b>
          <span>{canInvite ? "Create one to invite people to this server." : "You do not have permission to create invites."}</span>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .invites {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 16px 18px 18px;
  }
  .head {
    flex: none;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 14px;
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
  .primary {
    height: 33px;
    flex: none;
    border: 0;
    border-radius: 8px;
    background: var(--accent);
    color: white;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
    font-size: 10.5px;
    font-weight: 650;
  }
  .primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 86%, white);
  }
  .primary:disabled {
    opacity: 0.45;
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
    grid-template-columns: minmax(130px, 1.3fr) minmax(120px, 1fr) minmax(80px, 0.8fr) minmax(120px, 1.1fr) 68px;
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
    min-height: 52px;
    border-bottom: 1px solid var(--line);
  }
  .row:last-of-type {
    border-bottom: 0;
  }
  .row:hover {
    background: color-mix(in srgb, var(--panel-2) 65%, transparent);
  }
  .invite-code {
    border: 0;
    background: transparent;
    color: var(--text);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 0;
    text-align: left;
    min-width: 0;
  }
  .invite-code b {
    font-family: "JetBrains Mono", ui-monospace, monospace;
    font-size: 11.5px;
    letter-spacing: 0.02em;
  }
  .invite-code small {
    margin-top: 2px;
    color: var(--muted);
    font-size: 9px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }
  .invite-code:hover b {
    color: var(--accent);
  }
  .creator {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    color: var(--text-dim);
    font-size: 10.5px;
  }
  .creator span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .muted {
    color: var(--muted);
    font-size: 10px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 2px;
  }
  .icon-btn {
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    display: grid;
    place-items: center;
  }
  .icon-btn:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  .icon-btn.danger:hover:not(:disabled) {
    background: rgba(var(--accent-rgb), 0.12);
    color: #ff8a8a;
  }
  .icon-btn:disabled {
    opacity: 0.3;
  }
  .empty-state {
    min-height: 150px;
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
    font-size: 10px;
    max-width: 260px;
    line-height: 1.5;
  }
  .empty,
  .denied {
    padding: 14px;
    margin: 0;
    color: var(--muted);
    font-size: 10.5px;
  }
  .denied {
    color: #d68a8a;
  }
</style>
