<script lang="ts">
  import { onMount, tick } from "svelte";
  import * as api from "../api";
  import { channelDialog, closeChannelDialog } from "../channel-dialog";
  import { channels, moveChannelToCategory, notify, openGuild, reloadChannels, servers } from "../workspace";
  import Icon from "./Icon.svelte";

  type ChannelKind = {
    type: string;
    label: string;
    description: string;
    icon: string;
  };

  const channelKinds: Record<string, ChannelKind> = {
    text: {
      type: "text",
      label: "Text",
      description: "Messages, files, and reactions",
      icon: "hash",
    },
    voice: {
      type: "voice",
      label: "Voice",
      description: "Voice rooms and streaming",
      icon: "speaker",
    },
    stats: {
      type: "stats",
      label: "Stats",
      description: "Live member, online, and voice activity for this server",
      icon: "chart",
    },
  };

  let name = "";
  let topic = "";
  let type = "text";
  let categoryId = "";
  let saving = false;
  let nameInput: HTMLInputElement | undefined;
  let creatable = ["text", "voice", "stats"];

  $: typeChoices = creatable
    .map((kind) => channelKinds[kind])
    .filter((kind): kind is ChannelKind => !!kind);

  $: request = $channelDialog;
  $: categories = [...($channels?.categories ?? [])].sort(
    (a, b) => (a.position ?? 0) - (b.position ?? 0),
  );
  $: isCategory =
    request?.mode === "category-create" || request?.mode === "category-edit";
  $: editing = request?.mode === "channel-edit" || request?.mode === "category-edit";
  $: title = isCategory
    ? editing
      ? "Edit category"
      : "Create category"
    : editing
      ? `Edit #${request?.channel?.name ?? "channel"}`
      : "Create channel";

  let lastRequest: unknown = null;
  $: if (request && request !== lastRequest) {
    lastRequest = request;
    name =
      request.mode === "channel-edit"
        ? request.channel?.name ?? ""
        : request.mode === "category-edit"
          ? request.category?.name ?? ""
          : "";
    topic = request.mode === "channel-edit" ? request.channel?.topic ?? "" : "";
    type = request.mode === "channel-edit" ? request.channel?.type ?? "text" : "text";
    categoryId = request.categoryId ?? "";
    void tick().then(() => nameInput?.focus());
  }

  onMount(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && $channelDialog) closeChannelDialog();
    };
    window.addEventListener("keydown", onKey);
    void api
      .fetchChannelKinds()
      .then((kinds) => {
        const known = kinds.filter((kind) => !!channelKinds[kind]);
        if (known.length > 0) {
          creatable = known;
          if (!known.includes(type)) type = known[0];
        }
      })
      .catch(() => {
        // Keep the built-in kinds when the endpoint is unavailable.
      });
    return () => window.removeEventListener("keydown", onKey);
  });

  function cleanName(value: string, category: boolean) {
    const trimmed = value.trim();
    if (category) return trimmed;
    return trimmed.toLowerCase().replace(/\s+/g, "-");
  }

  async function submit() {
    if (!request || saving) return;
    const value = cleanName(name, isCategory);
    if (!value) {
      notify(isCategory ? "A category needs a name." : "A channel needs a name.");
      return;
    }
    const server = $servers.find((entry) => entry.id === request.serverId);
    if (!server) return;
    saving = true;
    try {
      if (request.mode === "category-create") {
        await api.createCategory(request.serverId, value);
        await reloadChannels(server);
        notify("Category created.");
      } else if (request.mode === "category-edit" && request.category) {
        await api.updateCategory(request.serverId, request.category.id, value);
        await reloadChannels(server);
        notify("Category updated.");
      } else if (request.mode === "channel-create") {
        const created = await api.createChannel(request.serverId, {
          name: value,
          type,
          category_id: categoryId || null,
          topic: topic.trim() || null,
        });
        const bundle = await reloadChannels(server);
        notify("Channel created.");
        if (
          (created.type === "text" || created.type === "stats") &&
          bundle.channels.some((item) => item.id === created.id)
        ) {
          await openGuild(server, created.id);
        }
      } else if (request.mode === "channel-edit" && request.channel) {
        await api.updateChannel(request.serverId, request.channel.id, {
          name: value,
          topic: topic.trim(),
        });
        if ((request.channel.category_id ?? null) !== (categoryId || null)) {
          await moveChannelToCategory(server, request.channel.id, categoryId || null);
        } else {
          await reloadChannels(server);
        }
        notify("Channel updated.");
      }
      closeChannelDialog();
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    } finally {
      saving = false;
    }
  }
</script>

{#if request}
  <div class="backdrop" role="presentation" on:click={closeChannelDialog}>
    <div class="modal" role="dialog" aria-label={title} tabindex="-1" on:click|stopPropagation>
      <header>
        <div>
          <h2>{title}</h2>
          <p>{request.serverName}</p>
        </div>
        <button class="icon-btn" aria-label="Close" on:click={closeChannelDialog}>
          <Icon name="close" size={17} />
        </button>
      </header>

      <div class="body">
        <label class="field">
          <span>{isCategory ? "Category name" : "Channel name"}</span>
          <input
            bind:this={nameInput}
            bind:value={name}
            maxlength={isCategory ? 100 : 60}
            spellcheck="false"
            placeholder={isCategory ? "New category" : "new-channel"}
            on:keydown={(event) => {
              if (event.key === "Enter") void submit();
            }}
          />
        </label>

        {#if !isCategory && !editing}
          <label class="field">
            <span>Channel type</span>
            <div class="type-row">
              {#each typeChoices as choice (choice.type)}
                <button
                  type="button"
                  class="type-choice"
                  class:active={type === choice.type}
                  on:click={() => (type = choice.type)}
                >
                  <Icon name={choice.icon} size={16} />
                  <b>{choice.label}</b>
                  <small>{choice.description}</small>
                </button>
              {/each}
            </div>
          </label>
        {/if}

        {#if !isCategory}
          <label class="field">
            <span>Category</span>
            <select bind:value={categoryId}>
              <option value="">No category</option>
              {#each categories as category (category.id)}
                <option value={category.id}>{category.name}</option>
              {/each}
            </select>
          </label>

          {#if type === "text"}
            <label class="field">
              <span>Topic</span>
              <textarea
                bind:value={topic}
                rows="3"
                maxlength="1024"
                placeholder="What is this channel for?"
              ></textarea>
            </label>
          {/if}
        {/if}

        {#if editing && !isCategory}
          <p class="hint">
            Channel type cannot be changed. Drag the channel in the sidebar to move it between categories.
          </p>
        {/if}
      </div>

      <footer>
        <button class="secondary" on:click={closeChannelDialog}>Cancel</button>
        <button class="primary" disabled={saving} on:click={submit}>
          {saving ? "Saving…" : editing ? "Save changes" : isCategory ? "Create category" : "Create channel"}
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 150;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(8px);
    display: grid;
    place-items: center;
    padding: 28px;
  }
  .modal {
    width: min(480px, calc(100vw - 56px));
    max-height: calc(100vh - 56px);
    background: var(--card);
    border: 1px solid var(--line-strong);
    border-radius: 16px;
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: rise 150ms ease-out;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(7px) scale(0.995);
    }
  }
  header {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 16px 16px 14px 20px;
    border-bottom: 1px solid var(--line);
  }
  h2 {
    margin: 0;
    font-size: 16px;
    letter-spacing: -0.03em;
  }
  header p {
    margin: 2px 0 0;
    font-size: 10.5px;
    color: var(--muted);
  }
  .body {
    padding: 18px 20px 4px;
    overflow-y: auto;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 15px;
  }
  .field > span {
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--muted);
  }
  input,
  textarea,
  select {
    width: 100%;
    border: 1px solid var(--line-strong);
    border-radius: 8px;
    background: var(--input);
    color: var(--text);
    font-size: 12.5px;
    outline: none;
  }
  input,
  select {
    height: 38px;
    padding: 0 10px;
  }
  textarea {
    padding: 9px 10px;
    resize: vertical;
    line-height: 1.5;
  }
  input:focus,
  textarea:focus,
  select:focus {
    border-color: rgba(var(--accent-rgb), 0.4);
    box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.06);
  }
  .type-row {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 7px;
  }
  .type-choice {
    border: 1px solid var(--line-strong);
    border-radius: 9px;
    background: var(--panel-2);
    color: var(--muted);
    padding: 10px;
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: center;
    gap: 3px 8px;
    text-align: left;
  }
  .type-choice b {
    font-size: 11.5px;
    color: var(--text);
  }
  .type-choice small {
    grid-column: 2;
    font-size: 9.5px;
    color: var(--muted);
    line-height: 1.35;
  }
  .type-choice:hover,
  .type-choice.active {
    border-color: rgba(var(--accent-rgb), 0.38);
    background: var(--accent-dim);
  }
  .type-choice.active {
    color: var(--accent);
  }
  .hint {
    margin: 0 0 12px;
    color: var(--muted);
    font-size: 10px;
    line-height: 1.5;
  }
  footer {
    flex: none;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 14px 20px;
    border-top: 1px solid var(--line);
  }
  .primary,
  .secondary {
    height: 34px;
    border-radius: 8px;
    padding: 0 13px;
    font-size: 11px;
    font-weight: 650;
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
    opacity: 0.45;
    cursor: default;
  }
  .secondary {
    border: 1px solid var(--line);
    background: var(--panel-2);
    color: var(--text-dim);
  }
  .secondary:hover {
    background: var(--panel-3);
    color: var(--text);
  }
</style>
