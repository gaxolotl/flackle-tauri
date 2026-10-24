<script lang="ts">
  import { channelIcons } from "../channel-icons";
  import Icon from "./Icon.svelte";

  export let name: string | null | undefined;
  export let color: string | null | undefined = null;
  export let size = 16;
  export let fallback = "hash";

  function safeColor(value: string | null | undefined): string {
    return value && /^#[0-9a-f]{3,8}$/i.test(value) ? value : "currentColor";
  }

  $: paths = name ? channelIcons[name] : undefined;
</script>

{#if paths && paths.length > 0}
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="currentColor"
    style={`color:${safeColor(color)}`}
    aria-hidden="true"
  >
    {#each paths as path}
      <path d={path.d} fill-rule={path.fillRule} clip-rule={path.clipRule} />
    {/each}
  </svg>
{:else}
  <Icon name={fallback} {size} />
{/if}
