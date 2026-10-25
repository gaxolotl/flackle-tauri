<script lang="ts">
  import { onMount, tick } from "svelte";
  import * as api from "../api";
  import { imageViewer, openImage } from "../image-viewer";
  import { closeFullProfile, fullProfile } from "../profile";
  import type { BentoCard, BentoGame, ProfileBlock, Role, UserCard } from "../types";
  import { serverRoles } from "../workspace";
  import Avatar from "./Avatar.svelte";
  import Emoji from "./Emoji.svelte";
  import EmojiText from "./EmojiText.svelte";
  import Icon from "./Icon.svelte";

  let card: UserCard | null = null;
  let bento: BentoCard[] = [];
  let loading = true;
  let lastKey = "";
  let dialogEl: HTMLDivElement | undefined;
  let now = new Date();
  let ticker: ReturnType<typeof setInterval> | undefined;

  $: target = $fullProfile;
  $: key = target ? `${target.user.username}:${target.serverId ?? ""}` : "";
  $: user = card?.user ?? target?.user ?? null;
  $: member = card?.member ?? target?.member ?? null;
  $: displayName = user?.display_name || user?.username || "Profile";
  $: blocks = (card?.blocks ?? [])
    .slice()
    .sort((a, b) => (a.order ?? 0) - (b.order ?? 0));
  $: roleIds = member?.role_ids ?? member?.roles ?? [];
  $: roles = $serverRoles
    .filter((role) => !role.is_everyone && roleIds.includes(role.id))
    .sort((a, b) => (b.position ?? 0) - (a.position ?? 0));
  $: primaryRole = roles.find((role) => role.color) ?? null;
  $: bentoCards = bento
    .slice()
    .sort((a, b) => (a.row ?? 0) - (b.row ?? 0) || (a.col ?? 0) - (b.col ?? 0));

  $: if (!key) {
    lastKey = "";
    loading = true;
    card = null;
    bento = [];
  } else if (key !== lastKey) {
    lastKey = key;
    loading = true;
    card = null;
    bento = [];
    void load();
    void tick().then(() => dialogEl?.focus());
  }

  async function load() {
    const current = target;
    if (!current) return;
    const requestKey = key;
    try {
      const [profileCard, bentoCards] = await Promise.all([
        api
          .fetchUserCard(current.user.username, current.serverId ?? null)
          .catch(() => null),
        api.fetchUserBento(current.user.username).catch(() => [] as BentoCard[]),
      ]);
      if (key !== requestKey) return;
      card =
        profileCard ?? {
          user: current.user,
          member: current.member ?? null,
          blocks: [],
          socials: [],
        };
      bento = bentoCards;
    } catch {
      if (key === requestKey) {
        card = {
          user: current.user,
          member: current.member ?? null,
          blocks: [],
          socials: [],
        };
        bento = [];
      }
    } finally {
      if (key === requestKey) loading = false;
    }
  }

  function roleStyle(role: Role) {
    return role.color ? `--role-color:${role.color}` : "";
  }

  function joined(value?: string | null): string {
    if (!value) return "";
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return "";
    return date.toLocaleDateString([], { month: "long", year: "numeric" });
  }

  function socialUrl(block: ProfileBlock): string | null {
    if (block.url) return block.url;
    const username = block.username?.trim();
    if (!username) return null;
    if (/^https?:\/\//i.test(username)) return username;
    switch ((block.platform ?? "").toLowerCase()) {
      case "github":
        return `https://github.com/${username}`;
      case "x":
      case "twitter":
        return `https://x.com/${username}`;
      case "instagram":
        return `https://instagram.com/${username}`;
      case "youtube":
        return `https://youtube.com/@${username}`;
      case "tiktok":
        return `https://tiktok.com/@${username}`;
      case "twitch":
        return `https://twitch.tv/${username}`;
      case "spotify":
        return `https://open.spotify.com/user/${username}`;
      case "reddit":
        return `https://reddit.com/user/${username}`;
      case "steam":
        return `https://steamcommunity.com/id/${username}`;
      default:
        return null;
    }
  }

  function copyBlock(block: ProfileBlock): string {
    if (block.type === "social") {
      if (block.content?.trim()) return block.content;
      return block.username ? `@${block.username}` : "Social";
    }
    if (block.content?.trim()) return block.content;
    if (block.url) {
      try {
        return new URL(block.url).hostname;
      } catch {
        return block.url;
      }
    }
    return block.type;
  }

  function openLink(block: ProfileBlock) {
    const url = socialUrl(block);
    if (url) void api.openExternal(url);
  }

  function imageBlock(block: ProfileBlock): string | null {
    if (block.type === "image") {
      return block.image_url ?? block.url ?? null;
    }
    const direct = typeof block.image_url === "string" ? block.image_url : null;
    if (direct) return direct;
    const url = typeof block.url === "string" ? block.url : null;
    return url && /\.(png|jpe?g|gif|webp|avif|svg)(\?|$)/i.test(url) ? url : null;
  }

  function bentoGames(entry: BentoCard): BentoGame[] {
    return Array.isArray(entry.data?.games) ? entry.data.games : [];
  }

  function bentoImage(entry: BentoCard): string | null {
    return typeof entry.data?.url === "string" ? entry.data.url : null;
  }

  function bentoLocation(entry: BentoCard): string {
    return typeof entry.data?.location === "string" ? entry.data.location : "Weather";
  }

  function bentoTimezone(entry: BentoCard): string {
    return typeof entry.data?.timezone === "string"
      ? entry.data.timezone.replace(/_/g, " ")
      : "";
  }

  function bentoTime(entry: BentoCard, at: Date): string {
    const timezone =
      typeof entry.data?.timezone === "string" ? entry.data.timezone : undefined;
    try {
      return new Intl.DateTimeFormat(undefined, {
        hour: "2-digit",
        minute: "2-digit",
        timeZone: timezone,
      }).format(at);
    } catch {
      return new Intl.DateTimeFormat(undefined, {
        hour: "2-digit",
        minute: "2-digit",
      }).format(at);
    }
  }

  function bentoStyle(entry: BentoCard): string {
    const width = Math.max(1, Math.min(16, entry.width ?? 4));
    const height = Math.max(1, Math.min(5, entry.height ?? 2));
    const column = Math.max(0, Math.min(15, entry.col ?? 0)) + 1;
    const row = Math.max(0, entry.row ?? 0) + 1;
    return `--bento-column:${column} / span ${width}; --bento-row:${row} / span ${height}`;
  }

  function gameUrl(game: BentoGame): string | null {
    return game.gameId ? `https://www.igdb.com/games/${game.gameId}` : null;
  }

  function openGame(game: BentoGame) {
    const url = gameUrl(game);
    if (url) void api.openExternal(url);
  }

  function close() {
    closeFullProfile();
  }

  function onBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) close();
  }

  onMount(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && $fullProfile && !$imageViewer) close();
    };
    window.addEventListener("keydown", onKey);
    ticker = setInterval(() => {
      now = new Date();
    }, 30_000);
    return () => {
      window.removeEventListener("keydown", onKey);
      if (ticker) clearInterval(ticker);
    };
  });
</script>

{#if target && user}
  <div
    class="backdrop"
    role="presentation"
    on:click={onBackdropClick}
    on:keydown={(event) => {
      if (event.key === "Escape") close();
    }}
  >
    <div
      class="profile"
      bind:this={dialogEl}
      role="dialog"
      aria-modal="true"
      aria-label={`${displayName} full profile`}
      tabindex="-1"
      style={primaryRole?.color ? `--role:${primaryRole.color}` : ""}
    >
      <button class="close" aria-label="Close profile" on:click={close}>
        <Icon name="close" size={17} />
      </button>

      <div class="scroller">
        <div class="banner">
        {#if user.banner_url}
          <button
            class="banner-button"
            title="View banner"
            aria-label={`View ${displayName}'s banner`}
            on:click={() => openImage(user.banner_url, `${displayName}'s banner`)}
          >
            <img src={user.banner_url} alt="" />
          </button>
        {/if}
      </div>

      <div class="intro">
        {#if user.avatar_url}
          <button
            class="avatar-button"
            title="View avatar"
            aria-label={`View ${displayName}'s avatar`}
            on:click={() => openImage(user.avatar_url, `${displayName}'s avatar`)}
          >
            <Avatar user={user} size={80} status={false} />
          </button>
        {:else}
          <div class="avatar-button static">
            <Avatar user={user} size={80} status={false} />
          </div>
        {/if}
        <div class="who">
          <h2 style={primaryRole?.color ? `color:${primaryRole.color}` : ""}>
            <EmojiText text={displayName} size={24} />
          </h2>
          <p>@{user.username}</p>
          {#if member?.is_owner}
            <div class="pills">
              <span class="pill owner"><Icon name="crown" size={11} /> Server owner</span>
            </div>
          {/if}
        </div>
      </div>

      {#if loading}
        <div class="skeleton" role="status" aria-label="Loading full profile">
          <span class="sk-line sk-heading"></span>
          <span class="sk-line"></span>
          <span class="sk-line short"></span>
          <div class="sk-grid">
            <span class="sk-card wide"></span>
            <span class="sk-card"></span>
            <span class="sk-card wide"></span>
            <span class="sk-card"></span>
          </div>
        </div>
      {:else}
        <div class="content">
          {#if roles.length > 0}
            <section>
              <h3>Roles</h3>
              <div class="roles">
                {#each roles as role}
                  <span class="role" style={roleStyle(role)}>
                    {#if role.icon_url}
                      <img src={role.icon_url} alt="" />
                    {:else if role.icon_emoji}
                      <Emoji char={role.icon_emoji} size={13} />
                    {:else}
                      <i></i>
                    {/if}
                    <EmojiText text={role.name} size={10} />
                  </span>
                {/each}
              </div>
            </section>
          {/if}

          <section>
            <h3>About Me</h3>
            {#if blocks.length === 0}
              <p class="empty">No about me yet.</p>
            {:else}
              <div class="blocks">
                {#each blocks as block (block.id ?? `${block.type}-${block.order ?? 0}`)}
                  {#if block.type === "header"}
                    <div class="block-header" class:small={(block.level ?? 2) > 2}>
                      <EmojiText text={block.content ?? ""} size={(block.level ?? 2) > 2 ? 13 : 18} />
                    </div>
                  {:else if block.type === "social" || block.type === "link"}
                    {@const url = socialUrl(block)}
                    {#if url}
                      <button class="block-link" on:click={() => openLink(block)}>
                        <span class="block-icon"><Icon name={block.type === "social" ? "at" : "link"} size={14} /></span>
                        <span class="block-copy">
                          <b><EmojiText text={copyBlock(block)} size={12} /></b>
                          {#if block.type === "social" && block.platform}
                            <small>{block.platform}</small>
                          {:else if block.url}
                            <small>{block.url}</small>
                          {/if}
                        </span>
                        <Icon name="link" size={13} />
                      </button>
                    {:else}
                      <div class="block-link static">
                        <span class="block-icon"><Icon name={block.type === "social" ? "at" : "link"} size={14} /></span>
                        <span class="block-copy">
                          <b><EmojiText text={copyBlock(block)} size={12} /></b>
                          {#if block.username}<small>@{block.username}</small>{/if}
                        </span>
                      </div>
                    {/if}
                  {:else if imageBlock(block)}
                    <button
                      class="block-image"
                      title="View image"
                      aria-label="View profile image"
                      on:click={() => openImage(imageBlock(block), "Profile image")}
                    >
                      <img src={imageBlock(block) ?? ""} alt="" loading="lazy" />
                    </button>
                  {:else if block.content}
                    <p class="block-text"><EmojiText text={block.content} size={12.5} /></p>
                  {/if}
                {/each}
              </div>
            {/if}
          </section>

          {#if bentoCards.length > 0}
            <section>
              <h3>Bento Grid</h3>
              <div class="bento">
                {#each bentoCards as entry (entry.id ?? `${entry.type}-${entry.row ?? 0}-${entry.col ?? 0}`)}
                  <div class="bento-card" style={bentoStyle(entry)}>
                    {#if entry.type === "time"}
                      <div class="bento-time">
                        <b>{bentoTime(entry, now)}</b>
                        <span>{bentoTimezone(entry)}</span>
                      </div>
                    {:else if entry.type === "weather"}
                      <div class="bento-weather">
                        <span class="bento-icon"><Icon name="at" size={17} /></span>
                        <div>
                          <b>{bentoLocation(entry)}</b>
                          <span>Weather</span>
                        </div>
                      </div>
                    {:else if entry.type === "game" && bentoGames(entry).length > 0}
                      <span class="bento-label">{entry.data?.label || "Favourite games"}</span>
                      <div class="games">
                        {#each bentoGames(entry).slice(0, 4) as game}
                          <button class="game" on:click={() => openGame(game)}>
                            {#if game.cover}
                              <img src={game.cover} alt="" loading="lazy" />
                            {/if}
                            <span>{game.title ?? "Game"}</span>
                          </button>
                        {/each}
                      </div>
                    {:else if bentoImage(entry)}
                      <button
                        class="bento-image"
                        title="View image"
                        aria-label="View bento image"
                        on:click={() => openImage(bentoImage(entry), "Bento image")}
                      >
                        <img src={bentoImage(entry) ?? ""} alt="" loading="lazy" />
                      </button>
                    {:else if entry.data?.label}
                      <span class="bento-label">{entry.data.label}</span>
                    {:else}
                      <span class="bento-label">{entry.type}</span>
                    {/if}
                  </div>
                {/each}
              </div>
            </section>
          {/if}

          <section class="facts">
            {#if joined(member?.joined_at)}
              <div><span>Joined</span><b>{joined(member?.joined_at)}</b></div>
            {/if}
            <div><span>User ID</span><b class="mono">{user.id}</b></div>
          </section>
        </div>
      {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 230;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(4, 4, 7, 0.74);
    backdrop-filter: blur(7px);
    animation: fade-in 120ms ease-out;
  }
  @keyframes fade-in {
    from {
      opacity: 0;
    }
  }
  .profile {
    position: relative;
    width: min(820px, 100%);
    max-height: calc(100vh - 48px);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid var(--line-strong);
    border-radius: 16px;
    background: var(--card);
    box-shadow: var(--shadow);
    outline: none;
    animation: profile-in 140ms ease-out;
  }
  .scroller {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
  }
  @keyframes profile-in {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.995);
    }
  }
  .close {
    position: absolute;
    top: 12px;
    right: 12px;
    z-index: 3;
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: 8px;
    background: rgba(0, 0, 0, 0.36);
    color: #f0f0f3;
    display: grid;
    place-items: center;
  }
  .close:hover {
    background: rgba(0, 0, 0, 0.58);
    color: #fff;
  }
  .banner {
    height: 158px;
    overflow: hidden;
    border-bottom: 3px solid var(--role, var(--accent));
    background: var(--panel-2);
  }
  .banner-button {
    width: 100%;
    height: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    display: block;
    cursor: zoom-in;
  }
  .banner img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .intro {
    position: relative;
    display: flex;
    align-items: flex-end;
    gap: 16px;
    padding: 0 24px;
    margin-top: -40px;
  }
  .avatar-button {
    width: 88px;
    height: 88px;
    flex: none;
    padding: 4px;
    border: 0;
    border-radius: 50%;
    background: var(--card);
    display: grid;
    place-items: center;
    cursor: zoom-in;
  }
  .avatar-button.static {
    cursor: default;
  }
  .who {
    min-width: 0;
    padding-bottom: 4px;
  }
  .who h2 {
    margin: 0;
    font-size: 24px;
    line-height: 1.12;
    letter-spacing: -0.04em;
  }
  .who p {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--muted);
  }
  .pills {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    margin-top: 8px;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 19px;
    padding: 0 7px;
    border: 1px solid var(--line);
    border-radius: 99px;
    background: var(--panel-2);
    color: var(--text-dim);
    font-size: 9.5px;
    font-weight: 600;
  }
  .pill.owner {
    color: var(--yellow);
  }
  .content {
    display: flex;
    flex-direction: column;
    gap: 23px;
    padding: 20px 24px 26px;
  }
  section h3 {
    margin: 0 0 9px;
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .roles {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }
  .role {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 22px;
    padding: 0 8px;
    border: 1px solid color-mix(in srgb, var(--role-color, var(--muted)) 30%, transparent);
    border-radius: 5px;
    background: color-mix(in srgb, var(--role-color, var(--muted)) 12%, transparent);
    color: color-mix(in srgb, var(--role-color, var(--text-dim)) 70%, white);
    font-size: 10px;
    font-weight: 600;
  }
  .role i {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--role-color, var(--muted));
  }
  .role img {
    width: 13px;
    height: 13px;
    object-fit: contain;
    display: block;
  }
  .blocks {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .block-header {
    font-size: 18px;
    line-height: 1.35;
    font-weight: 650;
    letter-spacing: -0.025em;
    color: var(--text);
  }
  .block-header.small {
    font-size: 13px;
    color: var(--text-dim);
  }
  .block-text {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.55;
    color: var(--text-dim);
  }
  .block-link {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 11px;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: var(--panel-2);
    color: var(--text-dim);
    text-align: left;
  }
  .block-link:hover {
    background: var(--panel-3);
    color: var(--text);
  }
  .block-link.static {
    cursor: default;
  }
  .block-link.static:hover {
    background: var(--panel-2);
    color: var(--text-dim);
  }
  .block-icon {
    width: 26px;
    height: 26px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 7px;
    background: var(--panel-3);
    color: var(--muted);
  }
  .block-copy {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .block-copy b {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    font-weight: 600;
  }
  .block-copy small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 9.5px;
    color: var(--muted);
  }
  .block-image {
    width: 100%;
    max-height: 240px;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 9px;
    background: var(--panel-2);
    overflow: hidden;
    cursor: zoom-in;
  }
  .block-image img {
    width: 100%;
    max-height: 240px;
    object-fit: cover;
    display: block;
  }
  .empty {
    margin: 0;
    font-size: 11.5px;
    color: var(--muted);
  }
  .bento {
    display: grid;
    grid-template-columns: repeat(16, minmax(0, 1fr));
    grid-auto-rows: 44px;
    gap: 8px;
  }
  .bento-card {
    grid-column: var(--bento-column, span 4);
    grid-row: var(--bento-row, span 2);
    min-width: 0;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--panel-2);
    padding: 12px;
    overflow: hidden;
  }
  .bento-time {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
  }
  .bento-time b {
    font-size: 27px;
    line-height: 1;
    letter-spacing: -0.04em;
  }
  .bento-time span,
  .bento-weather span,
  .bento-label {
    font-size: 9.5px;
    color: var(--muted);
  }
  .bento-weather {
    height: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .bento-weather div {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .bento-weather b {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }
  .bento-icon {
    width: 32px;
    height: 32px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 9px;
    background: var(--panel-3);
    color: var(--accent);
  }
  .bento-label {
    display: block;
    margin-bottom: 8px;
  }
  .games {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 6px;
  }
  .game {
    min-width: 0;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--text-dim);
    text-align: left;
  }
  .game img {
    width: 100%;
    aspect-ratio: 3 / 4;
    object-fit: cover;
    border-radius: 6px;
    background: var(--panel-3);
    display: block;
  }
  .game span {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-top: 4px;
    font-size: 8.5px;
  }
  .bento-image {
    width: 100%;
    height: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: zoom-in;
  }
  .bento-image img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 7px;
    display: block;
  }
  .facts {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-top: 12px;
    border-top: 1px solid var(--line);
  }
  .facts div {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
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
  .skeleton {
    display: flex;
    flex-direction: column;
    gap: 9px;
    padding: 20px 24px 30px;
  }
  .sk-line,
  .sk-card {
    display: block;
    border-radius: 6px;
    background: var(--panel-3);
    animation: skeleton-pulse 1.35s ease-in-out infinite;
  }
  .sk-line {
    width: 100%;
    height: 10px;
  }
  .sk-line.short {
    width: 58%;
  }
  .sk-line.sk-heading {
    width: 32%;
    height: 14px;
    margin-bottom: 3px;
  }
  .sk-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px;
    margin-top: 7px;
  }
  .sk-card {
    height: 104px;
  }
  .sk-card.wide {
    grid-column: span 2;
  }
  @keyframes skeleton-pulse {
    0%,
    100% {
      opacity: 0.35;
    }
    50% {
      opacity: 0.75;
    }
  }

  @media (max-width: 680px) {
    .backdrop {
      padding: 12px;
    }
    .profile {
      max-height: calc(100vh - 24px);
    }
    .banner {
      height: 118px;
    }
    .intro {
      padding: 0 16px;
    }
    .content,
    .skeleton {
      padding-left: 16px;
      padding-right: 16px;
    }
    .intro {
      gap: 12px;
    }
    .avatar-button {
      width: 74px;
      height: 74px;
    }
    .who h2 {
      font-size: 20px;
    }
    .bento {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      grid-auto-rows: auto;
    }
    .bento-card {
      grid-column: span 1;
      grid-row: auto;
      min-height: 100px;
    }
    .games {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
