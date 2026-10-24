<script lang="ts">
  import { splitEmoji } from "../emoji";
  import Emoji from "./Emoji.svelte";

  export let text: string;
  export let size = 14;

  $: parts = splitEmoji(text ?? "");
</script>

{#each parts as part}
  {#if "emoji" in part}
    <Emoji char={part.emoji} {size} />
  {:else}{part.text}{/if}
{/each}
