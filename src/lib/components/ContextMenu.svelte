<script lang="ts">
  import { onMount } from "svelte";
  import { closeContextMenu, contextMenu, type ContextMenuItem } from "../context-menu";
  import Icon from "./Icon.svelte";

  const width = 214;

  $: menu = $contextMenu;
  $: height = menu
    ? menu.items.reduce((total, item) => total + (item.separator ? 9 : 31), 12)
    : 0;
  $: left = menu
    ? Math.max(8, Math.min(menu.x, window.innerWidth - width - 8))
    : 0;
  $: top = menu
    ? Math.max(8, Math.min(menu.y, window.innerHeight - height - 8))
    : 0;

  onMount(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") closeContextMenu();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  function choose(item: ContextMenuItem) {
    if (item.disabled || !item.run) return;
    closeContextMenu();
    item.run();
  }
</script>

{#if menu}
  <div
    class="backdrop"
    role="presentation"
    on:click={closeContextMenu}
    on:contextmenu|preventDefault={closeContextMenu}
  >
    <div
      class="menu"
      role="menu"
      tabindex="-1"
      style={`left:${left}px; top:${top}px; width:${width}px`}
      on:click|stopPropagation
    >
      {#each menu.items as item}
        {#if item.separator}
          <div class="separator"></div>
        {:else}
          <button
            role="menuitem"
            class:danger={item.danger}
            disabled={item.disabled}
            on:click={() => choose(item)}
          >
            <span class="icon">
              {#if item.icon}<Icon name={item.icon} size={14} />{/if}
            </span>
            <span class="label">{item.label}</span>
            {#if item.shortcut}<span class="shortcut">{item.shortcut}</span>{/if}
          </button>
        {/if}
      {/each}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 220;
  }
  .menu {
    position: fixed;
    background: var(--card);
    border: 1px solid var(--line-strong);
    border-radius: 9px;
    box-shadow: var(--shadow);
    padding: 5px;
    display: flex;
    flex-direction: column;
  }
  button {
    height: 29px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--text-dim);
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 7px;
    font-size: 11.5px;
    text-align: left;
  }
  button:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  button.danger {
    color: #ff8a8a;
  }
  button.danger:hover {
    background: rgba(var(--accent-rgb), 0.12);
  }
  button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .icon {
    width: 16px;
    display: grid;
    place-items: center;
    color: var(--muted);
    flex: none;
  }
  button.danger .icon {
    color: #ff8a8a;
  }
  .label {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .shortcut {
    font-size: 9px;
    color: var(--muted);
    flex: none;
  }
  .separator {
    height: 1px;
    background: var(--line);
    margin: 4px 6px;
  }
</style>
