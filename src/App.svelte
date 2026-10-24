<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import * as api from "./lib/api";
  import { applyTheme, getTheme } from "./lib/theme";
  import type { SelfUser } from "./lib/types";
  import {
    connection,
    handleGatewayEvent,
    init,
    loadWorkspace,
    me,
    modal,
    notify,
    toast,
  } from "./lib/workspace";
  import Login from "./lib/components/Login.svelte";
  import Mark from "./lib/components/Mark.svelte";
  import AddServer from "./lib/components/AddServer.svelte";
  import ChannelDialog from "./lib/components/ChannelDialog.svelte";
  import ContextMenu from "./lib/components/ContextMenu.svelte";
  import DialogHost from "./lib/components/DialogHost.svelte";
  import Friends from "./lib/components/Friends.svelte";
  import ProfileCard from "./lib/components/ProfileCard.svelte";
  import ServerSettings from "./lib/components/ServerSettings.svelte";
  import ServerInfo from "./lib/components/ServerInfo.svelte";
  import Settings from "./lib/components/Settings.svelte";
  import Shell from "./lib/components/Shell.svelte";
  import Titlebar from "./lib/components/Titlebar.svelte";

  let loading = true;
  let bootError = "";
  const forceLogin = new URLSearchParams(location.search).has("login");

  applyTheme(getTheme());

  onMount(() => {
    let unlistenGateway: (() => void) | undefined;
    let cancelled = false;

    void (async () => {
      try {
        unlistenGateway = await api.listenGateway(handleGatewayEvent);
      } catch {
        // The web layer can still function without the event listener.
      }

      if (forceLogin) {
        loading = false;
        return;
      }

      try {
        const restored = await init();
        if (!restored) me.set(null);
      } catch (error) {
        bootError = error instanceof Error ? error.message : String(error);
      } finally {
        if (!cancelled) loading = false;
      }
    })();

    return () => {
      cancelled = true;
      unlistenGateway?.();
    };
  });

  async function handleLogin(user: SelfUser) {
    bootError = "";
    me.set(user);
    try {
      await loadWorkspace();
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    }
  }
</script>

<div class="app">
  <Titlebar />

  {#if loading}
    <div class="boot">
      <div class="boot-mark"><Mark size={42} /></div>
      <div class="boot-copy">
        <b>Flackle</b>
        <span>Restoring your session…</span>
      </div>
      <span class="spin"></span>
      {#if bootError}<p class="boot-error">{bootError}</p>{/if}
    </div>
  {:else if $me}
    <Shell />
    <ProfileCard />
    <ContextMenu />
    <DialogHost />
    <ChannelDialog />
    {#if $modal === "settings"}
      <Settings />
    {:else if $modal === "server-settings"}
      <ServerSettings />
    {:else if $modal === "server-info"}
      <ServerInfo />
    {:else if $modal === "add-server"}
      <AddServer />
    {:else if $modal === "friends"}
      <Friends />
    {/if}
    {#if $connection !== "online"}
      <div class="net" class:offline={$connection === "offline"}>
        <i></i>
        {$connection === "connecting" ? "Connecting to the gateway…" : "Reconnecting…"}
      </div>
    {/if}
  {:else}
    <div class="login-wrap">
      <Login onSuccess={handleLogin} />
      {#if bootError}<p class="login-error">{bootError}</p>{/if}
    </div>
  {/if}

  {#if $toast}
    <div class="toast" transition:fly={{ y: 8, duration: 160 }}>
      <span>{$toast}</span>
      <button aria-label="Dismiss" on:click={() => toast.set(null)}>×</button>
    </div>
  {/if}
</div>

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg);
  }
  .login-wrap {
    flex: 1;
    min-height: 0;
    position: relative;
    display: flex;
    flex-direction: column;
  }
  .login-wrap :global(.login) {
    flex: 1;
    height: auto;
    min-height: 0;
  }
  .boot {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
    position: relative;
    overflow: hidden;
  }
  .boot-mark {
    color: var(--accent);
    animation: breathe 2.4s ease-in-out infinite;
  }
  @keyframes breathe {
    0%,
    100% {
      transform: scale(1);
      opacity: 0.85;
    }
    50% {
      transform: scale(1.04);
      opacity: 1;
    }
  }
  .boot-copy {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    margin-top: 4px;
  }
  .boot-copy b {
    font-size: 16px;
    letter-spacing: -0.03em;
  }
  .boot-copy span {
    font-size: 11px;
    color: var(--muted);
  }
  .boot .spin {
    margin-top: 4px;
    width: 16px;
    height: 16px;
    border-width: 2px;
  }
  .boot-error,
  .login-error {
    font-size: 11px;
    color: #ff8a8a;
    background: rgba(var(--accent-rgb), 0.08);
    border: 1px solid rgba(var(--accent-rgb), 0.18);
    border-radius: 7px;
    padding: 6px 10px;
    margin: 0;
    z-index: 2;
  }
  .login-error {
    position: absolute;
    bottom: 18px;
    left: 50%;
    transform: translateX(-50%);
  }
  .net {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 90;
    display: flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 10px;
    border-radius: 99px;
    background: var(--panel-2);
    border: 1px solid var(--line-strong);
    color: var(--muted);
    font-size: 10px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
  }
  .net i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--yellow);
    animation: pulse 1.4s infinite;
  }
  .net.offline i {
    background: var(--accent);
  }
  @keyframes pulse {
    0%,
    100% {
      opacity: 0.35;
    }
    50% {
      opacity: 1;
    }
  }
  .toast {
    position: fixed;
    left: 50%;
    bottom: 20px;
    transform: translateX(-50%);
    z-index: 200;
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: min(560px, calc(100vw - 48px));
    background: var(--panel-2);
    border: 1px solid var(--line-strong);
    border-radius: 9px;
    box-shadow: var(--shadow);
    padding: 9px 9px 9px 13px;
    font-size: 11.5px;
    color: var(--text-dim);
  }
  .toast button {
    width: 20px;
    height: 20px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--muted);
    font-size: 14px;
    line-height: 1;
  }
  .toast button:hover {
    background: var(--hover);
    color: var(--text);
  }
</style>
