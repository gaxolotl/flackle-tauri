<script lang="ts">
  import * as api from "../../api";
  import { confirmAction } from "../../dialog";
  import type { ContextMenuItem } from "../../context-menu";
  import { hasPermission } from "../../permissions";
  import { openProfile } from "../../profile";
  import { showUserMenu } from "../../user-menu";
  import type { Member, Role, Server } from "../../types";
  import { me, memberRoleUpdate, notify, serverPermissions, setMemberRoles as applyMemberRoles } from "../../workspace";
  import Avatar from "../Avatar.svelte";
  import Emoji from "../Emoji.svelte";
  import EmojiText from "../EmojiText.svelte";
  import Icon from "../Icon.svelte";

  export let server: Server;

  let members: Member[] = [];
  let roles: Role[] = [];
  let query = "";
  let loading = true;
  let error = "";
  let busy: string | null = null;
  let lastServerId = "";
  let lastRoleUpdate = 0;

  $: canManageRoles =
    $serverPermissions !== null && hasPermission($serverPermissions, "ManageRoles");
  $: canKick =
    $serverPermissions !== null && hasPermission($serverPermissions, "KickMembers");
  $: canBan =
    $serverPermissions !== null && hasPermission($serverPermissions, "BanMembers");
  $: roleMap = new Map(roles.map((role) => [role.id, role]));
  $: filtered = members.filter((member) => {
    const needle = query.trim().toLowerCase();
    if (!needle) return true;
    const user = member.user;
    return [user.username, user.display_name, member.nickname, user.id]
      .filter(Boolean)
      .some((value) => String(value).toLowerCase().includes(needle));
  });
  $: assignable = roles
    .filter((role) => !role.is_everyone)
    .sort((a, b) => (b.position ?? 0) - (a.position ?? 0));

  $: if (
    $memberRoleUpdate &&
    $memberRoleUpdate.serverId === server.id &&
    $memberRoleUpdate.at !== lastRoleUpdate
  ) {
    lastRoleUpdate = $memberRoleUpdate.at;
    const update = $memberRoleUpdate;
    members = members.map((entry) =>
      entry.user.id === update.member.user.id
        ? mergeMember(entry, update.member)
        : entry,
    );
  }

  $: if (server.id !== lastServerId) {
    lastServerId = server.id;
    void load();
  }

  async function load() {
    loading = true;
    error = "";
    try {
      const [memberList, roleResponse] = await Promise.all([
        api.fetchMembers(server.id),
        api.fetchRoles(server.id),
      ]);
      members = memberList;
      roles = roleResponse.roles ?? [];
      lastRoleUpdate = $memberRoleUpdate?.at ?? 0;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
    }
  }

  function memberName(member: Member) {
    return member.nickname || member.user.display_name || member.user.username;
  }

  function roleIds(member: Member): string[] {
    return member.role_ids ?? member.roles ?? [];
  }

  function memberRoles(member: Member): Role[] {
    return roleIds(member)
      .map((id) => roleMap.get(id))
      .filter((role): role is Role => Boolean(role) && !role?.is_everyone)
      .sort((a, b) => (b.position ?? 0) - (a.position ?? 0));
  }

  function roleStyle(role: Role) {
    return `--role-color:${role.color || "#99aab5"}`;
  }

  function mergeMember(member: Member, updated: Member): Member {
    return {
      ...member,
      ...updated,
      user: updated.user?.username
        ? { ...member.user, ...updated.user }
        : member.user,
    };
  }

  async function addRole(member: Member, roleId: string) {
    if (!canManageRoles || busy === member.user.id) return;
    busy = member.user.id;
    try {
      const updated = await applyMemberRoles(member.user.id, [roleId], []);
      if (!updated) return;
      members = members.map((entry) =>
        entry.user.id === member.user.id ? mergeMember(entry, updated) : entry,
      );
      notify(`Added ${roleMap.get(roleId)?.name ?? "role"} to ${memberName(member)}.`);
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      busy = null;
    }
  }

  async function removeRole(member: Member, roleId: string) {
    if (!canManageRoles || busy === member.user.id) return;
    busy = member.user.id;
    try {
      const updated = await applyMemberRoles(member.user.id, [], [roleId]);
      if (!updated) return;
      members = members.map((entry) =>
        entry.user.id === member.user.id ? mergeMember(entry, updated) : entry,
      );
      notify(`Removed ${roleMap.get(roleId)?.name ?? "role"} from ${memberName(member)}.`);
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      busy = null;
    }
  }

  function showMemberMenu(event: MouseEvent, member: Member) {
    const extraItems: ContextMenuItem[] = [];
    if (canKick) {
      extraItems.push({
        label: `Kick ${memberName(member)}`,
        icon: "alert",
        danger: true,
        disabled: member.is_owner || member.user.id === $me?.id,
        run: () => void kick(member),
      });
    }
    if (canBan) {
      extraItems.push({
        label: `Ban ${memberName(member)}`,
        icon: "ban",
        danger: true,
        disabled: member.is_owner || member.user.id === $me?.id,
        run: () => void ban(member),
      });
    }
    showUserMenu(event, member.user, member, { extraItems });
  }

  async function kick(member: Member) {
    if (member.is_owner || member.user.id === $me?.id) return;
    const confirmed = await confirmAction({
      title: `Kick ${memberName(member)}?`,
      body: "They can join again with an invite.",
      confirmLabel: "Kick member",
      danger: true,
    });
    if (!confirmed) return;
    try {
      await api.kickMember(server.id, member.user.id);
      members = members.filter((entry) => entry.user.id !== member.user.id);
      notify(`${memberName(member)} was kicked.`);
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function ban(member: Member) {
    if (member.is_owner || member.user.id === $me?.id) return;
    const confirmed = await confirmAction({
      title: `Ban ${memberName(member)}?`,
      body: "They will be removed and cannot rejoin until unbanned.",
      confirmLabel: "Ban member",
      danger: true,
    });
    if (!confirmed) return;
    try {
      await api.banMember(server.id, member.user.id);
      members = members.filter((entry) => entry.user.id !== member.user.id);
      notify(`${memberName(member)} was banned.`);
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    }
  }
</script>

<div class="members">
  <div class="members-head">
    <div>
      <h3>Members</h3>
      <p>{members.length} member{members.length === 1 ? "" : "s"} in {server.name}</p>
    </div>
    <label class="search">
      <Icon name="search" size={14} />
      <input bind:value={query} placeholder="Search members" spellcheck="false" />
    </label>
  </div>

  {#if error}
    <p class="denied">{error}</p>
  {:else if loading}
    <p class="empty">Loading members…</p>
  {:else}
    <div class="table">
      <div class="table-head">
        <span>Member</span>
        <span>Roles</span>
        <span>Actions</span>
      </div>
      {#each filtered as member (member.user.id)}
        <div class="member-row">
          <button
            class="member-main"
            on:click={(event) => openProfile(member.user, member, event.currentTarget as HTMLElement | null)}
          >
            <Avatar user={member.user} size={32} />
            <span class="member-copy">
              <b>{memberName(member)}</b>
              <small>
                {member.is_owner ? "Server owner" : `@${member.user.username}`}
              </small>
            </span>
          </button>

          <div class="role-cell">
            {#each memberRoles(member) as role (role.id)}
              <span class="role-chip" style={roleStyle(role)}>
                {#if role.icon_url}
                  <img src={role.icon_url} alt="" />
                {:else if role.icon_emoji}
                  <Emoji char={role.icon_emoji} size={12} />
                {:else}
                  <i></i>
                {/if}
                <EmojiText text={role.name} size={9.5} />
                {#if canManageRoles}
                  <button
                    class="role-remove"
                    title={`Remove ${role.name}`}
                    aria-label={`Remove ${role.name} from ${memberName(member)}`}
                    disabled={busy === member.user.id}
                    on:click={() => removeRole(member, role.id)}
                  >
                    <Icon name="close" size={10} />
                  </button>
                {/if}
              </span>
            {/each}
            {#if canManageRoles && assignable.length > 0}
              <select
                class="role-add"
                value=""
                disabled={busy === member.user.id}
                aria-label={`Add a role to ${memberName(member)}`}
                on:change={(event) => {
                  const value = event.currentTarget.value;
                  event.currentTarget.value = "";
                  if (value) void addRole(member, value);
                }}
              >
                <option value="">+ Add role</option>
                {#each assignable as role (role.id)}
                  <option value={role.id}>{role.name}</option>
                {/each}
              </select>
            {/if}
          </div>

          <div class="row-actions">
            <button
              class="more"
              title={`More for ${memberName(member)}`}
              aria-label={`More for ${memberName(member)}`}
              on:click={(event) => showMemberMenu(event, member)}
            >
              <Icon name="more" size={15} />
            </button>
          </div>
        </div>
      {/each}
      {#if filtered.length === 0}
        <p class="empty">No members match “{query}”.</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .members {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 16px 18px 18px;
  }
  .members-head {
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
  .members-head p {
    margin: 2px 0 0;
    font-size: 10.5px;
    color: var(--muted);
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
  .table {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    border: 1px solid var(--line);
    border-radius: 10px;
  }
  .table-head,
  .member-row {
    display: grid;
    grid-template-columns: minmax(180px, 1.2fr) minmax(240px, 2fr) 44px;
    align-items: center;
    gap: 12px;
    padding: 7px 10px;
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
  .member-row {
    min-height: 50px;
    border-bottom: 1px solid var(--line);
  }
  .member-row:last-child {
    border-bottom: 0;
  }
  .member-row:hover {
    background: color-mix(in srgb, var(--panel-2) 65%, transparent);
  }
  .member-main {
    border: 0;
    background: transparent;
    color: var(--text);
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0;
    text-align: left;
    min-width: 0;
  }
  .member-copy {
    display: flex;
    flex-direction: column;
    min-width: 0;
    line-height: 1.2;
  }
  .member-copy b {
    font-size: 11.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .member-copy small {
    margin-top: 2px;
    color: var(--muted);
    font-size: 9.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .role-cell {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px;
    min-width: 0;
  }
  .role-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 150px;
    height: 22px;
    padding: 0 4px 0 6px;
    border: 1px solid color-mix(in srgb, var(--role-color) 30%, var(--line));
    border-radius: 99px;
    background: color-mix(in srgb, var(--role-color) 12%, var(--panel-2));
    color: color-mix(in srgb, var(--role-color) 52%, var(--text));
    font-size: 9.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .role-chip img {
    width: 13px;
    height: 13px;
    border-radius: 50%;
    object-fit: cover;
  }
  .role-chip i {
    width: 6px;
    height: 6px;
    flex: none;
    border-radius: 50%;
    background: var(--role-color);
  }
  .role-remove {
    width: 15px;
    height: 15px;
    flex: none;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: inherit;
    display: grid;
    place-items: center;
    padding: 0;
  }
  .role-remove:hover {
    background: rgba(0, 0, 0, 0.28);
  }
  .role-add {
    height: 22px;
    max-width: 105px;
    border: 1px dashed var(--line-strong);
    border-radius: 99px;
    background: transparent;
    color: var(--muted);
    font-size: 9.5px;
    padding: 0 5px;
    outline: none;
  }
  .role-add:hover {
    color: var(--text-dim);
    border-color: var(--muted);
  }
  .row-actions {
    display: flex;
    justify-content: flex-end;
  }
  .more {
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    display: grid;
    place-items: center;
  }
  .more:hover {
    background: var(--hover);
    color: var(--text);
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
