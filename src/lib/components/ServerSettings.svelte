<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "../api";
  import { hasPermission } from "../permissions";
  import type { ServerUpdate } from "../types";
  import {
    active,
    applyServerUpdate,
    loadServerPermissions,
    modal,
    notify,
    serverPermissions,
    servers,
  } from "../workspace";
  import Icon from "./Icon.svelte";
  import AuditLogTab from "./server-settings/AuditLogTab.svelte";
  import BansTab from "./server-settings/BansTab.svelte";
  import InvitesTab from "./server-settings/InvitesTab.svelte";
  import LiftsTab from "./server-settings/LiftsTab.svelte";
  import MembersTab from "./server-settings/MembersTab.svelte";
  import RolesTab from "./server-settings/RolesTab.svelte";

  type Tab = "overview" | "roles" | "members" | "invites" | "bans" | "lifts" | "audit";

  const tabs: Array<{ key: Tab; label: string; icon: string }> = [
    { key: "overview", label: "Overview", icon: "server" },
    { key: "roles", label: "Roles", icon: "shield" },
    { key: "members", label: "Members", icon: "users" },
    { key: "invites", label: "Invites", icon: "link" },
    { key: "bans", label: "Bans", icon: "ban" },
    { key: "lifts", label: "Lifts & Storage", icon: "gift" },
    { key: "audit", label: "Audit Log", icon: "clock" },
  ];

  let tab: Tab = "overview";
  let name = "";
  let description = "";
  let notificationLevel = "mentions";
  let iconUrl: string | null = null;
  let saving = false;
  let uploading = false;
  let lastServerId = "";

  $: server =
    $active?.kind === "guild"
      ? $servers.find((item) => item.id === $active.serverId) ?? null
      : null;
  $: canManageServer =
    $serverPermissions !== null && hasPermission($serverPermissions, "ManageServer");
  $: if (server && server.id !== lastServerId) {
    lastServerId = server.id;
    name = server.name;
    description = server.description ?? "";
    notificationLevel = server.default_notification_level || "mentions";
    iconUrl = server.icon_url ?? null;
    tab = "overview";
  }

  onMount(() => {
    if (server) {
      const target = server;
      void loadServerPermissions(target.id);
      void api
        .fetchServer(target.id)
        .then((detail) => {
          applyServerUpdate(detail);
          name = detail.name;
          description = detail.description ?? "";
          notificationLevel = detail.default_notification_level || "mentions";
          iconUrl = detail.icon_url ?? null;
        })
        .catch(() => {});
    }
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") modal.set(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  async function saveOverview() {
    if (!server || !canManageServer || saving) return;
    if (!name.trim()) {
      notify("A server needs a name.");
      return;
    }
    saving = true;
    try {
      const patch: ServerUpdate = {
        name: name.trim(),
        description: description.trim(),
        default_notification_level: notificationLevel as ServerUpdate["default_notification_level"],
      };
      const updated = await api.updateServer(server.id, patch);
      applyServerUpdate({ ...server, ...updated, name: name.trim(), description: description.trim() });
      notify("Server updated.");
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    } finally {
      saving = false;
    }
  }

  async function changeIcon() {
    if (!server || !canManageServer || uploading) return;
    const path = await api.pickFile();
    if (!path) return;
    uploading = true;
    try {
      const url = await api.uploadServerIcon(server.id, path);
      iconUrl = url;
      applyServerUpdate({ ...server, icon_url: url });
      notify("Server icon updated.");
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    } finally {
      uploading = false;
    }
  }

  function createdLabel(value?: string | null) {
    if (!value) return "—";
    return new Intl.DateTimeFormat(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
    }).format(new Date(value));
  }

  function close() {
    modal.set(null);
  }
</script>

<div class="backdrop" role="presentation" on:click={close}>
  <div class="modal" role="dialog" aria-label="Server settings" tabindex="-1" on:click|stopPropagation>
    <header class="modal-head">
      <div class="title">
        <span class="server-mark">
          {#if iconUrl}
            <img src={iconUrl} alt="" />
          {:else}
            <Icon name="server" size={18} />
          {/if}
        </span>
        <div>
          <h2>Server settings</h2>
          <p>{server?.name ?? "No server selected"}</p>
        </div>
      </div>
      <button class="close-btn" aria-label="Close server settings" on:click={close}>
        <Icon name="close" size={18} />
      </button>
    </header>

    <div class="modal-body">
      <nav class="tabs" aria-label="Server settings sections">
        {#each tabs as item (item.key)}
          <button class:active={tab === item.key} on:click={() => (tab = item.key)}>
            <Icon name={item.icon} size={15} />
            {item.label}
          </button>
        {/each}
        <div class="tabs-spacer"></div>
        <div class="tabs-foot">
          <Icon name="lock" size={12} />
          <span>{canManageServer ? "Manager access" : "Read-only access"}</span>
        </div>
      </nav>

      <div class="content" class:flush={tab !== "overview"}>
        {#if !server}
          <p class="empty">Open a server first.</p>
        {:else if tab === "overview"}
          <div class="overview">
            <div class="icon-row">
              <button
                class="icon-preview"
                class:disabled={!canManageServer}
                title={canManageServer ? "Change server icon" : "You cannot change this icon"}
                on:click={changeIcon}
              >
                {#if iconUrl}
                  <img src={iconUrl} alt="" />
                {:else}
                  <Icon name="server" size={26} />
                {/if}
                <span class="icon-overlay">
                  {#if uploading}<span class="spin"></span>{:else}<Icon name="image" size={16} />{/if}
                </span>
              </button>
              <div class="icon-copy">
                <b>Server icon</b>
                <p>Square images work best. PNG, JPG, or WebP.</p>
                {#if !canManageServer}<p class="denied">You do not have permission to edit this server.</p>{/if}
              </div>
            </div>

            <label class="field">
              <span>Server name</span>
              <input bind:value={name} disabled={!canManageServer} maxlength="100" />
            </label>

            <label class="field">
              <span>Description</span>
              <textarea
                bind:value={description}
                disabled={!canManageServer}
                rows="3"
                maxlength="500"
                placeholder="What is this server about?"
              ></textarea>
            </label>

            <label class="field">
              <span>Default notification level for new members</span>
              <select bind:value={notificationLevel} disabled={!canManageServer}>
                <option value="all">All messages</option>
                <option value="mentions">Mentions only</option>
                <option value="nothing">Nothing</option>
              </select>
              <small class="field-note">
                Server-wide default. Each member can still choose their own level for this server.
              </small>
            </label>

            <div class="facts">
              <div>
                <span>Server ID</span>
                <b>{server.id}</b>
              </div>
              <div>
                <span>Created</span>
                <b>{createdLabel(server.created_at)}</b>
              </div>
              <div>
                <span>Members</span>
                <b>{server.approx_member_count ?? "—"}</b>
              </div>
            </div>

            <div class="footer-row">
              <span>{canManageServer ? "Changes apply to everyone in this server." : "Read-only view."}</span>
              <button class="primary" disabled={!canManageServer || saving} on:click={saveOverview}>
                {saving ? "Saving…" : "Save changes"}
              </button>
            </div>
          </div>
        {:else if tab === "roles"}
          <RolesTab {server} />
        {:else if tab === "members"}
          <MembersTab {server} />
        {:else if tab === "invites"}
          <InvitesTab {server} />
        {:else if tab === "bans"}
          <BansTab {server} />
        {:else if tab === "lifts"}
          <LiftsTab {server} />
        {:else if tab === "audit"}
          <AuditLogTab {server} />
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 125;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(8px);
    display: grid;
    place-items: center;
    padding: 22px;
  }
  .modal {
    width: min(1010px, calc(100vw - 44px));
    height: min(700px, calc(100vh - 44px));
    background: var(--card);
    border: 1px solid var(--line-strong);
    border-radius: 16px;
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: rise 160ms ease-out;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.995);
    }
  }
  .modal-head {
    height: 70px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 14px 0 18px;
    border-bottom: 1px solid var(--line);
  }
  .title {
    display: flex;
    align-items: center;
    gap: 11px;
    min-width: 0;
  }
  .server-mark {
    width: 40px;
    height: 40px;
    border-radius: 11px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    display: grid;
    place-items: center;
    color: var(--muted);
    overflow: hidden;
    flex: none;
  }
  .server-mark img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  h2 {
    margin: 0;
    font-size: 17px;
    letter-spacing: -0.03em;
  }
  .title p {
    margin: 1px 0 0;
    font-size: 11px;
    color: var(--muted);
  }
  .close-btn {
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--muted);
    display: grid;
    place-items: center;
  }
  .close-btn:hover {
    background: var(--hover);
    color: var(--text);
  }
  .modal-body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .tabs {
    width: 188px;
    flex: none;
    border-right: 1px solid var(--line);
    background: var(--sidebar);
    padding: 11px 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .tabs button {
    height: 34px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 10px;
    font-size: 11.5px;
    text-align: left;
    flex: none;
  }
  .tabs button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .tabs button.active {
    background: var(--active);
    color: var(--text);
  }
  .tabs-spacer {
    flex: 1;
  }
  .tabs-foot {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 10px 2px;
    margin-top: 6px;
    border-top: 1px solid var(--line);
    color: #616878;
    font-size: 8.5px;
  }
  .content {
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    padding: 20px 22px 24px;
  }
  .content.flush {
    padding: 0;
    overflow: hidden;
  }
  .overview {
    max-width: 560px;
  }
  .icon-row {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 18px;
  }
  .icon-preview {
    width: 74px;
    height: 74px;
    flex: none;
    border: 1px solid var(--line-strong);
    border-radius: 16px;
    background: var(--panel-2);
    overflow: hidden;
    position: relative;
    display: grid;
    place-items: center;
    color: var(--muted);
    padding: 0;
  }
  .icon-preview img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .icon-preview.disabled {
    cursor: default;
  }
  .icon-overlay {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.48);
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 120ms ease;
  }
  .icon-preview:hover .icon-overlay {
    opacity: 1;
  }
  .icon-copy b {
    font-size: 12px;
    font-weight: 650;
  }
  .icon-copy p {
    margin: 2px 0 0;
    font-size: 10.5px;
    color: var(--muted);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 14px;
  }
  .field > span {
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .field-note {
    font-size: 9.5px;
    line-height: 1.45;
    color: var(--muted);
  }
  input,
  textarea,
  select {
    width: 100%;
    border: 1px solid var(--line-strong);
    border-radius: 8px;
    background: var(--input);
    color: var(--text);
    font-size: 12.5px;
    outline: none;
  }
  input {
    height: 38px;
    padding: 0 10px;
  }
  textarea {
    padding: 9px 10px;
    resize: vertical;
    line-height: 1.5;
  }
  select {
    height: 38px;
    padding: 0 8px;
  }
  input:focus,
  textarea:focus,
  select:focus {
    border-color: rgba(var(--accent-rgb), 0.4);
    box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.06);
  }
  input:disabled,
  textarea:disabled,
  select:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .facts {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    margin: 16px 0 4px;
  }
  .facts div {
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel-2);
    padding: 8px 9px;
    min-width: 0;
  }
  .facts span {
    display: block;
    font-size: 8.5px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
  }
  .facts b {
    display: block;
    margin-top: 3px;
    font-size: 10px;
    font-weight: 600;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .footer-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-top: 18px;
    padding-top: 16px;
    border-top: 1px solid var(--line);
    color: var(--muted);
    font-size: 10px;
  }
  .primary {
    height: 34px;
    padding: 0 13px;
    border: 0;
    border-radius: 8px;
    background: var(--accent);
    color: white;
    font-size: 11px;
    font-weight: 650;
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }
  .primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 86%, white);
  }
  .primary:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .denied {
    color: #d68a8a !important;
    font-size: 10.5px;
    margin: 6px 0 0;
  }
  .empty {
    color: var(--muted);
    font-size: 11.5px;
    padding: 10px;
  }
</style>
