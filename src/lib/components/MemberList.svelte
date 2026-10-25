<script lang="ts">
  import { openProfile } from "../profile";
  import { showUserMenu } from "../user-menu";
  import { prefs } from "../preferences";
  import type { Member, Role } from "../types";
  import { memberSections, memberTotal, members, presenceStatuses, serverRoles, modal, notify } from "../workspace";
  import Avatar from "./Avatar.svelte";
  import Emoji from "./Emoji.svelte";
  import EmojiText from "./EmojiText.svelte";
  import Icon from "./Icon.svelte";

  let filter = "";

  $: normalized = filter.trim().toLowerCase();
  $: roleList = $serverRoles;
  $: presence = $presenceStatuses;
  $: total = $memberTotal || $members.length;
  $: roleSections = $memberSections.filter((section) => !!section.role_id);
  $: groupedIds = new Set(roleSections.flatMap((section) => section.member_ids));
  $: remainder = $members.filter((member) => !groupedIds.has(member.user.id));
  $: online = remainder
    .filter((member) => isOnline(member, presence))
    .sort((a, b) => compareMembers(a, b, roleList));
  $: offline = remainder
    .filter((member) => !isOnline(member, presence))
    .sort((a, b) => compareMembers(a, b, roleList));
  $: matches = normalized
    ? $members.filter((member) => memberName(member).toLowerCase().includes(normalized))
    : [];

  function isOnline(member: Member, presenceMap: Record<string, string>): boolean {
    const status = presenceMap[member.user.id] ?? member.presence ?? member.user.status;
    return status === "online" || status === "idle" || status === "dnd";
  }

  function statusOf(member: Member): string {
    return presence[member.user.id] ?? member.presence ?? member.user.status ?? "offline";
  }

  function memberName(member: Member): string {
    return member.nickname || member.user.display_name || member.user.username;
  }

  function roleFor(member: Member): Role | undefined {
    const ids = member.role_ids ?? member.roles ?? [];
    return roleList
      .filter((role) => !role.is_everyone && ids.includes(role.id))
      .sort((a, b) => (b.position ?? 0) - (a.position ?? 0))[0];
  }

  function compareMembers(a: Member, b: Member, roles: Role[]): number {
    if (a.is_owner !== b.is_owner) return a.is_owner ? -1 : 1;
    const rank = (roleFor(b)?.position ?? -1) - (roleFor(a)?.position ?? -1);
    if (rank !== 0) return rank;
    return memberName(a).localeCompare(memberName(b));
  }

  function statusText(member: Member): string {
    const status = statusOf(member);
    if (status === "idle") return "Idle";
    if (status === "dnd") return "Do not disturb";
    if (status === "online") return "Online";
    return "Offline";
  }

  function open(event: MouseEvent, member: Member) {
    openProfile(member.user, member, event.currentTarget as HTMLElement | null);
  }

  function showMemberMenu(event: MouseEvent, member: Member) {
    showUserMenu(event, member.user, member);
  }

  function membersInSection(ids: string[]): Member[] {
    return ids
      .map((id) => $members.find((member) => member.user.id === id))
      .filter((member): member is Member => !!member);
  }

  function sectionRole(roleId?: string | null): Role | undefined {
    return roleId ? roleList.find((role) => role.id === roleId) : undefined;
  }
</script>

<aside class="members">
  <header>
    <div class="title">
      <b>Members</b>
      <span>{total}</span>
    </div>
    <div class="actions">
      <button
        class="icon-btn"
        aria-label="Join a server"
        title="Join a server"
        on:click={() => modal.set("add-server")}
      >
        <Icon name="plus" size={17} />
      </button>
    </div>
  </header>

  <div class="filter">
    <Icon name="search" size={13} />
    <input bind:value={filter} placeholder="Find a member" spellcheck="false" />
  </div>

  <div class="member-scroll">
    {#if normalized}
      <div class="group-label">Results <span>{matches.length}</span></div>
      {#each matches as member (member.user.id)}
        <button class="member" title={statusText(member)} on:click={(event) => open(event, member)}
          on:contextmenu|preventDefault={(event) => showMemberMenu(event, member)}>
          <Avatar user={{ ...member.user, status: statusOf(member) }} size={32} />
          <span class="member-name" style={roleFor(member)?.color ? `color:${roleFor(member)?.color}` : ""}>
            {memberName(member)}
            {#if $prefs.showMeYourName && member.user.username !== memberName(member)}
              <span class="user-tag">@{member.user.username}</span>
            {/if}
          </span>
        </button>
      {/each}
      {#if matches.length === 0}<p class="empty">No members found.</p>{/if}
    {:else}
      {#each roleSections as section}
        {@const sectionMembers = membersInSection(section.member_ids)}
        {#if sectionMembers.length > 0}
          <div
            class="group-label role-label"
            style={sectionRole(section.role_id)?.color ? `color:${sectionRole(section.role_id)?.color}` : ""}
          >
            {#if sectionRole(section.role_id)?.icon_url}
              <img class="role-icon" src={sectionRole(section.role_id)?.icon_url} alt="" />
            {:else if sectionRole(section.role_id)?.icon_emoji}
              <Emoji char={sectionRole(section.role_id)?.icon_emoji ?? ""} size={13} />
            {/if}
            <EmojiText text={section.label} size={9.5} /> <span>{sectionMembers.length}</span>
          </div>
          {#each sectionMembers as member (member.user.id)}
            <button class="member" title={statusText(member)} on:click={(event) => open(event, member)}
          on:contextmenu|preventDefault={(event) => showMemberMenu(event, member)}>
              <Avatar user={{ ...member.user, status: statusOf(member) }} size={32} />
              <span
                class="member-name"
                style={roleFor(member)?.color ? `color:${roleFor(member)?.color}` : ""}
              >
                {memberName(member)}
                {#if $prefs.showMeYourName && member.user.username !== memberName(member)}
                  <span class="user-tag">@{member.user.username}</span>
                {/if}
                {#if member.is_owner}<span class="owner" title="Server owner"><Icon name="crown" size={10} /></span>{/if}
              </span>
            </button>
          {/each}
        {/if}
      {/each}

      {#if online.length > 0}
        <div class="group-label">Online <span>{online.length}</span></div>
        {#each online as member (member.user.id)}
          <button class="member" title={statusText(member)} on:click={(event) => open(event, member)}
          on:contextmenu|preventDefault={(event) => showMemberMenu(event, member)}>
            <Avatar user={{ ...member.user, status: statusOf(member) }} size={32} />
            <span class="member-name" style={roleFor(member)?.color ? `color:${roleFor(member)?.color}` : ""}>
              {memberName(member)}
              {#if $prefs.showMeYourName && member.user.username !== memberName(member)}
                <span class="user-tag">@{member.user.username}</span>
              {/if}
              {#if member.is_owner}<span class="owner" title="Server owner"><Icon name="crown" size={10} /></span>{/if}
            </span>
          </button>
        {/each}
      {/if}

      {#if offline.length > 0}
        <div class="group-label">Offline <span>{offline.length}</span></div>
        {#each offline as member (member.user.id)}
          <button class="member offline" title="Offline" on:click={(event) => open(event, member)}
          on:contextmenu|preventDefault={(event) => showMemberMenu(event, member)}>
            <Avatar user={{ ...member.user, status: "offline" }} size={32} />
            <span class="member-name">
              {memberName(member)}
              {#if $prefs.showMeYourName && member.user.username !== memberName(member)}
                <span class="user-tag">@{member.user.username}</span>
              {/if}
            </span>
          </button>
        {/each}
      {/if}

      {#if $members.length === 0}
        <p class="empty">No members to show.</p>
      {/if}
    {/if}
  </div>
</aside>

<style>
  .members {
    width: 240px;
    flex: none;
    background: var(--sidebar);
    border-left: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  header {
    height: 58px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px 0 16px;
    border-bottom: 1px solid var(--line);
  }
  .title {
    display: flex;
    align-items: baseline;
    gap: 5px;
  }
  .title b {
    font-family: "Satoshi", "Manrope", system-ui, sans-serif;
    font-size: 13px;
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  .title span {
    font-size: 10px;
    color: var(--muted);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 1px;
  }
  .filter {
    flex: none;
    margin: 10px 10px 2px;
    height: 32px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--input);
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 9px;
    color: var(--muted);
  }
  .filter input {
    flex: 1;
    border: 0;
    background: transparent;
    outline: none;
    font-size: 12px;
    color: var(--text);
  }
  .filter input::placeholder {
    color: #5a5a64;
  }
  .member-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 2px 8px 16px;
    min-height: 0;
  }
  .group-label {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 14px 8px 5px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.01em;
    color: var(--muted);
  }
  .group-label span {
    font-weight: 500;
    opacity: 0.72;
  }
  .role-label {
    text-transform: none;
    letter-spacing: 0.01em;
    font-size: 10px;
    font-weight: 700;
  }
  .role-icon {
    width: 13px;
    height: 13px;
    object-fit: contain;
    display: block;
    flex: none;
  }
  .member {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 42px;
    padding: 0 7px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    text-align: left;
    color: var(--text);
  }
  .member:hover,
  .member:focus-visible {
    background: var(--hover);
    outline: none;
  }
  .member.offline {
    opacity: 0.5;
  }
  .member-name {
    font-size: 12.5px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .owner {
    color: var(--yellow);
    display: flex;
    align-items: center;
    flex: none;
  }
  .user-tag {
    font-size: 10px;
    font-weight: 500;
    color: #737a8b;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .empty {
    font-size: 11.5px;
    color: var(--muted);
    padding: 14px 8px;
  }
</style>
