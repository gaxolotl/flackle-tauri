<script lang="ts">
  import { afterUpdate, onMount, tick } from "svelte";
  import * as api from "../api";
  import { confirmAction, reportAction } from "../dialog";
  import { openContextMenu } from "../context-menu";
  import { hasPermission } from "../permissions";
  import { closeProfile, profile } from "../profile";
  import {
    active,
    me,
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
  $: availableRoles = $serverRoles
    .filter((role) => !role.is_everyone && !assignedIds.has(role.id))
    .sort((a, b) => (b.position ?? 0) - (a.position ?? 0));
  $: if (target?.user.id !== lastTargetId) {
    lastTargetId = target?.user.id ?? "";
    rolePickerOpen = false;
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

  async function addRole(roleId: string) {
    if (!target?.member || assigningRole) return;
    assigningRole = true;
    try {
      const updated = await setMemberRoles(target.user.id, [roleId], []);
      if (updated) {
        profile.update((current) =>
          current
            ? { ...current, member: { ...(current.member ?? target.member!), ...updated } }
            : current,
        );
      }
      rolePickerOpen = false;
      notify("Role added.");
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    } finally {
      assigningRole = false;
    }
  }

  async function removeRole(roleId: string) {
    if (!target?.member || assigningRole) return;
    assigningRole = true;
    try {
      const updated = await setMemberRoles(target.user.id, [], [roleId]);
      if (updated) {
        profile.update((current) =>
          current
            ? { ...current, member: { ...(current.member ?? target.member!), ...updated } }
            : current,
        );
      }
      notify("Role removed.");
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    } finally {
      assigningRole = false;
    }
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
              on:click={() => (rolePickerOpen = !rolePickerOpen)}
            >
              <Icon name="plus" size={12} />
            </button>
          {/if}
        </div>

        {#if rolePickerOpen && canManageRoles}
          <div class="role-picker">
            {#if availableRoles.length > 0}
              {#each availableRoles as role}
                <button disabled={assigningRole} on:click={() => addRole(role.id)}>
                  <span style={role.color ? `background:${role.color}` : ""}></span>
                  {role.name}
                </button>
              {/each}
            {:else}
              <p>No roles left to add.</p>
            {/if}
          </div>
        {/if}

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

      <div class="actions">
        <button on:click={copyId}>
          <Icon name="copy" size={14} />
          {copied ? "Copied" : "Copy ID"}
        </button>
        {#if isSelf}
          <button class="primary" on:click={editSelf}>
            <Icon name="settings" size={14} />
            Settings
          </button>
        {/if}
      </div>
    </div>
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
    border: 1px solid var(--line);
    border-radius: 5px;
    background: var(--panel-2);
    color: var(--muted);
    display: grid;
    place-items: center;
  }
  .role-add:hover {
    background: var(--hover);
    color: var(--text);
  }
  .role-picker {
    margin: 0 16px 7px;
    max-height: 132px;
    overflow-y: auto;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel-2);
    padding: 3px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .role-picker button {
    height: 27px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--text-dim);
    font-size: 10.5px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 7px;
    text-align: left;
  }
  .role-picker button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .role-picker button span {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--muted);
    flex: none;
  }
  .role-picker p {
    margin: 6px;
    font-size: 10px;
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
