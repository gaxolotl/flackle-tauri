<script lang="ts">
  import { inTauri } from "../api";
  import Icon from "./Icon.svelte";
  import Mark from "./Mark.svelte";

  const isMac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.userAgent);
  let maximized = false;

  async function windowAction(action: "minimize" | "maximize" | "close") {
    if (!inTauri) return;
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();
    if (action === "minimize") await win.minimize();
    if (action === "close") await win.close();
    if (action === "maximize") {
      await win.toggleMaximize();
      maximized = await win.isMaximized();
    }
  }
</script>

<header class="titlebar" data-tauri-drag-region class:mac={isMac}>
  <div class="brand" data-tauri-drag-region>
    <span class="mark"><Mark size={16} /></span>
    <span class="name">Flackle</span>
  </div>
  <div class="drag" data-tauri-drag-region></div>
  {#if !isMac}
    <div class="controls">
      <button class="ctl" aria-label="Minimize" on:click={() => windowAction("minimize")}>
        <Icon name="minimize" size={14} />
      </button>
      <button class="ctl" aria-label="Maximize" on:click={() => windowAction("maximize")}>
        <Icon name={maximized ? "restore" : "maximize"} size={12} />
      </button>
      <button class="ctl close" aria-label="Close" on:click={() => windowAction("close")}>
        <Icon name="close" size={14} />
      </button>
    </div>
  {/if}
</header>

<style>
  .titlebar {
    height: 38px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding-left: 14px;
    background: var(--titlebar);
    border-bottom: 1px solid var(--line);
    -webkit-user-select: none;
    user-select: none;
    position: relative;
    z-index: 40;
  }
  .titlebar.mac {
    padding-left: 82px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 100%;
    flex: none;
  }
  .mark {
    color: var(--brand);
    display: flex;
  }
  .name {
    font-family: "Satoshi", "Manrope", system-ui, sans-serif;
    font-size: 13px;
    font-weight: 650;
    letter-spacing: 0.005em;
  }
  .drag {
    flex: 1;
    height: 100%;
  }
  .controls {
    display: flex;
    height: 100%;
    flex: none;
  }
  .ctl {
    width: 44px;
    height: 100%;
    border: 0;
    background: transparent;
    color: #8c8c98;
    display: grid;
    place-items: center;
    cursor: default;
    border-radius: 0;
  }
  .ctl:hover {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text);
  }
  .ctl.close:hover {
    background: #d92b2b;
    color: white;
  }
</style>
