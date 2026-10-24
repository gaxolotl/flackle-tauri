<script lang="ts">
  import * as api from "../../api";
  import { hasPermission } from "../../permissions";
  import type { AuditChange, AuditLogEntry, Server } from "../../types";
  import { notify, serverPermissions } from "../../workspace";
  import Avatar from "../Avatar.svelte";
  import Icon from "../Icon.svelte";

  export let server: Server;

  const actionLabels: Record<string, string> = {
    member_roles_update: "Member roles updated",
    member_role_add: "Role added to member",
    member_role_remove: "Role removed from member",
    member_ban_add: "Member banned",
    member_ban_remove: "Member unbanned",
    member_kick: "Member kicked",
    member_timeout: "Member timed out",
    member_timeout_remove: "Member timeout removed",
    member_join: "Member joined",
    member_leave: "Member left",
    role_create: "Role created",
    role_update: "Role updated",
    role_delete: "Role deleted",
    channel_create: "Channel created",
    channel_update: "Channel updated",
    channel_delete: "Channel deleted",
    category_create: "Category created",
    category_update: "Category updated",
    category_delete: "Category deleted",
    invite_create: "Invite created",
    invite_delete: "Invite revoked",
    server_update: "Server updated",
    message_delete: "Message deleted",
    message_bulk_delete: "Messages bulk deleted",
  };

  let entries: AuditLogEntry[] = [];
  let loading = true;
  let loadingMore = false;
  let error = "";
  let query = "";
  let hasMore = false;
  let lastServerId = "";

  $: canView =
    $serverPermissions !== null && hasPermission($serverPermissions, "ViewAuditLog");
  $: filtered = entries.filter((entry) => {
    const needle = query.trim().toLowerCase();
    if (!needle) return true;
    return [
      actionLabel(entry.action_type),
      actorName(entry),
      targetName(entry),
      entry.target_id,
      entry.action_type,
    ]
      .filter(Boolean)
      .some((value) => String(value).toLowerCase().includes(needle));
  });

  $: if (server.id !== lastServerId) {
    lastServerId = server.id;
    entries = [];
    hasMore = false;
    void load();
  }

  async function load(more = false) {
    if (more) loadingMore = true;
    else loading = true;
    error = "";
    try {
      const before = more ? entries[entries.length - 1]?.id ?? null : null;
      if (!more) entries = [];
      const page = await api.fetchAuditLog(server.id, 50, before);
      entries = more ? [...entries, ...(page.entries ?? [])] : page.entries ?? [];
      hasMore = Boolean(page.has_more);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
      if (more) notify(error);
    } finally {
      loading = false;
      loadingMore = false;
    }
  }

  function actionLabel(type: string) {
    return (
      actionLabels[type] ||
      type
        .replace(/_/g, " ")
        .replace(/\b\w/g, (letter) => letter.toUpperCase())
    );
  }

  function actorName(entry: AuditLogEntry) {
    return entry.actor?.display_name || entry.actor?.username || "Unknown";
  }

  function targetName(entry: AuditLogEntry) {
    return (
      entry.target_user?.display_name ||
      entry.target_user?.username ||
      (entry.options?.target_name as string | undefined) ||
      entry.target_id ||
      entry.target_kind ||
      ""
    );
  }

  function formatValue(value: unknown): string {
    if (value == null) return "None";
    if (Array.isArray(value)) return value.map(formatValue).join(", ") || "None";
    if (typeof value === "object") {
      return Object.entries(value as Record<string, unknown>)
        .map(([key, item]) => `${key}: ${formatValue(item)}`)
        .join(", ");
    }
    return String(value);
  }

  function changeText(change: AuditChange): string {
    const key = String(change.key ?? "Change");
    if (key === "$remove") return `Removed ${formatValue(change.new)}`;
    if (key === "$add") return `Added ${formatValue(change.new)}`;
    const label = key
      .replace(/([a-z])([A-Z])/g, "$1 $2")
      .replace(/_/g, " ")
      .replace(/\b\w/g, (letter) => letter.toUpperCase());
    if (change.old === undefined) return `${label}: ${formatValue(change.new)}`;
    return `${label}: ${formatValue(change.old)} → ${formatValue(change.new)}`;
  }

  function changes(entry: AuditLogEntry) {
    return (entry.changes ?? []).map(changeText).join(" · ");
  }

  function timeLabel(value?: string | null) {
    if (!value) return "Unknown time";
    return new Intl.DateTimeFormat(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    }).format(new Date(value));
  }
</script>

<div class="audit">
  <div class="head">
    <div>
      <h3>Audit Log</h3>
      <p>Recent moderation and server changes.</p>
    </div>
    <label class="search">
      <Icon name="search" size={14} />
      <input bind:value={query} placeholder="Search audit log" spellcheck="false" />
    </label>
  </div>

  {#if !canView}
    <p class="denied">You do not have permission to view the audit log.</p>
  {:else if error}
    <p class="denied">{error}</p>
  {:else if loading}
    <p class="empty">Loading audit log…</p>
  {:else}
    <div class="list">
      {#each filtered as entry (entry.id)}
        <div class="entry">
          <Avatar
            user={entry.actor ?? { username: "?" }}
            size={28}
            status={false}
          />
          <div class="entry-copy">
            <div class="line">
              <b>{actorName(entry)}</b>
              <span>{actionLabel(entry.action_type)}</span>
              {#if targetName(entry)}<em>{targetName(entry)}</em>{/if}
            </div>
            {#if changes(entry)}
              <p>{changes(entry)}</p>
            {/if}
          </div>
          <time>{timeLabel(entry.created_at)}</time>
        </div>
      {/each}
      {#if filtered.length === 0}
        <div class="empty-state">
          <Icon name="clock" size={24} />
          <b>No audit entries</b>
          <span>{query ? "No entries match your search." : "Server changes will appear here."}</span>
        </div>
      {/if}
      {#if hasMore && !query}
        <button class="load-more" disabled={loadingMore} on:click={() => load(true)}>
          {loadingMore ? "Loading…" : "Load older entries"}
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .audit {
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
  .search {
    width: 210px;
    height: 32px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 9px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--input);
    color: var(--muted);
  }
  .search:focus-within {
    border-color: var(--line-strong);
  }
  .search input {
    width: 100%;
    border: 0;
    background: transparent;
    outline: none;
    color: var(--text);
    font-size: 11.5px;
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    border: 1px solid var(--line);
    border-radius: 10px;
  }
  .entry {
    display: flex;
    align-items: flex-start;
    gap: 9px;
    padding: 9px 11px;
    border-bottom: 1px solid var(--line);
  }
  .entry:last-of-type {
    border-bottom: 0;
  }
  .entry:hover {
    background: color-mix(in srgb, var(--panel-2) 65%, transparent);
  }
  .entry-copy {
    flex: 1;
    min-width: 0;
  }
  .line {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 4px;
    line-height: 1.35;
    color: var(--muted);
    font-size: 10.5px;
  }
  .line b {
    color: var(--text);
    font-weight: 600;
  }
  .line em {
    color: var(--text-dim);
    font-style: normal;
    font-weight: 500;
  }
  .entry-copy p {
    margin: 3px 0 0;
    color: var(--muted);
    font-size: 9.5px;
    line-height: 1.4;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  time {
    flex: none;
    margin-left: 8px;
    color: #5f6675;
    font-size: 8.5px;
    white-space: nowrap;
    padding-top: 1px;
  }
  .load-more {
    display: block;
    width: 100%;
    height: 34px;
    border: 0;
    border-top: 1px solid var(--line);
    background: var(--panel-2);
    color: var(--muted);
    font-size: 10px;
    font-weight: 600;
  }
  .load-more:hover:not(:disabled) {
    background: var(--panel-3);
    color: var(--text);
  }
  .load-more:disabled {
    opacity: 0.5;
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
    padding: 14px 2px;
    margin: 0;
    color: var(--muted);
    font-size: 10.5px;
  }
  .denied {
    color: #d68a8a;
  }
</style>
