<script lang="ts">
  import { onMount } from "svelte";
  import { confirmDialog, reportDialog } from "../dialog";
  import Icon from "./Icon.svelte";

  let category = "harassment";
  let details = "";

  $: report = $reportDialog;
  $: if (report) {
    category = "harassment";
    details = "";
  }

  onMount(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if ($confirmDialog) closeConfirm(false);
      else if ($reportDialog) closeReport(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  function closeConfirm(value: boolean) {
    const dialog = $confirmDialog;
    confirmDialog.set(null);
    dialog?.resolve(value);
  }

  function closeReport(value: { category: string; details: string } | null) {
    const dialog = $reportDialog;
    reportDialog.set(null);
    dialog?.resolve(value);
  }

  function submitReport() {
    closeReport({ category, details: details.trim() });
  }
</script>

{#if $confirmDialog}
  <div class="backdrop" role="presentation" on:click={() => closeConfirm(false)}>
    <div class="dialog" role="dialog" aria-label={$confirmDialog.title} tabindex="-1" on:click|stopPropagation>
      <header>
        <div>
          <h2>{$confirmDialog.title}</h2>
          {#if $confirmDialog.body}<p>{$confirmDialog.body}</p>{/if}
        </div>
        <button class="icon-btn" aria-label="Close" on:click={() => closeConfirm(false)}>
          <Icon name="close" size={17} />
        </button>
      </header>
      <div class="actions">
        <button class="ghost" on:click={() => closeConfirm(false)}>Cancel</button>
        <button class:danger={$confirmDialog.danger} class="primary" on:click={() => closeConfirm(true)}>
          {$confirmDialog.confirmLabel ?? "Confirm"}
        </button>
      </div>
    </div>
  </div>
{:else if report}
  <div class="backdrop" role="presentation" on:click={() => closeReport(null)}>
    <div class="dialog report" role="dialog" aria-label="Report user" tabindex="-1" on:click|stopPropagation>
      <header>
        <div>
          <h2>Report @{report.user.username}</h2>
          <p>Reports go to Flackle moderators.</p>
        </div>
        <button class="icon-btn" aria-label="Close" on:click={() => closeReport(null)}>
          <Icon name="close" size={17} />
        </button>
      </header>

      <label>
        <span>Reason</span>
        <select bind:value={category}>
          <option value="harassment">Harassment</option>
          <option value="spam">Spam</option>
          <option value="impersonation">Impersonation</option>
          <option value="hate">Hateful content</option>
          <option value="other">Other</option>
        </select>
      </label>

      <label>
        <span>Details</span>
        <textarea
          bind:value={details}
          rows="4"
          maxlength="1000"
          placeholder="Add context for the moderators…"
        ></textarea>
      </label>

      <div class="actions">
        <button class="ghost" on:click={() => closeReport(null)}>Cancel</button>
        <button class="primary danger" disabled={!details.trim()} on:click={submitReport}>
          Submit report
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 240;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(8px);
    display: grid;
    place-items: center;
    padding: 24px;
  }
  .dialog {
    width: min(410px, calc(100vw - 48px));
    background: var(--card);
    border: 1px solid var(--line-strong);
    border-radius: 14px;
    box-shadow: var(--shadow);
    padding: 16px;
    animation: rise 140ms ease-out;
  }
  .dialog.report {
    width: min(440px, calc(100vw - 48px));
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  h2 {
    margin: 0;
    font-size: 16px;
    letter-spacing: -0.025em;
  }
  header p {
    margin: 3px 0 0;
    font-size: 11px;
    line-height: 1.5;
    color: var(--muted);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 7px;
    margin-top: 18px;
  }
  .actions button {
    height: 34px;
    padding: 0 13px;
    border-radius: 8px;
    font-size: 11px;
    font-weight: 650;
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
  .primary.danger {
    background: var(--accent);
  }
  .primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 86%, white);
  }
  .primary:disabled {
    opacity: 0.45;
    cursor: default;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 15px;
  }
  label span {
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--muted);
  }
  select,
  textarea {
    width: 100%;
    border: 1px solid var(--line-strong);
    border-radius: 8px;
    background: var(--input);
    color: var(--text);
    font-size: 12px;
    outline: none;
  }
  select {
    height: 36px;
    padding: 0 9px;
  }
  textarea {
    padding: 9px 10px;
    resize: vertical;
    line-height: 1.5;
  }
  select:focus,
  textarea:focus {
    border-color: rgba(var(--accent-rgb), 0.42);
  }
</style>
