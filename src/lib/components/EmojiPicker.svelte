<script lang="ts">
  import { onMount, tick } from "svelte";
  import {
    emojiCategories,
    getRecentEmoji,
    pushRecentEmoji,
    searchEmoji,
  } from "../emoji";
  import Emoji from "./Emoji.svelte";
  import Icon from "./Icon.svelte";

  export let x: number;
  export let y: number;
  export let onSelect: (emoji: string) => void;
  export let onClose: () => void;

  const width = 352;
  const height = 392;

  let query = "";
  let active = emojiCategories[0].id;
  let searchInput: HTMLInputElement;
  let recent = getRecentEmoji();

  $: results = searchEmoji(query);
  $: activeCategory =
    emojiCategories.find((category) => category.id === active) ?? emojiCategories[0];
  $: entries = query.trim() ? results : activeCategory.entries;
  $: left = Math.max(10, Math.min(x, window.innerWidth - width - 10));
  $: top =
    y + height + 14 > window.innerHeight
      ? Math.max(10, y - height - 10)
      : Math.max(10, Math.min(y + 8, window.innerHeight - height - 10));

  onMount(() => {
    void tick().then(() => searchInput?.focus());
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  function choose(char: string) {
    pushRecentEmoji(char);
    onSelect(char);
    onClose();
  }

  function setCategory(id: string) {
    active = id;
    query = "";
  }
</script>

<div class="backdrop" role="presentation" on:click={onClose}>
  <div
    class="picker"
    role="dialog"
    aria-label="Emoji picker"
    tabindex="-1"
    style={`left:${left}px; top:${top}px`}
    on:click|stopPropagation
  >
    <div class="head">
      <div class="search">
        <Icon name="search" size={14} />
        <input
          bind:this={searchInput}
          bind:value={query}
          placeholder="Search emoji"
          spellcheck="false"
        />
      </div>
      <button class="close" aria-label="Close emoji picker" on:click={onClose}>
        <Icon name="close" size={15} />
      </button>
    </div>

    <div class="tabs" role="tablist" aria-label="Emoji categories">
      {#each emojiCategories as category}
        <button
          class:active={!query && active === category.id}
          title={category.label}
          aria-label={category.label}
          on:click={() => setCategory(category.id)}
        >
          <Emoji char={category.entries[0].char} size={17} />
        </button>
      {/each}
    </div>

    <div class="scroll">
      {#if !query && recent.length > 0}
        <div class="label">Recent</div>
        <div class="grid">
          {#each recent as char}
            <button
              class="cell"
              title={char}
              aria-label={char}
              on:click={() => choose(char)}
            >
              <Emoji {char} size={22} />
            </button>
          {/each}
        </div>
      {/if}

      <div class="label">{query.trim() ? "Results" : activeCategory.label}</div>
      <div class="grid">
        {#each entries as entry (entry.char)}
          <button
            class="cell"
            title={entry.name}
            aria-label={entry.name}
            on:click={() => choose(entry.char)}
          >
            <Emoji char={entry.char} size={22} />
          </button>
        {/each}
      </div>

      {#if entries.length === 0}
        <p class="empty">No emoji match “{query}”.</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 160;
  }
  .picker {
    position: fixed;
    width: 352px;
    height: 392px;
    background: var(--card);
    border: 1px solid var(--line-strong);
    border-radius: 14px;
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: pop 120ms ease-out;
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(4px) scale(0.99);
    }
  }
  .head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 9px 9px 7px;
  }
  .search {
    flex: 1;
    height: 34px;
    display: flex;
    align-items: center;
    gap: 7px;
    background: var(--input);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0 9px;
    color: var(--muted);
  }
  .search:focus-within {
    border-color: var(--line-strong);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    outline: none;
    color: var(--text);
    font-size: 12.5px;
  }
  .search input::placeholder {
    color: #5a5a64;
  }
  .close {
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    display: grid;
    place-items: center;
  }
  .close:hover {
    background: var(--hover);
    color: var(--text);
  }
  .tabs {
    flex: none;
    display: flex;
    gap: 2px;
    padding: 0 9px 6px;
    border-bottom: 1px solid var(--line);
  }
  .tabs button {
    width: 34px;
    height: 30px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    display: grid;
    place-items: center;
    opacity: 0.55;
  }
  .tabs button:hover {
    background: var(--hover);
    opacity: 1;
  }
  .tabs button.active {
    background: var(--active);
    opacity: 1;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 2px 10px 12px;
  }
  .label {
    padding: 12px 4px 5px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.09em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 2px;
  }
  .cell {
    height: 36px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    display: grid;
    place-items: center;
  }
  .cell:hover,
  .cell:focus-visible {
    background: var(--hover);
    outline: none;
  }
  .empty {
    margin: 20px 4px;
    color: var(--muted);
    font-size: 11.5px;
  }
</style>
