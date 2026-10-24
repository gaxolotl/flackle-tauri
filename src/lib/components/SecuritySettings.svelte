<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "../api";
  import { confirmAction } from "../dialog";
  import type {
    AuthSession,
    BackupCodes,
    SecurityEvent,
    TwoFactorSetup,
    TwoFactorStatus,
  } from "../types";
  import { notify } from "../workspace";
  import Icon from "./Icon.svelte";

  const eventLabels: Record<string, string> = {
    login: "Signed in",
    login_failed: "Failed sign-in attempt",
    mfa_login: "Signed in with two-factor authentication",
    password_changed: "Password changed",
    password_reset_requested: "Password reset code requested",
    password_reset_failed: "Failed password reset",
    password_reset_mfa_failed: "Password reset stopped at two-factor authentication",
    password_reset: "Password reset completed",
    mfa_failed: "Incorrect two-factor authentication code",
    mfa_enabled: "Two-factor authentication enabled",
    mfa_disabled: "Two-factor authentication disabled",
    mfa_backup_code_used: "Backup code used to sign in",
    mfa_backup_codes_regenerated: "Backup codes regenerated",
    refresh_token_reuse: "Session token replayed, session ended",
    refresh_token_origin_conflict: "Conflicting session token use detected, session ended",
    refresh_token_reuse_reclaimed: "Session reclaimed after token replay",
    session_new_location: "Session used from a new location",
  };

  let loading = true;
  let error = "";
  let busy = "";
  let twofa: TwoFactorStatus | null = null;
  let sessions: AuthSession[] = [];
  let events: SecurityEvent[] = [];
  let showIps = false;
  let sessionsCode = "";
  let eventsCode = "";

  let setupStep: "off" | "password" | "code" = "off";
  let setupPassword = "";
  let setupCode = "";
  let setupData: TwoFactorSetup | null = null;

  let disableOpen = false;
  let disablePassword = "";
  let disableCode = "";
  let useBackupCode = false;

  let backupOpen = false;
  let backupPassword = "";
  let backupCode = "";
  let backupCodes: string[] = [];

  $: currentSession = sessions.find((session) => session.current) ?? null;
  $: otherSessions = sessions.filter((session) => !session.current);

  onMount(() => {
    void load();
  });

  async function load() {
    loading = true;
    error = "";
    try {
      const [status, sessionList, eventList] = await Promise.all([
        api.fetch2FAStatus(),
        api.fetchSessions(),
        api.fetchSecurityEvents(),
      ]);
      twofa = status;
      sessions = sessionList;
      events = eventList;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
    }
  }

  async function startSetup() {
    if (!setupPassword || busy === "setup") return;
    busy = "setup";
    try {
      setupData = await api.setup2FA(setupPassword);
      setupPassword = "";
      setupStep = "code";
      setupCode = "";
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      busy = "";
    }
  }

  async function confirmSetup() {
    if (!setupCode.trim() || busy === "confirm") return;
    busy = "confirm";
    try {
      await api.confirm2FA(setupCode.trim());
      setupStep = "off";
      setupData = null;
      setupCode = "";
      await load();
      notify("Two-factor authentication enabled.");
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      busy = "";
    }
  }

  function cancelSetup() {
    setupStep = "off";
    setupPassword = "";
    setupCode = "";
    setupData = null;
  }

  async function disableTwoFactor() {
    if (!disablePassword || busy === "disable") return;
    busy = "disable";
    try {
      await api.disable2FA(
        disablePassword,
        useBackupCode ? undefined : disableCode.trim(),
        useBackupCode ? disableCode.trim() : undefined,
      );
      disablePassword = "";
      disableCode = "";
      disableOpen = false;
      await load();
      notify("Two-factor authentication turned off.");
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      busy = "";
    }
  }

  async function regenerateBackupCodes() {
    if (!backupPassword || !backupCode || busy === "backup") return;
    busy = "backup";
    try {
      const result: BackupCodes = await api.createBackupCodes(
        backupPassword,
        backupCode.trim(),
      );
      backupCodes = result.codes;
      if (twofa) twofa = { ...twofa, backup_codes_remaining: result.codes.length };
      backupPassword = "";
      backupCode = "";
      backupOpen = false;
      notify("New backup codes generated.");
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      busy = "";
    }
  }

  async function signOutSession(session: AuthSession) {
    const confirmed = await confirmAction({
      title: "Sign out this device?",
      body: `${deviceName(session)} will need to sign in again.`,
      confirmLabel: "Sign out device",
      danger: true,
    });
    if (!confirmed) return;
    busy = session.id;
    try {
      await api.revokeSession(session.id, sessionsCode.trim() || undefined);
      sessionsCode = "";
      await load();
      notify("Device signed out.");
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      busy = "";
    }
  }

  async function signOutAll() {
    const confirmed = await confirmAction({
      title: "Sign out all other devices?",
      body: "Every other active session will need to sign in again.",
      confirmLabel: "Sign out all",
      danger: true,
    });
    if (!confirmed) return;
    busy = "all";
    try {
      await api.revokeAllSessions(sessionsCode.trim() || undefined);
      sessionsCode = "";
      await load();
      notify("Other devices signed out.");
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      busy = "";
    }
  }

  async function clearHistory() {
    const confirmed = await confirmAction({
      title: "Clear security history?",
      body: "This removes the recorded security activity for your account.",
      confirmLabel: "Clear history",
      danger: true,
    });
    if (!confirmed) return;
    busy = "clear";
    try {
      await api.clearSecurityEvents(eventsCode.trim() || undefined);
      eventsCode = "";
      await load();
      notify("Security history cleared.");
    } catch (cause) {
      notify(cause instanceof Error ? cause.message : String(cause));
    } finally {
      busy = "";
    }
  }

  function osName(userAgent?: string | null) {
    if (!userAgent) return "Unknown";
    if (/Windows/.test(userAgent)) return "Windows";
    if (/Android/.test(userAgent)) return "Android";
    if (/iPhone|iPad|iPod/.test(userAgent)) return "iOS";
    if (/Mac OS X|Macintosh/.test(userAgent)) return "macOS";
    if (/CrOS/.test(userAgent)) return "ChromeOS";
    if (/Linux/.test(userAgent)) return "Linux";
    return "Unknown";
  }

  function clientName(userAgent?: string | null) {
    if (!userAgent) return "Unknown client";
    if (userAgent.includes("FlackleDesktop")) return "Flackle client";
    if (/Edg\//.test(userAgent)) return "Edge";
    if (/OPR\/|Opera/.test(userAgent)) return "Opera";
    if (/Firefox\//.test(userAgent)) return "Firefox";
    if (/Chrome\//.test(userAgent)) return "Chrome";
    if (/Safari\//.test(userAgent)) return "Safari";
    return "Browser";
  }

  function deviceName(session: AuthSession) {
    return `${clientName(session.user_agent)} on ${osName(session.user_agent)}`;
  }

  function eventLabel(type: string) {
    return eventLabels[type] || type.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
  }

  function timeLabel(value?: string | null) {
    if (!value) return "Unknown time";
    return new Intl.DateTimeFormat(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    }).format(new Date(value));
  }

  function qrSource(value?: string | null) {
    if (!value) return null;
    if (value.startsWith("data:")) return value;
    if (/^https?:\/\//.test(value)) return value;
    if (value.includes("<svg")) {
      return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(value)}`;
    }
    return `data:image/png;base64,${value}`;
  }

  function copy(value: string, label: string) {
    void navigator.clipboard
      .writeText(value)
      .then(() => notify(`${label} copied.`))
      .catch(() => notify(`Could not copy ${label.toLowerCase()}.`));
  }
</script>

<div class="security">
  {#if error}
    <p class="error">{error}</p>
  {/if}

  <section class="block">
    <div class="block-head">
      <div>
        <h3>Two-factor authentication</h3>
        <p>Add a second step when signing in to this account.</p>
      </div>
      <span class="status" class:on={twofa?.enabled}>
        {#if loading}Checking…{:else if twofa?.enabled}On{:else if twofa?.pending}Pending{:else}Off{/if}
      </span>
    </div>

    {#if setupStep === "password"}
      <div class="inline-form">
        <p class="form-copy">
          Enter your current password to start setting up two-factor authentication.
        </p>
        <input
          type="password"
          bind:value={setupPassword}
          autocomplete="current-password"
          placeholder="Current password"
          on:keydown={(event) => {
            if (event.key === "Enter") void startSetup();
          }}
        />
        <div class="form-actions">
          <button class="secondary" on:click={cancelSetup}>Cancel</button>
          <button
            class="primary"
            disabled={!setupPassword || busy === "setup"}
            on:click={startSetup}
          >
            {busy === "setup" ? "Starting…" : "Continue"}
          </button>
        </div>
      </div>
    {:else if setupStep === "code" && setupData}
      <div class="setup">
        <div class="setup-grid">
          {#if qrSource(setupData.qr_code)}
            <img class="qr" src={qrSource(setupData.qr_code) ?? ""} alt="Two-factor setup QR code" />
          {:else}
            <div class="qr qr-empty"><Icon name="lock" size={28} /></div>
          {/if}
          <div class="setup-copy">
            <b>Scan the code with your authenticator app</b>
            <p>If you cannot scan it, enter this secret manually:</p>
            <button class="secret" on:click={() => copy(setupData?.secret ?? "", "Secret")}>
              <code>{setupData.secret}</code>
              <Icon name="copy" size={12} />
            </button>
            {#if setupData.otpauth_url}
              <button
                class="link-button"
                on:click={() => copy(setupData?.otpauth_url ?? "", "Setup link")}
              >
                Copy setup link
              </button>
            {/if}
          </div>
        </div>
        <p class="form-copy">Enter the six-digit code from your authenticator to finish.</p>
        <input
          class="code-input"
          bind:value={setupCode}
          maxlength="6"
          inputmode="numeric"
          autocomplete="one-time-code"
          placeholder="000000"
          on:keydown={(event) => {
            if (event.key === "Enter") void confirmSetup();
          }}
        />
        <div class="form-actions">
          <button class="secondary" on:click={cancelSetup}>Cancel</button>
          <button
            class="primary"
            disabled={!setupCode.trim() || busy === "confirm"}
            on:click={confirmSetup}
          >
            {busy === "confirm" ? "Verifying…" : "Enable"}
          </button>
        </div>
      </div>
    {:else if twofa?.enabled}
      <div class="block-body">
        <p class="form-copy">
          Two-factor authentication is on.
          {twofa.backup_codes_remaining ?? 0} backup code{(twofa.backup_codes_remaining ?? 0) === 1 ? "" : "s"} remaining.
        </p>
        {#if backupCodes.length > 0}
          <div class="backup-codes">
            <div class="backup-head">
              <b>Save these backup codes</b>
              <button class="link-button" on:click={() => copy(backupCodes.join("\n"), "Backup codes")}>
                Copy all
              </button>
            </div>
            <div class="codes">
              {#each backupCodes as code (code)}<code>{code}</code>{/each}
            </div>
          </div>
        {:else if backupOpen}
          <div class="inline-form compact">
            <p class="form-copy">Confirm your password and current code to generate new backup codes.</p>
            <input type="password" bind:value={backupPassword} placeholder="Current password" autocomplete="current-password" />
            <input class="code-input" bind:value={backupCode} maxlength="6" placeholder="000000" inputmode="numeric" />
            <div class="form-actions">
              <button class="secondary" on:click={() => (backupOpen = false)}>Cancel</button>
              <button class="primary" disabled={!backupPassword || !backupCode || busy === "backup"} on:click={regenerateBackupCodes}>
                {busy === "backup" ? "Generating…" : "Generate codes"}
              </button>
            </div>
          </div>
        {:else if disableOpen}
          <div class="inline-form compact">
            <p class="form-copy">Confirm your password and a current code to turn it off.</p>
            <input type="password" bind:value={disablePassword} placeholder="Current password" autocomplete="current-password" />
            <input class="code-input" bind:value={disableCode} maxlength="32" placeholder={useBackupCode ? "Backup code" : "000000"} />
            <label class="check-row">
              <input type="checkbox" bind:checked={useBackupCode} />
              Use a backup code
            </label>
            <div class="form-actions">
              <button class="secondary" on:click={() => (disableOpen = false)}>Cancel</button>
              <button class="danger" disabled={!disablePassword || !disableCode || busy === "disable"} on:click={disableTwoFactor}>
                {busy === "disable" ? "Turning off…" : "Turn off"}
              </button>
            </div>
          </div>
        {:else}
          <div class="actions">
            <button class="secondary" on:click={() => { backupOpen = true; backupPassword = ""; backupCode = ""; }}>
              Regenerate backup codes
            </button>
            <button class="danger-outline" on:click={() => { disableOpen = true; disablePassword = ""; disableCode = ""; }}>
              Turn off
            </button>
          </div>
        {/if}
      </div>
    {:else}
      <div class="block-body">
        <p class="form-copy">Protect your account with an authenticator app. You can also generate backup codes.</p>
        <button class="primary" on:click={() => { setupStep = "password"; setupPassword = ""; }}>
          Turn on two-factor authentication
        </button>
      </div>
    {/if}
  </section>

  <section class="block">
    <div class="block-head">
      <div>
        <h3>Active sessions</h3>
        <p>Devices currently signed in to your account.</p>
      </div>
      <div class="head-actions">
        {#if twofa?.enabled}
          <input class="code-input small" bind:value={sessionsCode} maxlength="6" placeholder="2FA code" inputmode="numeric" />
        {/if}
        <button class="secondary small" on:click={() => (showIps = !showIps)}>
          {showIps ? "Hide IPs" : "Show IPs"}
        </button>
        <button class="danger-outline small" disabled={busy === "all" || otherSessions.length === 0} on:click={signOutAll}>
          Sign out all
        </button>
      </div>
    </div>
    <div class="session-list">
      {#if loading}
        <p class="empty">Loading sessions…</p>
      {:else if sessions.length === 0}
        <p class="empty">No active sessions.</p>
      {:else}
        {#if currentSession}
          <div class="session current">
            <span class="session-icon"><Icon name="check" size={14} /></span>
            <div class="session-copy">
              <div class="session-title"><b>{deviceName(currentSession)}</b><span class="tag">Current</span></div>
              <p>
                {currentSession.location || "Unknown location"} · Last used {timeLabel(currentSession.last_used_at || currentSession.created_at)}
                {#if showIps && currentSession.ip_address} · {currentSession.ip_address}{/if}
              </p>
            </div>
          </div>
        {/if}
        {#each otherSessions as session (session.id)}
          <div class="session">
            <span class="session-icon"><Icon name="server" size={14} /></span>
            <div class="session-copy">
              <div class="session-title"><b>{deviceName(session)}</b></div>
              <p>
                {session.location || "Unknown location"} · Last used {timeLabel(session.last_used_at || session.created_at)}
                {#if showIps && session.ip_address} · {session.ip_address}{/if}
              </p>
            </div>
            <button
              class="sign-out"
              title={`Sign out ${deviceName(session)}`}
              aria-label={`Sign out ${deviceName(session)}`}
              disabled={busy === session.id}
              on:click={() => signOutSession(session)}
            >
              {busy === session.id ? "Signing out…" : "Sign out"}
            </button>
          </div>
        {/each}
      {/if}
    </div>
  </section>

  <section class="block">
    <div class="block-head">
      <div>
        <h3>Recent security activity</h3>
        <p>Sign-ins, password changes, and unusual session activity.</p>
      </div>
      <div class="head-actions">
        {#if twofa?.enabled}
          <input class="code-input small" bind:value={eventsCode} maxlength="6" placeholder="2FA code" inputmode="numeric" />
        {/if}
        <button class="secondary small" disabled={busy === "clear" || events.length === 0} on:click={clearHistory}>
          Clear history
        </button>
      </div>
    </div>
    <div class="event-list">
      {#if loading}
        <p class="empty">Loading activity…</p>
      {:else if events.length === 0}
        <p class="empty">No recent security activity.</p>
      {:else}
        {#each events as event, index (event.created_at + ":" + index)}
          <div class="event" class:unusual={event.unusual}>
            <span class="event-icon"><Icon name={event.unusual ? "alert" : "shield"} size={14} /></span>
            <div class="event-copy">
              <div class="event-title">
                <b>{eventLabel(event.event_type)}</b>
                {#if event.unusual}
                  <span class="risk">{event.unusual_reason === "network" ? "Unrecognised network" : "Unusual location"}</span>
                {/if}
              </div>
              <p>
                {clientName(event.user_agent)} on {osName(event.user_agent)} · {timeLabel(event.created_at)}
                {#if event.origin_location} · {event.origin_location}{/if}
              </p>
            </div>
          </div>
        {/each}
      {/if}
    </div>
  </section>
</div>

<style>
  .security {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .error {
    margin: 0;
    padding: 9px 11px;
    border: 1px solid rgba(var(--accent-rgb), 0.24);
    border-radius: 8px;
    background: rgba(var(--accent-rgb), 0.08);
    color: #ff9a9a;
    font-size: 10.5px;
  }
  .block {
    border: 1px solid var(--line);
    border-radius: 11px;
    background: var(--panel-2);
    overflow: hidden;
  }
  .block-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 14px;
    padding: 12px 13px;
    border-bottom: 1px solid var(--line);
  }
  .block-head h3 {
    margin: 0;
    font-size: 12.5px;
    font-weight: 650;
    color: var(--text);
  }
  .block-head p {
    margin: 2px 0 0;
    font-size: 9.5px;
    line-height: 1.4;
    color: var(--muted);
  }
  .status {
    flex: none;
    border: 1px solid var(--line);
    border-radius: 99px;
    padding: 2px 8px;
    font-size: 9px;
    font-weight: 650;
    color: var(--muted);
    background: var(--panel-3);
  }
  .status.on {
    color: var(--green);
    border-color: rgba(52, 211, 153, 0.28);
    background: rgba(52, 211, 153, 0.08);
  }
  .block-body,
  .inline-form,
  .setup {
    padding: 12px 13px;
  }
  .form-copy {
    margin: 0 0 9px;
    font-size: 10px;
    line-height: 1.45;
    color: var(--muted);
  }
  .inline-form {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .inline-form.compact {
    padding: 0;
    margin-top: 10px;
  }
  .inline-form input:not([type="checkbox"]),
  .setup input {
    width: 100%;
    height: 34px;
    border: 1px solid var(--line-strong);
    border-radius: 8px;
    background: var(--input);
    color: var(--text);
    padding: 0 9px;
    font-size: 11.5px;
    outline: none;
  }
  .inline-form input:focus,
  .setup input:focus {
    border-color: rgba(var(--accent-rgb), 0.4);
    box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.06);
  }
  .code-input {
    font-family: "JetBrains Mono", ui-monospace, monospace;
    letter-spacing: 0.14em;
    text-align: center;
  }
  .code-input.small {
    width: 84px;
    height: 27px;
    font-size: 10px;
    letter-spacing: 0.08em;
  }
  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 2px;
  }
  .primary,
  .secondary,
  .danger,
  .danger-outline,
  .sign-out,
  .link-button {
    border-radius: 7px;
    font-size: 9.5px;
    font-weight: 650;
    height: 29px;
    padding: 0 10px;
  }
  .primary {
    border: 0;
    background: var(--accent);
    color: white;
  }
  .primary:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 86%, white);
  }
  .secondary {
    border: 1px solid var(--line);
    background: var(--panel-3);
    color: var(--text-dim);
  }
  .secondary:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  .secondary.small,
  .danger-outline.small {
    height: 27px;
    font-size: 9px;
  }
  .danger,
  .danger-outline {
    border: 1px solid rgba(var(--accent-rgb), 0.3);
    background: rgba(var(--accent-rgb), 0.1);
    color: #ff9a9a;
  }
  .danger:hover:not(:disabled),
  .danger-outline:hover:not(:disabled) {
    background: rgba(var(--accent-rgb), 0.18);
  }
  .primary:disabled,
  .secondary:disabled,
  .danger:disabled,
  .danger-outline:disabled,
  .sign-out:disabled {
    opacity: 0.38;
    cursor: default;
  }
  .actions,
  .head-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .head-actions {
    justify-content: flex-end;
  }
  .setup-grid {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    margin-bottom: 10px;
  }
  .qr {
    width: 112px;
    height: 112px;
    flex: none;
    border-radius: 9px;
    background: white;
    object-fit: contain;
    padding: 5px;
  }
  .qr-empty {
    display: grid;
    place-items: center;
    background: var(--panel-3);
    color: var(--muted);
    padding: 0;
  }
  .setup-copy {
    min-width: 0;
    flex: 1;
  }
  .setup-copy > b {
    display: block;
    font-size: 10.5px;
    font-weight: 650;
  }
  .setup-copy > p {
    margin: 2px 0 6px;
    font-size: 9.5px;
    color: var(--muted);
  }
  .secret {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--input);
    color: var(--text-dim);
    padding: 6px 8px;
    text-align: left;
  }
  .secret code {
    font-family: "JetBrains Mono", ui-monospace, monospace;
    font-size: 9.5px;
    word-break: break-all;
  }
  .link-button {
    border: 0;
    background: transparent;
    color: var(--muted);
    text-decoration: underline;
    text-underline-offset: 2px;
    padding: 0;
    height: auto;
  }
  .link-button:hover {
    color: var(--text);
  }
  .check-row {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 9.5px;
    color: var(--muted);
  }
  .backup-codes {
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--input);
    padding: 9px;
  }
  .backup-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 7px;
    font-size: 10px;
    color: var(--text-dim);
  }
  .codes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
  }
  .codes code {
    border: 1px solid var(--line);
    border-radius: 4px;
    background: var(--panel-2);
    padding: 4px 6px;
    text-align: center;
    font-family: "JetBrains Mono", ui-monospace, monospace;
    font-size: 9px;
    color: var(--text-dim);
  }
  .session-list,
  .event-list {
    display: flex;
    flex-direction: column;
  }
  .session,
  .event {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 9px 13px;
    min-height: 48px;
  }
  .session + .session,
  .event + .event {
    border-top: 1px solid var(--line);
  }
  .session.current {
    background: rgba(var(--accent-rgb), 0.045);
  }
  .session-icon,
  .event-icon {
    width: 28px;
    height: 28px;
    flex: none;
    border-radius: 7px;
    background: var(--panel-3);
    color: var(--muted);
    display: grid;
    place-items: center;
  }
  .session-copy,
  .event-copy {
    flex: 1;
    min-width: 0;
  }
  .session-title,
  .event-title {
    display: flex;
    align-items: center;
    gap: 5px;
    flex-wrap: wrap;
    line-height: 1.25;
  }
  .session-title b,
  .event-title b {
    font-size: 10.5px;
    font-weight: 600;
    color: var(--text);
  }
  .session-copy p,
  .event-copy p {
    margin: 2px 0 0;
    font-size: 9px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tag,
  .risk {
    font-size: 8px;
    font-weight: 650;
    border-radius: 99px;
    padding: 1px 5px;
  }
  .tag {
    color: var(--green);
    background: rgba(52, 211, 153, 0.08);
    border: 1px solid rgba(52, 211, 153, 0.22);
  }
  .risk {
    color: #ffb07a;
    background: rgba(242, 163, 60, 0.09);
    border: 1px solid rgba(242, 163, 60, 0.22);
  }
  .sign-out {
    flex: none;
    height: 25px;
    border: 1px solid var(--line);
    background: var(--panel-3);
    color: var(--muted);
    padding: 0 7px;
    font-size: 8.5px;
  }
  .sign-out:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  .event.unusual .event-icon {
    color: #ffb07a;
    background: rgba(242, 163, 60, 0.08);
  }
  .empty {
    margin: 0;
    padding: 14px 13px;
    color: var(--muted);
    font-size: 10px;
  }
</style>
