<script lang="ts">
  export let user: { username: string; display_name?: string | null; avatar_url?: string | null; status?: string | null } | null = null;
  export let size = 36;
  export let status = true;

  const palette = ["#ff6b5e", "#7c5cff", "#2ec4a6", "#f2a33c", "#4aa8ff", "#e05ea8", "#9ad14b"];

  $: name = user?.display_name || user?.username || "?";
  $: initials = name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase())
    .join("");
  $: color = palette[(name.charCodeAt(0) || 0) % palette.length];
  $: online = user?.status === "online" || user?.status === "idle" || user?.status === "dnd";
</script>

<div class="avatar" style={`--av-size:${size}px; --av-color:${color}`}>
  {#if user?.avatar_url}
    <img src={user.avatar_url} alt="" loading="lazy" />
  {:else}
    <span>{initials}</span>
  {/if}
  {#if status && user}
    <i class:online class:dnd={user.status === "dnd"} class:idle={user.status === "idle"}></i>
  {/if}
</div>

<style>
  .avatar {
    width: var(--av-size);
    height: var(--av-size);
    min-width: var(--av-size);
    min-height: var(--av-size);
    max-width: var(--av-size);
    max-height: var(--av-size);
    aspect-ratio: 1 / 1;
    border-radius: 50%;
    background: color-mix(in srgb, var(--av-color) 22%, #161922);
    color: color-mix(in srgb, var(--av-color) 82%, white);
    display: grid;
    place-items: center;
    font-size: calc(var(--av-size) * 0.36);
    font-weight: 650;
    letter-spacing: -0.02em;
    position: relative;
    flex: 0 0 var(--av-size);
    align-self: center;
    overflow: visible;
    user-select: none;
  }
  img {
    width: 100%;
    height: 100%;
    aspect-ratio: 1 / 1;
    object-fit: cover;
    border-radius: 50%;
    display: block;
  }
  i {
    position: absolute;
    right: -1px;
    bottom: -1px;
    width: calc(var(--av-size) * 0.38);
    height: calc(var(--av-size) * 0.38);
    min-width: 11px;
    min-height: 11px;
    border-radius: 50%;
    background: #6b7280;
    border: 2px solid var(--panel, #111319);
  }
  i.online {
    background: var(--green);
  }
  i.idle {
    background: var(--yellow);
  }
  i.dnd {
    background: var(--accent);
  }
</style>
