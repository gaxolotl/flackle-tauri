<script lang="ts">
  import * as api from "../../api";
  import { confirmAction } from "../../dialog";
  import { hasPermission, Permission, type PermissionName } from "../../permissions";
  import type { Role, Server } from "../../types";
  import { applyRoles, notify, serverPermissions } from "../../workspace";
  import Emoji from "../Emoji.svelte";
  import EmojiPicker from "../EmojiPicker.svelte";
  import EmojiText from "../EmojiText.svelte";
  import Icon from "../Icon.svelte";

  export let server: Server;

  type PermissionItem = { key: PermissionName; label: string; hint?: string };

  const permissionGroups: Array<{ name: string; items: PermissionItem[] }> = [
    {
      name: "General server permissions",
      items: [
        { key: "ViewChannel", label: "View Channels" },
        { key: "ManageChannels", label: "Manage Channels" },
        { key: "ManageRoles", label: "Manage Roles" },
        { key: "ManageServer", label: "Manage Server" },
        { key: "ViewAuditLog", label: "View Audit Log" },
        { key: "ManageMembers", label: "Manage Members" },
        { key: "ManageNicknames", label: "Manage Nicknames" },
        { key: "ManagePermissions", label: "Manage Permissions" },
        {
          key: "Administrator",
          label: "Administrator",
          hint: "Grants every permission and bypasses channel overrides.",
        },
      ],
    },
    {
      name: "Membership",
      items: [
        { key: "CreateInvite", label: "Create Invite" },
        { key: "KickMembers", label: "Kick Members" },
        { key: "BanMembers", label: "Ban Members" },
        { key: "TimeoutMembers", label: "Timeout Members" },
      ],
    },
    {
      name: "Text channels",
      items: [
        { key: "SendMessages", label: "Send Messages" },
        { key: "AttachFiles", label: "Attach Files" },
        { key: "AddReactions", label: "Add Reactions" },
        { key: "MentionEveryone", label: "Mention Everyone" },
        { key: "ManageMessages", label: "Manage Messages" },
        { key: "PinMessages", label: "Pin Messages" },
        { key: "BulkDelete", label: "Bulk Delete Messages" },
        { key: "BypassSlowmode", label: "Bypass Slowmode" },
        { key: "SaveMedia", label: "Save Media" },
      ],
    },
    {
      name: "Voice channels",
      items: [
        { key: "Connect", label: "Connect" },
        { key: "Speak", label: "Speak" },
        { key: "UseVideo", label: "Video" },
        { key: "MuteMembers", label: "Mute Members" },
        { key: "DeafenMembers", label: "Deafen Members" },
        { key: "MoveMembers", label: "Move Members" },
      ],
    },
    {
      name: "Custom channels",
      items: [
        {
          key: "ViewGeneralStats",
          label: "View General Stats",
          hint: "Member counts, online members, voice activity, and activity trends.",
        },
        {
          key: "ViewElevatedStats",
          label: "View Elevated Stats",
          hint: "Daily, weekly, and monthly active-member counts and trends.",
        },
      ],
    },
  ];

  let roles: Role[] = [];
  let selectedId: string | null = null;
  let name = "";
  let color = "#99aab5";
  let hoist = false;
  let mentionable = false;
  let permissions = "0";
  let canViewPermissions = true;
  let loading = true;
  let saving = false;
  let creating = false;
  let deleting = false;
  let uploadingIcon = false;
  let emojiPicker: { x: number; y: number } | null = null;
  let error = "";
  let lastServerId = "";

  $: canManageRoles =
    $serverPermissions !== null && hasPermission($serverPermissions, "ManageRoles");
  $: ordered = [...roles].sort((a, b) => (b.position ?? 0) - (a.position ?? 0));
  $: selected = roles.find((role) => role.id === selectedId) ?? null;
  $: roleMembers = selected?.member_count ?? null;

  $: if (server.id !== lastServerId) {
    lastServerId = server.id;
    selectedId = null;
    void load();
  }

  async function load(preferred?: string | null, preserveForm = false) {
    loading = true;
    error = "";
    try {
      const response = await api.fetchRoles(server.id);
      roles = response.roles ?? [];
      canViewPermissions = response.can_view_role_permissions !== false;
      applyRoles(roles);
      const next =
        (preferred && roles.find((role) => role.id === preferred)) ||
        roles.find((role) => role.id === selectedId) ||
        roles.find((role) => !role.is_everyone) ||
        roles[0] ||
        null;
      if (next) {
        if (!preserveForm || next.id !== selectedId) choose(next);
      } else {
        selectedId = null;
        resetForm();
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
    }
  }

  function resetForm() {
    name = "";
    color = "#99aab5";
    hoist = false;
    mentionable = false;
    permissions = "0";
  }

  function choose(role: Role) {
    selectedId = role.id;
    name = role.name;
    color = role.color || "#99aab5";
    hoist = role.hoist ?? false;
    mentionable = role.mentionable ?? false;
    permissions = role.permissions || "0";
  }

  function has(permission: PermissionName) {
    const bits = BigInt(permissions || "0");
    const bit = Permission[permission];
    return (bits & bit) === bit;
  }

  function toggle(permission: PermissionName) {
    if (!canManageRoles || !selected) return;
    let bits = BigInt(permissions || "0");
    const bit = Permission[permission];
    if ((bits & bit) === bit) bits &= ~bit;
    else bits |= bit;
    permissions = bits.toString();
  }

  async function uploadRoleIcon() {
    if (!selected || !canManageRoles || uploadingIcon) return;
    const path = await api.pickImage();
    if (!path) return;
    uploadingIcon = true;
    try {
      await api.uploadRoleIcon(server.id, selected.id, path);
      await load(selected.id, true);
      notify("Role icon updated.");
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      uploadingIcon = false;
    }
  }

  function openEmojiPicker(event: MouseEvent) {
    if (!canManageRoles) return;
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    emojiPicker = { x: rect.left, y: rect.bottom + 6 };
  }

  async function applyRoleEmoji(emoji: string) {
    if (!selected || !canManageRoles) return;
    emojiPicker = null;
    try {
      await api.batchRoles(server.id, {
        updates: [{ id: selected.id, icon_emoji: emoji }],
      });
      await load(selected.id, true);
      notify("Role icon updated.");
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function removeRoleIcon() {
    if (!selected || !canManageRoles) return;
    try {
      await api.batchRoles(server.id, {
        updates: [{ id: selected.id, clear_icon: true }],
      });
      await load(selected.id, true);
      notify("Role icon removed.");
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function save() {
    if (!selected || !canManageRoles || saving) return;
    if (!name.trim()) {
      notify("A role needs a name.");
      return;
    }
    saving = true;
    try {
      await api.batchRoles(server.id, {
        updates: [
          {
            id: selected.id,
            name: name.trim(),
            permissions,
            color: color || null,
            hoist,
            mentionable,
          },
        ],
      });
      await load(selected.id);
      notify("Role updated.");
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      saving = false;
    }
  }

  async function createRole() {
    if (!canManageRoles || creating) return;
    creating = true;
    try {
      const result = await api.batchRoles(server.id, {
        creates: [
          {
            id: "draft:new",
            name: "New Role",
            permissions: "0",
            color: "#99aab5",
            hoist: false,
            mentionable: false,
          },
        ],
      });
      const created = result.created_role_ids?.["draft:new"] ?? null;
      await load(created);
      notify("Role created.");
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      creating = false;
    }
  }

  async function deleteRole() {
    if (!selected || selected.is_everyone || !canManageRoles || deleting) return;
    const confirmed = await confirmAction({
      title: `Delete ${selected.name}?`,
      body: "Members with this role will lose it. This cannot be undone.",
      confirmLabel: "Delete role",
      danger: true,
    });
    if (!confirmed) return;
    deleting = true;
    try {
      await api.batchRoles(server.id, { deletes: [{ id: selected.id }] });
      selectedId = null;
      await load();
      notify("Role deleted.");
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      deleting = false;
    }
  }

  async function moveRole(offset: number) {
    if (!selected || selected.is_everyone || !canManageRoles) return;
    const list = [...ordered];
    const index = list.findIndex((role) => role.id === selected.id);
    const target = index + offset;
    if (index < 0 || target < 0 || target >= list.length) return;
    if (list[target]?.is_everyone) return;
    [list[index], list[target]] = [list[target], list[index]];
    try {
      await api.batchRoles(server.id, { order: list.map((role) => role.id) });
      await load(selected.id);
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    }
  }

  function roleColor(role: Role) {
    return role.color || "#99aab5";
  }
</script>

<div class="roles">
  <aside class="role-list">
    <div class="list-head">
      <span>Roles</span>
      <button
        class="tiny"
        title="Create role"
        aria-label="Create role"
        disabled={!canManageRoles || creating}
        on:click={createRole}
      >
        <Icon name="plus" size={14} />
      </button>
    </div>
    {#if loading}
      <p class="empty">Loading roles…</p>
    {:else if roles.length === 0}
      <p class="empty">No roles yet.</p>
    {:else}
      {#each ordered as role (role.id)}
        <button
          class="role-row"
          class:active={role.id === selectedId}
          on:click={() => choose(role)}
        >
          <span class="role-dot" style={`--role-color:${roleColor(role)}`}>
            {#if role.icon_url}
              <img src={role.icon_url} alt="" />
            {:else if role.icon_emoji}
              <Emoji char={role.icon_emoji} size={14} />
            {:else}
              <Icon name={role.is_everyone ? "users" : "shield"} size={14} />
            {/if}
          </span>
          <span class="role-name">
            <b><EmojiText text={role.name} size={11.5} /></b>
            <small>
              {role.is_everyone
                ? "Default role"
                : role.member_count != null
                  ? `${role.member_count} member${role.member_count === 1 ? "" : "s"}`
                  : "Role"}
            </small>
          </span>
          {#if role.hoist}<span class="flag" title="Hoisted">H</span>{/if}
        </button>
      {/each}
    {/if}
  </aside>

  <section class="role-editor">
    {#if error}
      <p class="denied">{error}</p>
    {:else if !selected}
      <div class="blank">
        <Icon name="shield" size={28} />
        <b>Select a role</b>
        <span>Choose a role to edit its name, color, and permissions.</span>
      </div>
    {:else}
      <div class="editor-scroll">
        <div class="editor-head">
          <div>
            <h3><EmojiText text={selected.name} size={16} /></h3>
            <p>
              {selected.is_everyone
                ? "Applied to every member of this server."
                : "Role members are assigned by server managers."}
            </p>
          </div>
          <div class="head-actions">
            <button
              class="icon-btn"
              title="Move role up"
              aria-label="Move role up"
              disabled={!canManageRoles || selected.is_everyone || ordered[0]?.id === selected.id}
              on:click={() => moveRole(-1)}
            >
              <Icon name="up" size={15} />
            </button>
            <button
              class="icon-btn"
              title="Move role down"
              aria-label="Move role down"
              disabled={!canManageRoles || selected.is_everyone || ordered[ordered.length - 1]?.id === selected.id}
              on:click={() => moveRole(1)}
            >
              <Icon name="down" size={15} />
            </button>
            <button
              class="icon-btn danger"
              title="Delete role"
              aria-label="Delete role"
              disabled={!canManageRoles || selected.is_everyone || deleting}
              on:click={deleteRole}
            >
              <Icon name="trash" size={15} />
            </button>
          </div>
        </div>

        <div class="identity-row">
          <span class="role-dot large" style={`--role-color:${color}`}>
            {#if selected.icon_url}
              <img src={selected.icon_url} alt="" />
            {:else if selected.icon_emoji}
              <Emoji char={selected.icon_emoji} size={24} />
            {:else}
              <Icon name={selected.is_everyone ? "users" : "shield"} size={17} />
            {/if}
          </span>
          <div class="identity-fields">
            <label class="field">
              <span>Role name</span>
              <input
                bind:value={name}
                disabled={!canManageRoles || selected.is_everyone}
                maxlength="100"
              />
            </label>
            <label class="field color-field">
              <span>Role color</span>
              <div class="color-input">
                <input type="color" bind:value={color} disabled={!canManageRoles} />
                <input
                  value={color}
                  disabled={!canManageRoles}
                  spellcheck="false"
                  maxlength="7"
                  on:change={(event) => (color = event.currentTarget.value)}
                />
              </div>
            </label>
          </div>
        </div>

        <div class="role-icon-row">
          <div class="role-icon-preview" aria-label={selected.icon_url || selected.icon_emoji ? "Role icon preview" : "No role icon"}>
            {#if selected.icon_url}
              <img src={selected.icon_url} alt="" />
            {:else if selected.icon_emoji}
              <Emoji char={selected.icon_emoji} size={38} />
            {:else}
              <Icon name="image" size={26} />
            {/if}
            {#if (selected.icon_url || selected.icon_emoji) && canManageRoles}
              <button
                class="icon-remove"
                title="Remove icon"
                aria-label="Remove icon"
                on:click={removeRoleIcon}
              >
                <Icon name="close" size={10} />
              </button>
            {/if}
          </div>
          <div class="role-icon-copy">
            <b>Role icon</b>
            <p>
              {selected.icon_url || selected.icon_emoji
                ? "Make this role recognisable."
                : "Give this role an icon."}
              PNG, JPG, GIF or WebP · Up to 2 MB.
            </p>
            <div class="icon-actions">
              <button
                class="icon-action"
                disabled={!canManageRoles || uploadingIcon}
                on:click={uploadRoleIcon}
              >
                <Icon name="image" size={13} />
                {uploadingIcon
                  ? "Uploading…"
                  : selected.icon_url
                    ? "Replace image"
                    : "Upload image"}
              </button>
              <button
                class="icon-action ghost"
                disabled={!canManageRoles}
                on:click={openEmojiPicker}
              >
                <Icon name="smile" size={13} />
                {selected.icon_emoji ? "Change emoji" : "Pick emoji"}
              </button>
            </div>
          </div>
        </div>

        <div class="switch-row">
          <div>
            <b>Display role members separately</b>
            <p>Show this role as its own group in the member list.</p>
          </div>
          <button
            class="switch"
            class:on={hoist}
            role="switch"
            aria-label="Display role members separately"
            aria-checked={hoist}
            disabled={!canManageRoles}
            on:click={() => (hoist = !hoist)}
          >
            <span></span>
          </button>
        </div>
        <div class="switch-row">
          <div>
            <b>Allow anyone to mention this role</b>
            <p>Members can ping this role with @{selected.name}.</p>
          </div>
          <button
            class="switch"
            class:on={mentionable}
            role="switch"
            aria-label="Allow anyone to mention this role"
            aria-checked={mentionable}
            disabled={!canManageRoles}
            on:click={() => (mentionable = !mentionable)}
          >
            <span></span>
          </button>
        </div>

        <div class="members-note">
          {#if roleMembers != null}
            <Icon name="users" size={14} />
            {roleMembers} member{roleMembers === 1 ? "" : "s"} currently {roleMembers === 1 ? "has" : "have"} this role.
          {:else}
            <Icon name="shield" size={14} />
            Role permissions apply everywhere unless a channel overrides them.
          {/if}
        </div>

        {#if !canViewPermissions}
          <p class="denied">You do not have permission to view this role's permissions.</p>
        {:else}
          {#each permissionGroups as group (group.name)}
            <div class="permission-group">
              <h4>{group.name}</h4>
              {#each group.items as item (item.key)}
                <div class="permission-row">
                  <div>
                    <b>{item.label}</b>
                    {#if item.hint}<p>{item.hint}</p>{/if}
                  </div>
                  <button
                    class="switch"
                    class:on={has(item.key)}
                    role="switch"
                    aria-label={item.label}
                    aria-checked={has(item.key)}
                    disabled={!canManageRoles}
                    on:click={() => toggle(item.key)}
                  >
                    <span></span>
                  </button>
                </div>
              {/each}
            </div>
          {/each}
        {/if}
      </div>

      <footer class="editor-foot">
        <span>
          {canManageRoles
            ? selected.is_everyone
              ? "Changes affect everyone."
              : "Changes apply to members with this role."
            : "You do not have permission to edit roles."}
        </span>
        <button
          class="primary"
          disabled={!canManageRoles || saving}
          on:click={save}
        >
          {saving ? "Saving…" : "Save changes"}
        </button>
      </footer>
    {/if}
  </section>
</div>

{#if emojiPicker && selected}
  <EmojiPicker
    x={emojiPicker.x}
    y={emojiPicker.y}
    onSelect={applyRoleEmoji}
    onClose={() => (emojiPicker = null)}
  />
{/if}

<style>
  .roles {
    display: flex;
    height: 100%;
    min-height: 0;
  }
  .role-list {
    width: 198px;
    flex: none;
    border-right: 1px solid var(--line);
    overflow-y: auto;
    padding: 10px 8px;
  }
  .list-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 2px 6px 8px;
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .tiny {
    width: 24px;
    height: 24px;
    flex: none;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--panel-2);
    color: var(--text-dim);
    display: grid;
    place-items: center;
    line-height: 0;
  }
  .tiny:hover:not(:disabled) {
    color: var(--text);
    background: var(--panel-3);
  }
  .tiny:disabled {
    opacity: 0.35;
  }
  .role-row {
    width: 100%;
    min-height: 40px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: var(--text-dim);
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 7px;
    text-align: left;
  }
  .role-row:hover {
    background: var(--hover);
    color: var(--text);
  }
  .role-row.active {
    background: var(--active);
    border-color: var(--line);
    color: var(--text);
  }
  .role-dot {
    width: 24px;
    height: 24px;
    flex: none;
    border-radius: 50%;
    background: color-mix(in srgb, var(--role-color) 22%, var(--panel-2));
    color: var(--role-color);
    border: 1px solid color-mix(in srgb, var(--role-color) 36%, transparent);
    display: grid;
    place-items: center;
    overflow: hidden;
  }
  .role-dot.large {
    width: 42px;
    height: 42px;
    border-radius: 12px;
  }
  .role-dot img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .role-name {
    min-width: 0;
    display: flex;
    flex-direction: column;
    line-height: 1.2;
  }
  .role-name b {
    font-size: 11.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .role-name small {
    margin-top: 2px;
    font-size: 9px;
    color: var(--muted);
  }
  .flag {
    margin-left: auto;
    font-size: 8px;
    font-weight: 750;
    color: var(--accent);
    border: 1px solid rgba(var(--accent-rgb), 0.25);
    background: var(--accent-dim);
    border-radius: 4px;
    padding: 1px 4px;
  }
  .role-editor {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .blank {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 5px;
    color: var(--muted);
    padding: 24px;
    text-align: center;
  }
  .blank b {
    margin-top: 4px;
    color: var(--text-dim);
    font-size: 12px;
  }
  .blank span {
    max-width: 250px;
    font-size: 10.5px;
    line-height: 1.5;
  }
  .editor-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 16px 20px 20px;
  }
  .editor-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 16px;
  }
  .editor-head h3 {
    margin: 0;
    font-size: 16px;
    letter-spacing: -0.03em;
  }
  .editor-head p {
    margin: 2px 0 0;
    font-size: 10.5px;
    color: var(--muted);
  }
  .head-actions {
    display: flex;
    gap: 3px;
  }
  .icon-btn {
    width: 28px;
    height: 28px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--panel-2);
    color: var(--muted);
    display: grid;
    place-items: center;
  }
  .icon-btn:hover:not(:disabled) {
    background: var(--panel-3);
    color: var(--text);
  }
  .icon-btn.danger:hover:not(:disabled) {
    color: #ff8a8a;
    background: rgba(var(--accent-rgb), 0.12);
  }
  .icon-btn:disabled {
    opacity: 0.3;
  }
  .role-icon-row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 0 14px;
    border-top: 1px solid var(--line);
    margin-bottom: 2px;
  }
  .role-icon-preview {
    width: 68px;
    height: 68px;
    flex: none;
    border: 1px solid var(--line-strong);
    border-radius: 14px;
    background: var(--panel-2);
    display: grid;
    place-items: center;
    position: relative;
    overflow: visible;
    color: var(--muted);
  }
  .role-icon-preview img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 13px;
  }
  .icon-remove {
    position: absolute;
    top: -6px;
    right: -6px;
    width: 18px;
    height: 18px;
    border: 2px solid var(--card);
    border-radius: 50%;
    background: var(--panel-3);
    color: var(--text-dim);
    display: grid;
    place-items: center;
    padding: 0;
  }
  .icon-remove:hover {
    background: var(--accent);
    color: white;
  }
  .role-icon-copy {
    min-width: 0;
    flex: 1;
  }
  .role-icon-copy > b {
    display: block;
    font-size: 11.5px;
    font-weight: 650;
    color: var(--text);
  }
  .role-icon-copy > p {
    margin: 2px 0 8px;
    font-size: 9.5px;
    line-height: 1.45;
    color: var(--muted);
  }
  .icon-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .icon-action {
    height: 28px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--panel-2);
    color: var(--text-dim);
    padding: 0 9px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 9.5px;
    font-weight: 600;
  }
  .icon-action:hover:not(:disabled) {
    background: var(--panel-3);
    color: var(--text);
  }
  .icon-action.ghost {
    background: transparent;
    color: var(--muted);
  }
  .icon-action.ghost:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  .icon-action:disabled {
    opacity: 0.4;
  }
  .identity-row {
    display: flex;
    gap: 13px;
    align-items: flex-start;
    margin-bottom: 15px;
  }
  .identity-fields {
    flex: 1;
    min-width: 0;
    display: flex;
    gap: 10px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    flex: 1;
    min-width: 0;
  }
  .field > span {
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .field input {
    height: 36px;
    border: 1px solid var(--line-strong);
    border-radius: 8px;
    background: var(--input);
    color: var(--text);
    padding: 0 9px;
    font-size: 12px;
    outline: none;
    min-width: 0;
  }
  .field input:focus {
    border-color: rgba(var(--accent-rgb), 0.4);
    box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.06);
  }
  .field input:disabled {
    opacity: 0.55;
  }
  .color-field {
    max-width: 154px;
  }
  .color-input {
    display: flex;
    gap: 6px;
  }
  .color-input input[type="color"] {
    width: 36px;
    padding: 3px;
    flex: none;
  }
  .switch-row,
  .permission-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 9px 0;
    border-top: 1px solid var(--line);
  }
  .switch-row b,
  .permission-row b {
    display: block;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-dim);
  }
  .switch-row p,
  .permission-row p {
    margin: 2px 0 0;
    font-size: 9.5px;
    line-height: 1.4;
    color: var(--muted);
  }
  .switch {
    width: 36px;
    height: 20px;
    flex: none;
    border: 1px solid var(--line-strong);
    border-radius: 99px;
    background: var(--panel-3);
    padding: 2px;
    display: flex;
    align-items: center;
    transition: background 120ms ease, border-color 120ms ease;
  }
  .switch span {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #8a91a0;
    transition: transform 120ms ease, background 120ms ease;
  }
  .switch.on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .switch.on span {
    background: white;
    transform: translateX(16px);
  }
  .switch:disabled {
    opacity: 0.45;
  }
  .members-note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 14px 0 4px;
    padding: 8px 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel-2);
    color: var(--muted);
    font-size: 9.5px;
  }
  .permission-group {
    margin-top: 17px;
  }
  .permission-group h4 {
    margin: 0 0 3px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .editor-foot {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 12px 20px;
    border-top: 1px solid var(--line);
    color: var(--muted);
    font-size: 9.5px;
  }
  .primary {
    height: 32px;
    border: 0;
    border-radius: 8px;
    background: var(--accent);
    color: white;
    padding: 0 13px;
    font-size: 10.5px;
    font-weight: 650;
  }
  .primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 86%, white);
  }
  .primary:disabled {
    opacity: 0.45;
  }
  .empty,
  .denied {
    padding: 12px 8px;
    margin: 0;
    color: var(--muted);
    font-size: 10px;
    line-height: 1.5;
  }
  .denied {
    color: #d68a8a;
    padding: 16px;
  }
</style>
