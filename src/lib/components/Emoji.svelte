<script lang="ts">
  import { emojiPath, isBundledEmoji } from "../emoji";

  export let char: string;
  export let size = 16;
  export let title = "";

  let failed = false;
</script>

{#if isBundledEmoji(char) && !failed}
  <img
    class="emoji"
    src={emojiPath(char)}
    alt={title || char}
    title={title || undefined}
    style={`--emoji-size:${size}px`}
    draggable="false"
    loading="lazy"
    on:error={() => (failed = true)}
  />
{:else}
  <span class="fallback" style={`font-size:${size}px`} title={title || undefined}>{char}</span>
{/if}

<style>
  .emoji {
    width: var(--emoji-size, 1em);
    height: var(--emoji-size, 1em);
    display: inline-block;
    vertical-align: -0.14em;
    object-fit: contain;
    user-select: none;
  }
  .fallback {
    line-height: 1;
    display: inline-block;
    vertical-align: -0.1em;
  }
</style>
