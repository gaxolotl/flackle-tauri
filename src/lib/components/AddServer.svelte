<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "../api";
  import type { Server } from "../types";
  import {
    ensureChannels,
    modal,
    notify,
    openGuild,
    servers,
  } from "../workspace";
  import Icon from "./Icon.svelte";
  import Mark from "./Mark.svelte";

  type Tab = "join" | "create";

  let tab: Tab = "join";
  let code = "";
  let name = "";
  let busy = false;
  let error = "";

  onMount(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") modal.set(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  function inviteCode(value: string): string {
    let trimmed = value.trim();
    if (!trimmed) return "";
    try {
      const url = new URL(trimmed);
      const parts = url.pathname.split("/").filter(Boolean);
      trimmed = parts[parts.length - 1] ?? trimmed;
    } catch {
      // Raw code, not a URL.
    }
    return trimmed.replace(/^\/+|\/+$/g, "");
  }

  async function afterJoin() {
    const list = await api.fetchServers();
    servers.set(list);
    return list;
  }

  async function openServer(server: Server, preferredChannel?: string | null) {
    const bundle = await ensureChannels(server.id);
    const sorted = (type: string) =>
      bundle.channels
        .filter((channel) => channel.type === type)
        .sort((a, b) => (a.position ?? 0) - (b.position ?? 0));
    const channelId =
      preferredChannel ??
      sorted("text")[0]?.id ??
      sorted("stats")[0]?.id;
    if (channelId) await openGuild(server, channelId);
  }

  async function join() {
    if (busy) return;
    const value = inviteCode(code);
    if (!value) {
      error = "Enter an invite code or link.";
      return;
    }
    busy = true;
    error = "";
    try {
      const result = await api.acceptInvite(value);
      await afterJoin();
      const server = result.server ?? null;
      if (server) {
        await openServer(server, result.landing_channel_id);
        notify(`Joined ${server.name}.`);
      } else {
        notify("Joined the server.");
      }
      modal.set(null);
    } catch (reason) {
      error = reason instanceof Error ? reason.message : String(reason);
    } finally {
      busy = false;
    }
  }

  async function create() {
    if (busy) return;
    if (!name.trim()) {
      error = "Give the server a name.";
      return;
    }
    busy = true;
    error = "";
    try {
      const result = await api.createServer(name.trim());
      await afterJoin();
      await openServer(result.server, result.channel.id);
      notify(`Created ${result.server.name}.`);
      modal.set(null);
    } catch (reason) {
      error = reason instanceof Error ? reason.message : String(reason);
    } finally {
      busy = false;
    }
  }
</script>

<div class="backdrop" role="presentation" on:click={() => modal.set(null)}>
  <div class="modal" role="dialog" aria-label="Add a server" tabindex="-1" on:click|stopPropagation>
    <header>
      <span class="mark"><Mark size={26} /></span>
      <div>
        <h2>Add a server</h2>
        <p>Create your own or join one with an invite.</p>
      </div>
      <button class="icon-btn" aria-label="Close" on:click={() => modal.set(null)}>
        <Icon name="close" size={18} />
      </button>
    </header>

    <div class="tabs">
      <button class:active={tab === "join"} on:click={() => (tab = "join")}>
        <Icon name="link" size={14} /> Join a server
      </button>
      <button class:active={tab === "create"} on:click={() => (tab = "create")}>
        <Icon name="plus" size={14} /> Create a server
      </button>
    </div>

    <div class="body">
      {#if tab === "join"}
        <label>
          <span>Invite code or link</span>
          <input
            bind:value={code}
            placeholder="flackle.com/invite/ABCDEF"
            spellcheck="false"
            disabled={busy}
            on:keydown={(event) => {
              if (event.key === "Enter") void join();
            }}
          />
        </label>
        <p class="hint">Paste an invite link, or just the code.</p>
      {:else}
        <label>
          <span>Server name</span>
          <input
            bind:value={name}
            placeholder="My server"
            maxlength="100"
            disabled={busy}
            on:keydown={(event) => {
              if (event.key === "Enter") void create();
            }}
          />
        </label>
        <p class="hint">You can change the name and icon later in server settings.</p>
      {/if}

      {#if error}<p class="error"><Icon name="alert" size={13} />{error}</p>{/if}

      <div class="actions">
        <button class="ghost" on:click={() => modal.set(null)}>Cancel</button>
        <button
          class="primary"
          disabled={busy}
          on:click={() => (tab === "join" ? join() : create())}
        >
          {#if busy}<span class="spin"></span>{/if}
          {tab === "join" ? "Join server" : "Create server"}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 130;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(8px);
    display: grid;
    place-items: center;
    padding: 24px;
  }
  .modal {
    width: min(450px, calc(100vw - 48px));
    background: var(--card);
    border: 1px solid var(--line-strong);
    border-radius: 15px;
    box-shadow: var(--shadow);
    overflow: hidden;
    animation: rise 150ms ease-out;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(7px);
    }
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 18px 16px 16px 18px;
    border-bottom: 1px solid var(--line);
  }
  .mark {
    width: 42px;
    height: 42px;
    border-radius: 12px;
    background: var(--accent-dim);
    border: 1px solid rgba(var(--accent-rgb), 0.22);
    color: var(--accent);
    display: grid;
    place-items: center;
    flex: none;
  }
  header > div {
    flex: 1;
  }
  h2 {
    margin: 0;
    font-size: 17px;
    letter-spacing: -0.03em;
  }
  header p {
    margin: 2px 0 0;
    font-size: 11px;
    color: var(--muted);
  }
  .tabs {
    display: flex;
    gap: 2px;
    padding: 8px;
    border-bottom: 1px solid var(--line);
    background: var(--sidebar);
  }
  .tabs button {
    flex: 1;
    height: 33px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }
  .tabs button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .tabs button.active {
    background: var(--active);
    color: var(--text);
  }
  .body {
    padding: 18px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  label span {
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--muted);
  }
  input {
    height: 40px;
    border: 1px solid var(--line-strong);
    border-radius: 8px;
    background: var(--input);
    color: var(--text);
    padding: 0 11px;
    font-size: 13px;
    outline: none;
  }
  input:focus {
    border-color: rgba(var(--accent-rgb), 0.45);
  }
  .hint {
    margin: 7px 0 0;
    font-size: 10px;
    color: var(--muted);
  }
  .error {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 12px 0 0;
    color: #ff8a8a;
    font-size: 11px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 7px;
    margin-top: 20px;
  }
  .actions button {
    height: 34px;
    padding: 0 13px;
    border-radius: 8px;
    font-size: 11px;
    font-weight: 650;
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .ghost {
    border: 1px solid var(--line);
    background: transparent;
    color: var(--muted);
  }
  .ghost:hover {
    background: var(--hover);
    color: var(--text);
  }
  .primary {
    border: 0;
    background: var(--accent);
    color: white;
  }
  .primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 86%, white);
  }
  .primary:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .spin {
    width: 12px;
    height: 12px;
    border-width: 2px;
  }
</style>
