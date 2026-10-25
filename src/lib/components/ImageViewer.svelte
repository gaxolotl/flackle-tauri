<script lang="ts">
  import { onMount, tick } from "svelte";
  import { closeImage, imageViewer } from "../image-viewer";
  import Icon from "./Icon.svelte";

  let loaded = false;
  let failed = false;
  let currentUrl = "";
  let viewerEl: HTMLDivElement | undefined;

  $: if ($imageViewer) void tick().then(() => viewerEl?.focus());

  $: if ($imageViewer && $imageViewer.url !== currentUrl) {
    currentUrl = $imageViewer.url;
    loaded = false;
    failed = false;
  } else if (!$imageViewer) {
    currentUrl = "";
    loaded = false;
    failed = false;
  }

  function onKey(event: KeyboardEvent) {
    if (event.key === "Escape" && $imageViewer) closeImage();
  }

  function onStageClick(event: MouseEvent) {
    if (event.target === event.currentTarget) closeImage();
  }

  function onStageKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      closeImage();
    }
  }

  onMount(() => {
    const listener = (event: KeyboardEvent) => onKey(event);
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  });
</script>

{#if $imageViewer}
  <div
    class="viewer"
    bind:this={viewerEl}
    role="dialog"
    aria-modal="true"
    aria-label={$imageViewer.alt}
    tabindex="-1"
  >
    <div class="bar">
      <div class="copy">
        <b>{$imageViewer.alt}</b>
        <span>Click outside or press Esc to close</span>
      </div>
      <button class="icon-btn" aria-label="Close image" on:click={closeImage}>
        <Icon name="close" size={18} />
      </button>
    </div>

    <div
      class="stage"
      role="button"
      tabindex="0"
      aria-label="Close image"
      on:click={onStageClick}
      on:keydown={onStageKeydown}
    >
      {#if !loaded && !failed}
        <span class="state">Loading image…</span>
      {/if}
      {#if failed}
        <span class="state error">Could not load this image.</span>
      {/if}
      <img
        class:ready={loaded}
        src={$imageViewer.url}
        alt={$imageViewer.alt}
        draggable="false"
        on:load={() => (loaded = true)}
        on:error={() => {
          failed = true;
          loaded = false;
        }}
      />
    </div>
  </div>
{/if}

<style>
  .viewer {
    position: fixed;
    inset: 0;
    z-index: 400;
    display: flex;
    flex-direction: column;
    background: rgba(6, 6, 9, 0.93);
    backdrop-filter: blur(9px);
    animation: viewer-in 120ms ease-out;
  }
  .viewer:focus {
    outline: none;
  }
  @keyframes viewer-in {
    from {
      opacity: 0;
    }
  }
  .bar {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    padding: 10px 14px 10px 18px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(10, 10, 14, 0.72);
  }
  .copy {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .copy b {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    font-weight: 600;
    color: #f1f1f4;
  }
  .copy span {
    font-size: 10px;
    color: rgba(226, 226, 234, 0.55);
  }
  .bar .icon-btn {
    color: rgba(235, 235, 241, 0.8);
    background: rgba(255, 255, 255, 0.05);
  }
  .bar .icon-btn:hover {
    color: #fff;
    background: rgba(255, 255, 255, 0.12);
  }
  .stage {
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
    padding: 28px;
    cursor: zoom-out;
  }
  .stage:focus-visible {
    outline: 1px solid rgba(255, 255, 255, 0.18);
    outline-offset: -6px;
  }
  .stage img {
    max-width: 100%;
    max-height: calc(100vh - 110px);
    object-fit: contain;
    border-radius: 4px;
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.55);
    cursor: default;
    opacity: 0;
    transition: opacity 140ms ease-out;
  }
  .stage img.ready {
    opacity: 1;
  }
  .state {
    position: absolute;
    color: rgba(230, 230, 238, 0.7);
    font-size: 12px;
  }
  .state.error {
    color: #ff9292;
  }

  @media (max-width: 640px) {
    .stage {
      padding: 14px;
    }
    .copy span {
      display: none;
    }
  }
</style>
