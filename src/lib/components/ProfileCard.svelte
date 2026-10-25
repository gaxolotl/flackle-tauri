<script lang="ts">
  import { afterUpdate, onMount, tick } from "svelte";
  import * as api from "../api";
  import { confirmAction, reportAction } from "../dialog";
  import { openContextMenu } from "../context-menu";
  import { hasPermission } from "../permissions";
  import { prefs } from "../preferences";
  import { closeProfile, openFullProfile, profile } from "../profile";
  import { openImage } from "../image-viewer";
  import type { Member } from "../types";
  import {
    active,
    me,
    members,
    modal,
    notify,
    presenceStatuses,
    serverPermissions,
    serverRoles,
    setMemberRoles,
  } from "../workspace";
  import Avatar from "./Avatar.svelte";
  import Emoji from "./Emoji.svelte";
  import EmojiText from "./EmojiText.svelte";
  import Icon from "./Icon.svelte";

  const width = 300;
  const gap = 10;
  const margin = 10;
  let copied = false;
  let rolePickerOpen = false;
  let roleQuery = "";
  let rolePickerPos = { left: 0, top: 0 };
  let assigningRole = false;
  let lastTargetId = "";
  let cardEl: HTMLDivElement | undefined;
  let cardHeight = 420;
  let anchorRect: DOMRect | null = null;
  let position = { left: 0, top: 0 };

  $: target = $profile;
  $: roles = target
    ? $serverRoles
        .filter(
          (role) =>
            !role.is_everyone &&
            (target.member?.role_ids ?? target.member?.roles ?? []).includes(role.id),
        )
        .sort((a, b) => (b.position ?? 0) - (a.position ?? 0))
    : [];
  $: primaryRole = roles.find((role) => role.color) ?? null;
  $: isSelf = Boolean(target && $me?.id === target.user.id);
  $: canManageRoles =
    Boolean(target?.member) &&
    $active?.kind === "guild" &&
    $serverPermissions !== null &&
    hasPermission($serverPermissions, "ManageRoles");
  $: assignedIds = new Set(target?.member?.role_ids ?? target?.member?.roles ?? []);
  $: assignableRoles = $serverRoles
    .filter((role) => !role.is_everyone)
    .sort((a, b) => (b.position ?? 0) - (a.position ?? 0));
  $: filteredRoles = assignableRoles.filter((role) =>
    role.name.toLowerCase().includes(roleQuery.trim().toLowerCase()),
  );
  $: if (target?.user.id !== lastTargetId) {
    lastTargetId = target?.user.id ?? "";
    rolePickerOpen = false;
    roleQuery = "";
    if (target) {
      anchorRect =
        target.anchor && target.anchor.isConnected
          ? target.anchor.getBoundingClientRect()
          : new DOMRect(target.x, target.y, 1, 1);
    }
    void tick().then(() => measure());
  }
  $: status = target
    ? $presenceStatuses[target.user.id] ?? target.user.status ?? "offline"
    : "offline";
  $: liveMember = target
    ? $members.find((member) => member.user.id === target?.user.id)
    : undefined;
  $: if (target && liveMember && liveMember !== target.member) {
    profile.update((current) =>
      current && current.user.id === target?.user.id
        ? {
            ...current,
            member: liveMember,
            user: { ...current.user, ...liveMember.user },
          }
        : current,
    );
  }
  $: if (target) position = computePosition(anchorRect, cardHeight);

  function clamp(value: number, min: number, max: number) {
    return Math.max(min, Math.min(value, max));
  }

  function measure(updateAnchor = true) {
    if (!target) return;
    if (updateAnchor) {
      const element = target.anchor;
      const rect =
        element && element.isConnected ? element.getBoundingClientRect() : null;
      anchorRect = rect ?? new DOMRect(target.x, target.y, 1, 1);
    }
    cardHeight = cardEl?.offsetHeight || cardHeight;
  }

  function computePosition(rect: DOMRect | null, height: number) {
    if (!rect) return { left: 0, top: 0 };
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;
    const spaceRight = viewportWidth - rect.right - gap - margin;
    const spaceLeft = rect.left - gap - margin;
    const spaceBottom = viewportHeight - rect.bottom - gap - margin;
    const spaceTop = rect.top - gap - margin;
    let left: number;
    let top: number;

    if (spaceRight >= width) {
      // Right is the default when there is room.
      left = rect.right + gap;
      top = rect.top - 8;
    } else if (spaceLeft >= width) {
      left = rect.left - width - gap;
      top = rect.top - 8;
    } else if (spaceBottom >= Math.min(height, 240)) {
      left = rect.left + rect.width / 2 - width / 2;
      top = rect.bottom + gap;
    } else if (spaceTop >= Math.min(height, 240)) {
      left = rect.left + rect.width / 2 - width / 2;
      top = rect.top - height - gap;
    } else {
      left = spaceRight >= spaceLeft ? rect.right + gap : rect.left - width - gap;
      top = rect.top - 8;
    }

    return {
      left: clamp(left, margin, Math.max(margin, viewportWidth - width - margin)),
      top: clamp(top, margin, Math.max(margin, viewportHeight - height - margin)),
    };
  }

  afterUpdate(() => {
    if (!target) return;
    const next = cardEl?.offsetHeight;
    if (next && next !== cardHeight) cardHeight = next;
  });

  onMount(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") closeProfile();
    };
    const reposition = () => measure();
    window.addEventListener("keydown", onKey);
    window.addEventListener("resize", reposition);
    window.addEventListener("scroll", reposition, true);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("resize", reposition);
      window.removeEventListener("scroll", reposition, true);
    };
  });

  function statusText(status?: string | null): string {
    if (status === "idle") return "Idle";
    if (status === "dnd") return "Do not disturb";
    if (status === "invisible") return "Invisible";
    if (status === "offline") return "Offline";
    return "Online";
  }

  function joined(value?: string | null): string {
    if (!value) return "";
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return "";
    return date.toLocaleDateString([], { month: "long", year: "numeric" });
  }

  async function copyId() {
    if (!target) return;
    try {
      await navigator.clipboard.writeText(target.user.id);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      copied = false;
    }
  }

  function editSelf() {
    closeProfile();
    modal.set("settings");
  }

  function openRolePicker(event: MouseEvent) {
    if (rolePickerOpen) {
      rolePickerOpen = false;
      return;
    }
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const pickerWidth = 236;
    const pickerHeight = 252;
    const left = clamp(
      rect.right - pickerWidth,
      margin,
      Math.max(margin, window.innerWidth - pickerWidth - margin),
    );
    const below = rect.bottom + 6;
    const top =
      below + pickerHeight > window.innerHeight - margin
        ? Math.max(margin, rect.top - pickerHeight - 6)
        : below;
    rolePickerPos = { left, top };
    roleQuery = "";
    rolePickerOpen = true;
  }

  function mergeProfileMember(base: Member, updated: Member): Member {
    return {
      ...base,
      ...updated,
      user: updated.user?.username ? { ...base.user, ...updated.user } : base.user,
      role_ids: updated.role_ids ?? base.role_ids,
      roles: updated.roles ?? base.roles,
    };
  }

  function toggleRole(roleId: string) {
    if (assignedIds.has(roleId)) void removeRole(roleId);
    else void addRole(roleId);
  }

  function optimisticRoleIds(base: string[], addId?: string, removeId?: string): string[] {
    const ids = new Set(base);
    if (addId) ids.add(addId);
    if (removeId) ids.delete(removeId);
    return [...ids];
  }

  async function addRole(roleId: string) {
    if (!target?.member || assigningRole) return;
    const userId = target.user.id;
    const previousMember = target.member;
    const previousIds = previousMember.role_ids ?? previousMember.roles ?? [];
    const nextIds = optimisticRoleIds(previousIds, roleId);
    profile.update((current) =>
      current && current.user.id === userId
        ? {
            ...current,
            member: {
              ...(current.member ?? previousMember),
              role_ids: nextIds,
              roles: nextIds,
            },
          }
        : current,
    );
    assigningRole = true;
    try {
      const updated = await setMemberRoles(userId, [roleId], []);
      if (updated) {
        profile.update((current) =>
          current && current.user.id === userId
            ? {
                ...current,
                member: mergeProfileMember(current.member ?? previousMember, updated),
              }
            : current,
        );
      }
      notify("Role added.");
    } catch (error) {
      profile.update((current) =>
        current && current.user.id === userId
          ? { ...current, member: previousMember }
          : current,
      );
      notify(error instanceof Error ? error.message : String(error));
    } finally {
      assigningRole = false;
    }
  }

  async function removeRole(roleId: string) {
    if (!target?.member || assigningRole) return;
    const userId = target.user.id;
    const previousMember = target.member;
    const previousIds = previousMember.role_ids ?? previousMember.roles ?? [];
    const nextIds = optimisticRoleIds(previousIds, undefined, roleId);
    profile.update((current) =>
      current && current.user.id === userId
        ? {
            ...current,
            member: {
              ...(current.member ?? previousMember),
              role_ids: nextIds,
              roles: nextIds,
            },
          }
        : current,
    );
    assigningRole = true;
    try {
      const updated = await setMemberRoles(userId, [], [roleId]);
      if (updated) {
        profile.update((current) =>
          current && current.user.id === userId
            ? {
                ...current,
                member: mergeProfileMember(current.member ?? previousMember, updated),
              }
            : current,
        );
      }
      notify("Role removed.");
    } catch (error) {
      profile.update((current) =>
        current && current.user.id === userId
          ? { ...current, member: previousMember }
          : current,
      );
      notify(error instanceof Error ? error.message : String(error));
    } finally {
      assigningRole = false;
    }
  }

  function viewFullProfile() {
    if (!target) return;
    openFullProfile(
      target.user,
      target.member,
      $active?.kind === "guild" ? $active.serverId : null,
    );
  }

  function showMore(event: MouseEvent) {
    if (!target) return;
    const userId = target.user.id;
    openContextMenu(event.clientX, event.clientY, [
      {
        label: "Copy user ID",
        icon: "hash",
        run: () => {
          void navigator.clipboard
            .writeText(userId)
            .then(() => notify("User ID copied."))
            .catch(() => notify("Could not copy user ID."));
        },
      },
      { separator: true, label: "" },
      {
        label: "Report user",
        icon: "alert",
        run: () => {
          void (async () => {
            if (!target) return;
            const result = await reportAction(target.user);
            if (!result) return;
            void api
              .reportUser(userId, result.category, result.details)
              .then(() => notify("Report submitted."))
              .catch(() => notify("Could not submit the report."));
          })();
        },
      },
      {
        label: "Block user",
        icon: "lock",
        danger: true,
        run: () => {
          void (async () => {
            if (!target) return;
            const confirmed = await confirmAction({
              title: `Block @${target.user.username}?`,
              body: "They will no longer be able to message you.",
              confirmLabel: "Block",
              danger: true,
            });
            if (!confirmed) return;
            void api
              .blockUser(userId)
              .then(() => notify("User blocked."))
              .catch(() => notify("Could not block this user."));
          })();
        },
      },
    ]);
  }
</script>

{#if target}
  <div class="backdrop" role="presentation" on:click={closeProfile}>
    <div
      class="card"
      bind:this={cardEl}
      role="dialog"
      aria-label={`${target.user.display_name || target.user.username} profile`}
      tabindex="-1"
      style={`left:${position.left}px; top:${position.top}px`}
      on:click|stopPropagation
    >
      <div class="banner" style={primaryRole?.color ? `--role:${primaryRole.color}` : ""}>
        {#if target.user.banner_url}
          <img src={target.user.banner_url} alt="" />
        {/if}
      </div>
      <button class="close" aria-label="Close profile" on:click={closeProfile}>
        <Icon name="close" size={15} />
      </button>
      {#if !isSelf}
        <button class="more" aria-label="More profile actions" on:click={showMore}>
          <Icon name="more" size={15} />
        </button>
      {/if}

      <div class="identity">
        <div class="avatar-wrap">
          <Avatar user={target.user} size={64} status={false} />
        </div>

        <h3 style={primaryRole?.color ? `color:${primaryRole.color}` : ""}>
          {target.member?.nickname || target.user.display_name || target.user.username}
        </h3>
        <p class="handle">@{target.user.username}</p>

        <div class="presence">
          <i
            class:online={status === "online"}
            class:idle={status === "idle"}
            class:dnd={status === "dnd"}
          ></i>
          {statusText(status)}
        </div>
      </div>

      {#if target.member?.is_owner}
        <div class="owner"><Icon name="crown" size={12} /> Server owner</div>
      {/if}

      {#if roles.length > 0 || canManageRoles}
        <div class="section-head">
          <div class="section-label">Roles</div>
          {#if canManageRoles}
            <button
              class="role-add"
              title="Add role"
              aria-label="Add role"
              on:click={openRolePicker}
            >
              <Icon name="plus" size={12} />
            </button>
          {/if}
        </div>

        <div class="roles">
          {#each roles as role}
            <span class="role" style={role.color ? `--role:${role.color}` : ""}>
              {#if role.icon_url}
                <img src={role.icon_url} alt="" />
              {:else if role.icon_emoji}
                <Emoji char={role.icon_emoji} size={13} />
              {:else}
                <i></i>
              {/if}
              <EmojiText text={role.name} size={10} />
              {#if canManageRoles}
                <button
                  class="role-remove"
                  title="Remove role"
                  aria-label={`Remove ${role.name}`}
                  disabled={assigningRole}
                  on:click={() => removeRole(role.id)}
                >
                  <Icon name="close" size={9} />
                </button>
              {/if}
            </span>
          {/each}
        </div>
      {/if}

      <div class="facts">
        {#if joined(target.member?.joined_at)}
          <div><span>Joined</span><b>{joined(target.member?.joined_at)}</b></div>
        {/if}
        <div><span>User ID</span><b class="mono">{target.user.id}</b></div>
      </div>

      {#if $prefs.viewIcons && (target.user.avatar_url || target.user.banner_url)}
        <div class="view-icons">
          {#if target.user.avatar_url}
            <button
              on:click={() =>
                openImage(
                  target.user.avatar_url,
                  `${target.user.display_name || target.user.username}'s avatar`,
                )}
            >
              <Icon name="image" size={13} /> View avatar
            </button>
          {/if}
          {#if target.user.banner_url}
            <button
              on:click={() =>
                openImage(
                  target.user.banner_url,
                  `${target.user.display_name || target.user.username}'s banner`,
                )}
            >
              <Icon name="image" size={13} /> View banner
            </button>
          {/if}
        </div>
      {/if}

      <div class="actions">
        <button on:click={copyId}>
          <Icon name="copy" size={14} />
          {copied ? "Copied" : "Copy ID"}
        </button>
        <button class:primary={!isSelf} on:click={viewFullProfile}>
          <Icon name="user" size={14} />
          Full profile
        </button>
        {#if isSelf}
          <button class="primary" on:click={editSelf}>
            <Icon name="settings" size={14} />
            Settings
          </button>
        {/if}
      </div>
    </div>

    {#if rolePickerOpen && canManageRoles}
      <button
        class="role-picker-backdrop"
        aria-label="Close role picker"
        on:click|stopPropagation={() => (rolePickerOpen = false)}
      ></button>
      <div
        class="role-picker"
        role="dialog"
        aria-label="Assign roles"
        style={`left:${rolePickerPos.left}px; top:${rolePickerPos.top}px`}
        on:click|stopPropagation
      >
        <div class="role-search">
          <Icon name="search" size={13} />
          <input bind:value={roleQuery} placeholder="Search roles" spellcheck="false" />
        </div>
        <div class="role-options">
          {#each filteredRoles as role (role.id)}
            <button
              class:checked={assignedIds.has(role.id)}
              disabled={assigningRole}
              on:click={() => toggleRole(role.id)}
            >
              <span class="role-check">
                {#if assignedIds.has(role.id)}<Icon name="check" size={10} />{/if}
              </span>
              <span
                class="role-dot"
                style={role.color ? `background:${role.color}` : ""}
              ></span>
              <EmojiText text={role.name} size={11.5} />
            </button>
          {:else}
            <p>No roles found.</p>
          {/each}
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 150;
  }
  .card {
    position: fixed;
    width: 300px;
    background: var(--card);
    border: 1px solid var(--line-strong);
    border-radius: 14px;
    box-shadow: var(--shadow);
    overflow: hidden;
    animation: pop 120ms ease-out;
    z-index: 1;
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(4px) scale(0.99);
    }
  }
  .banner {
    height: 68px;
    background: var(--panel-2);
    border-bottom: 2px solid var(--role, var(--accent));
    overflow: hidden;
  }
  .banner img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .identity {
    padding: 0 16px;
    margin-top: -24px;
    position: relative;
  }
  .avatar-wrap {
    width: 72px;
    height: 72px;
    border-radius: 50%;
    background: var(--card);
    padding: 4px;
    display: grid;
    place-items: center;
  }
  .close,
  .more {
    position: absolute;
    top: 8px;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 7px;
    background: rgba(0, 0, 0, 0.28);
    color: #e0e0e8;
    display: grid;
    place-items: center;
    z-index: 2;
  }
  .close {
    right: 8px;
  }
  .more {
    right: 40px;
  }
  .close:hover,
  .more:hover {
    background: rgba(0, 0, 0, 0.48);
    color: white;
  }
  h3 {
    margin: 8px 0 0;
    font-size: 19px;
    letter-spacing: -0.03em;
    line-height: 1.2;
  }
  .handle {
    margin: 1px 0 0;
    font-size: 11.5px;
    color: var(--muted);
  }
  .presence {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 7px;
    font-size: 10.5px;
    color: var(--text-dim);
  }
  .presence i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #6b7280;
  }
  .presence i.online {
    background: var(--green);
  }
  .presence i.idle {
    background: var(--yellow);
  }
  .presence i.dnd {
    background: var(--accent);
  }
  .owner {
    display: flex;
    align-items: center;
    gap: 5px;
    margin: 10px 16px 0;
    font-size: 10px;
    color: var(--yellow);
  }
  .section-label {
    margin: 15px 16px 6px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: 15px 16px 6px;
  }
  .section-head .section-label {
    margin: 0;
  }
  .role-add {
    width: 20px;
    height: 20px;
    flex: none;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 5px;
    background: var(--panel-2);
    color: var(--muted);
    display: grid;
    place-items: center;
    line-height: 0;
  }
  .role-add:hover {
    background: var(--hover);
    color: var(--text);
  }
  .role-picker {
    position: fixed;
    z-index: 160;
    width: 236px;
    padding: 5px;
    border: 1px solid var(--line-strong);
    border-radius: 9px;
    background: var(--panel-2);
    box-shadow: 0 16px 38px rgba(0, 0, 0, 0.46);
  }
  .role-picker-backdrop {
    position: fixed;
    inset: 0;
    z-index: 159;
    border: 0;
    background: transparent;
  }
  .role-search {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 29px;
    padding: 0 7px;
    margin-bottom: 4px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--input);
    color: var(--muted);
  }
  .role-search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text);
    font-size: 11px;
  }
  .role-options {
    max-height: 198px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .role-picker button {
    height: 30px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--text-dim);
    font-size: 11px;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 7px;
    text-align: left;
  }
  .role-picker button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .role-picker button:disabled {
    opacity: 0.55;
  }
  .role-check {
    width: 14px;
    height: 14px;
    flex: none;
    display: grid;
    place-items: center;
    border: 1px solid var(--line-strong);
    border-radius: 4px;
    background: var(--input);
    color: var(--text);
  }
  .role-picker button.checked .role-check {
    background: rgba(var(--accent-rgb), 0.22);
    border-color: rgba(var(--accent-rgb), 0.55);
    color: #ff9a9a;
  }
  .role-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--muted);
    flex: none;
  }
  .role-picker p {
    margin: 7px;
    font-size: 10.5px;
    color: var(--muted);
  }
  .roles {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 0 16px;
  }
  .role {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 21px;
    padding: 0 7px;
    border: 1px solid color-mix(in srgb, var(--role, var(--muted)) 30%, transparent);
    border-radius: 5px;
    background: color-mix(in srgb, var(--role, var(--muted)) 12%, transparent);
    color: color-mix(in srgb, var(--role, var(--text-dim)) 70%, white);
    font-size: 10px;
    font-weight: 600;
  }
  .role i {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--role, var(--muted));
  }
  .role img {
    width: 13px;
    height: 13px;
    object-fit: contain;
    display: block;
  }
  .role-remove {
    width: 12px;
    height: 12px;
    padding: 0;
    border: 0;
    border-radius: 3px;
    background: transparent;
    color: currentColor;
    display: grid;
    place-items: center;
    opacity: 0.45;
    flex: none;
  }
  .role-remove:hover {
    opacity: 1;
    background: rgba(0, 0, 0, 0.22);
  }
  .facts {
    margin: 14px 16px 0;
    border-top: 1px solid var(--line);
    padding-top: 9px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .facts div {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .facts span {
    font-size: 10px;
    color: var(--muted);
  }
  .facts b {
    font-size: 10.5px;
    font-weight: 550;
    color: var(--text-dim);
  }
  .mono {
    font-family: "Cascadia Code", "SF Mono", ui-monospace, monospace;
    font-size: 9px !important;
    color: var(--muted) !important;
  }
  .view-icons {
    display: flex;
    gap: 6px;
    padding: 8px 16px 0;
  }
  .view-icons button {
    flex: 1;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    font-size: 10px;
  }
  .view-icons button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .actions {
    display: flex;
    gap: 6px;
    padding: 14px 16px 16px;
  }
  .actions button {
    flex: 1;
    height: 32px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel-2);
    color: var(--text-dim);
    font-size: 10.5px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }
  .actions button:hover {
    background: var(--panel-3);
    color: var(--text);
  }
  .actions button.primary {
    background: var(--accent);
    border-color: transparent;
    color: white;
  }
</style>
