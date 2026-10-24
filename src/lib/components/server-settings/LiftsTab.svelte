<script lang="ts">
  import * as api from "../../api";
  import type { LiftsInfo, SavedMediaPage, Server, User } from "../../types";
  import { notify } from "../../workspace";
  import Avatar from "../Avatar.svelte";
  import Icon from "../Icon.svelte";

  export let server: Server;

  let lifts: LiftsInfo | null = null;
  let supporters: Array<User & { user?: User }> = [];
  let saved: SavedMediaPage | null = null;
  let loading = true;
  let error = "";
  let lastServerId = "";

  $: if (server.id !== lastServerId) {
    lastServerId = server.id;
    void load();
  }

  $: levelPercent = lifts
    ? lifts.next_level_at > 0
      ? Math.min(100, Math.round((lifts.count / lifts.next_level_at) * 100))
      : 100
    : 0;
  $: remaining = lifts ? Math.max(0, lifts.next_level_at - lifts.count) : 0;
  $: storagePercent =
    saved && saved.budget && saved.budget > 0
      ? Math.min(100, Math.round(((saved.used ?? 0) / saved.budget) * 100))
      : 0;

  async function load() {
    loading = true;
    error = "";
    try {
      const [liftInfo, supporterList, storage] = await Promise.all([
        api.fetchLifts(server.id),
        api.fetchLiftSupporters(server.id).catch(() => []),
        api.fetchSavedMedia(server.id).catch(() => ({ items: [], count: 0, used: 0, budget: 0 })),
      ]);
      lifts = liftInfo;
      supporters = supporterList;
      saved = storage;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
      notify(error);
    } finally {
      loading = false;
    }
  }

  function bytes(value?: number | null) {
    if (!value || value <= 0) return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"];
    let size = value;
    let unit = 0;
    while (size >= 1024 && unit < units.length - 1) {
      size /= 1024;
      unit += 1;
    }
    return `${size >= 100 ? size.toFixed(0) : size.toFixed(1)} ${units[unit]}`;
  }

  function supporterName(user: User) {
    return user.display_name || user.username;
  }

  function supporterUser(entry: User & { user?: User }): User {
    return entry.user ?? entry;
  }
</script>

<div class="lifts">
  <div class="head">
    <div>
      <h3>Lifts & Storage</h3>
      <p>Server boosts unlock higher upload limits, storage, and stream quality.</p>
    </div>
  </div>

  {#if error}
    <p class="denied">{error}</p>
  {:else if loading || !lifts}
    <p class="empty">Loading lifts…</p>
  {:else}
    <div class="scroll">
      <div class="summary">
        <div class="level-block">
          <div class="level-ring">
            <b>{lifts.level}</b>
            <span>Level</span>
          </div>
          <div class="level-copy">
            <b>Server Level {lifts.level}</b>
            <p>
              {lifts.count} lift{lifts.count === 1 ? "" : "s"} · {remaining}
              {remaining === 1 ? "lift" : "lifts"} until Level {lifts.level + 1}
            </p>
            <div class="progress"><span style={`width:${levelPercent}%`}></span></div>
          </div>
        </div>
        <div class="stat-grid">
          <div class="stat">
            <span>Supporters</span>
            <b>{supporters.length || lifts.count}</b>
          </div>
          <div class="stat">
            <span>Max upload</span>
            <b>{bytes(lifts.max_upload_bytes)}</b>
          </div>
          <div class="stat">
            <span>Saved media budget</span>
            <b>{bytes(lifts.saved_budget_bytes)}</b>
          </div>
          <div class="stat">
            <span>Retention</span>
            <b>{lifts.retention_multiplier}×</b>
          </div>
        </div>
      </div>

      <div class="section">
        <div class="section-head">
          <div>
            <b>Saved media</b>
            <p>Media saved from this server counts toward the shared budget.</p>
          </div>
          <span class="storage-count">
            {saved?.count ?? 0} item{(saved?.count ?? 0) === 1 ? "" : "s"}
          </span>
        </div>
        <div class="storage-row">
          <div class="storage-bar"><span style={`width:${storagePercent}%`}></span></div>
          <span>{bytes(saved?.used)} of {bytes(saved?.budget ?? lifts.saved_budget_bytes)}</span>
        </div>
      </div>

      <div class="section">
        <div class="section-head">
          <div>
            <b>Supporter role</b>
            <p>Supporters receive the server's supporter role automatically.</p>
          </div>
          <span class="storage-count">{supporters.length} supporter{supporters.length === 1 ? "" : "s"}</span>
        </div>
        {#if supporters.length > 0}
          <div class="supporters">
            {#each supporters as supporter (supporter.id)}
              {@const person = supporterUser(supporter)}
              <div class="supporter">
                <Avatar user={person} size={28} />
                <div>
                  <b>{supporterName(person)}</b>
                  <small>@{person.username}</small>
                </div>
                <Icon name="gift" size={14} />
              </div>
            {/each}
          </div>
        {:else}
          <p class="inline-empty">No supporters yet. Lifts can be purchased from Flackle.</p>
        {/if}
      </div>

      <div class="section">
        <div class="section-head">
          <div>
            <b>Level rewards</b>
            <p>Each level increases the server's limits.</p>
          </div>
        </div>
        <div class="level-table">
          <div class="level-head">
            <span>Level</span>
            <span>Lifts</span>
            <span>Stream</span>
            <span>Upload</span>
            <span>Saved budget</span>
            <span>Retention</span>
          </div>
          {#each lifts.levels as level (level.level)}
            <div class="level-row" class:current={level.level === lifts.level}>
              <span class="level-cell">Level {level.level}</span>
              <span>{level.lifts_needed}</span>
              <span>
                {Object.entries(level.stream_fps ?? {})
                  .map(([quality, fps]) => `${quality} ${fps}fps`)
                  .join(", ") || "—"}
              </span>
              <span>{bytes(level.max_upload_bytes)}</span>
              <span>{bytes(level.saved_budget_bytes)}</span>
              <span>{level.retention_multiplier}×</span>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .lifts {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 16px 18px 0;
  }
  .head {
    flex: none;
    margin-bottom: 12px;
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
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding-bottom: 20px;
  }
  .summary {
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--panel-2);
    padding: 14px;
  }
  .level-block {
    display: flex;
    align-items: center;
    gap: 13px;
  }
  .level-ring {
    width: 58px;
    height: 58px;
    flex: none;
    border-radius: 50%;
    border: 3px solid rgba(var(--accent-rgb), 0.5);
    background: var(--accent-dim);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    line-height: 1;
  }
  .level-ring b {
    font-size: 20px;
    color: var(--accent);
  }
  .level-ring span {
    margin-top: 2px;
    font-size: 8px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--muted);
  }
  .level-copy {
    flex: 1;
    min-width: 0;
  }
  .level-copy b {
    font-size: 12.5px;
    font-weight: 650;
  }
  .level-copy p {
    margin: 2px 0 6px;
    color: var(--muted);
    font-size: 9.5px;
  }
  .progress,
  .storage-bar {
    height: 5px;
    border-radius: 99px;
    background: var(--panel-3);
    overflow: hidden;
  }
  .progress span,
  .storage-bar span {
    display: block;
    height: 100%;
    background: var(--accent);
    border-radius: inherit;
    transition: width 180ms ease;
  }
  .stat-grid {
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid var(--line);
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
  }
  .stat span {
    display: block;
    font-size: 8.5px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
  }
  .stat b {
    display: block;
    margin-top: 2px;
    font-size: 11.5px;
    font-weight: 650;
  }
  .section {
    margin-top: 12px;
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 12px 14px;
  }
  .section-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 9px;
  }
  .section-head b {
    font-size: 11.5px;
    font-weight: 650;
  }
  .section-head p {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 9.5px;
  }
  .storage-count {
    flex: none;
    font-size: 9px;
    color: var(--muted);
    border: 1px solid var(--line);
    border-radius: 99px;
    padding: 2px 7px;
  }
  .storage-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .storage-bar {
    flex: 1;
  }
  .storage-row > span {
    flex: none;
    font-size: 9.5px;
    color: var(--muted);
  }
  .supporters {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
  }
  .supporter {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 5px 7px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel-2);
    color: var(--muted);
  }
  .supporter > div {
    min-width: 0;
    line-height: 1.2;
  }
  .supporter b {
    display: block;
    font-size: 10.5px;
    font-weight: 600;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .supporter small {
    display: block;
    margin-top: 1px;
    font-size: 8.5px;
    color: var(--muted);
  }
  .supporter :global(svg) {
    margin-left: auto;
    flex: none;
    color: var(--accent);
  }
  .inline-empty {
    margin: 0;
    color: var(--muted);
    font-size: 9.5px;
    padding: 4px 0;
  }
  .level-table {
    border: 1px solid var(--line);
    border-radius: 8px;
    overflow: hidden;
  }
  .level-head,
  .level-row {
    display: grid;
    grid-template-columns: 0.62fr 0.42fr 1.15fr 0.72fr 0.82fr 0.6fr;
    gap: 6px;
    align-items: center;
    padding: 6px 8px;
  }
  .level-head {
    background: var(--panel-2);
    border-bottom: 1px solid var(--line);
    font-size: 8px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
  }
  .level-row {
    border-bottom: 1px solid var(--line);
    color: var(--text-dim);
    font-size: 9.5px;
  }
  .level-row:last-child {
    border-bottom: 0;
  }
  .level-row.current {
    background: var(--accent-dim);
  }
  .level-cell {
    font-weight: 600;
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
