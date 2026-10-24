<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "../api";
  import {
    accentOptions,
    formatKeybind,
    keybindDefinitions,
    keybindFromEvent,
    keybindTokens,
    prefs,
    resetAllKeybinds,
    resetKeybind,
    setKeybind,
    setPreference,
    type AccentName,
    type Density,
    type KeybindAction,
    type SendKey,
    type TextSize,
    type TimeFormat,
  } from "../preferences";
  import { setTheme, theme, type Theme } from "../theme";
  import { confirmAction } from "../dialog";
  import { me, modal, notify, setMyStatus, showMembers } from "../workspace";
  import Avatar from "./Avatar.svelte";
  import Icon from "./Icon.svelte";
  import Mark from "./Mark.svelte";
  import SecuritySettings from "./SecuritySettings.svelte";

  type Tab = "account" | "security" | "appearance" | "notifications" | "behavior" | "about";

  const tabs: Array<{ id: Tab; label: string; icon: string; title: string; subtitle: string }> = [
    {
      id: "account",
      label: "My account",
      icon: "users",
      title: "My account",
      subtitle: "Your identity, presence, and session.",
    },
    {
      id: "security",
      label: "Security",
      icon: "lock",
      title: "Security",
      subtitle: "Two-factor authentication, active sessions, and recent activity.",
    },
    {
      id: "appearance",
      label: "Appearance",
      icon: "palette",
      title: "Appearance",
      subtitle: "Make the client feel like yours.",
    },
    {
      id: "notifications",
      label: "Notifications",
      icon: "bell",
      title: "Notifications",
      subtitle: "Decide what deserves your attention.",
    },
    {
      id: "behavior",
      label: "Behavior",
      icon: "sliders",
      title: "Behavior",
      subtitle: "How messages and timestamps behave.",
    },
    {
      id: "about",
      label: "About",
      icon: "command",
      title: "About",
      subtitle: "What this app is, and what it is not.",
    },
  ];

  let tab: Tab = "account";
  let status = $me?.status ?? "online";
  let signingOut = false;
  let capturing: KeybindAction | null = null;

  $: currentTab = tabs.find((item) => item.id === tab) ?? tabs[0];

  onMount(() => {
    const capture = (event: KeyboardEvent) => {
      if (!capturing) return;
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape") {
        capturing = null;
        return;
      }
      const binding = keybindFromEvent(event);
      if (!binding) return;
      if (!(event.metaKey || event.ctrlKey || event.altKey)) {
        capturing = null;
        notify("Include Ctrl/⌘ or Alt in the shortcut.");
        return;
      }
      const conflict = keybindDefinitions.find(
        (definition) =>
          definition.id !== capturing && $prefs.keybinds[definition.id] === binding,
      );
      if (conflict) {
        capturing = null;
        notify(`${conflict.label} already uses that shortcut.`);
        return;
      }
      setKeybind(capturing, binding);
      capturing = null;
      notify("Keybind updated.");
    };
    window.addEventListener("keydown", capture, true);
    return () => window.removeEventListener("keydown", capture, true);
  });

  function beginCapture(action: KeybindAction) {
    capturing = action;
  }

  function clearKeybind(action: KeybindAction) {
    resetKeybind(action);
    if (capturing === action) capturing = null;
  }

  function resetKeybinds() {
    resetAllKeybinds();
    capturing = null;
    notify("Keybinds reset.");
  }

  const statuses = [
    { id: "online", label: "Online", color: "var(--green)" },
    { id: "idle", label: "Idle", color: "var(--yellow)" },
    { id: "dnd", label: "Do not disturb", color: "var(--accent)" },
    { id: "invisible", label: "Invisible", color: "#6b7280" },
  ];

  async function chooseStatus(next: string) {
    status = next;
    await setMyStatus(next);
  }

  async function signOut() {
    if (signingOut) return;
    const confirmed = await confirmAction({
      title: "Sign out of Flackle Desktop?",
      body: "Your saved session will be removed from this device.",
      confirmLabel: "Sign out",
      danger: true,
    });
    if (!confirmed) return;
    signingOut = true;
    try {
      await api.logout();
      location.reload();
    } catch {
      signingOut = false;
    }
  }

  function chooseTheme(next: Theme) {
    setTheme(next);
  }

  function chooseAccent(next: AccentName) {
    setPreference("accent", next);
  }

  function chooseDensity(next: Density) {
    setPreference("density", next);
  }

  function chooseTextSize(next: TextSize) {
    setPreference("textSize", next);
  }

  function chooseTimeFormat(next: TimeFormat) {
    setPreference("timeFormat", next);
  }

  function chooseSendKey(next: SendKey) {
    setPreference("sendKey", next);
  }

  function toggleMemberStart() {
    const next = !$prefs.startWithMembers;
    setPreference("startWithMembers", next);
    showMembers.set(next);
  }

  function resetAppearance() {
    prefs.update((current) => ({
      ...current,
      accent: "ember",
      density: "cozy",
      textSize: "default",
      reduceMotion: false,
      startWithMembers: true,
    }));
    setTheme("dark");
    showMembers.set(true);
  }

  function open(url: string) {
    void api.openExternal(url);
  }
</script>

<div
  class="backdrop"
  role="presentation"
  on:click={() => modal.set(null)}
>
  <div class="modal" role="dialog" aria-label="Settings" tabindex="-1" on:click|stopPropagation>
    <aside class="nav">
      <div class="nav-head">
        <span class="nav-mark"><Mark size={18} /></span>
        <div>
          <b>Settings</b>
          <small>Flackle Desktop</small>
        </div>
      </div>

      {#each tabs as item}
        <button class:active={tab === item.id} on:click={() => (tab = item.id)}>
          <span class="nav-icon"><Icon name={item.icon} size={15} /></span>
          {item.label}
        </button>
      {/each}

      <div class="nav-spacer"></div>

      <button class="nav-signout" on:click={signOut} disabled={signingOut}>
        <span class="nav-icon"><Icon name="alert" size={15} /></span>
        {signingOut ? "Signing out…" : "Sign out"}
      </button>
      <p class="nav-version">v0.1.0 · MIT</p>
    </aside>

    <section class="panel">
      <header class="panel-head">
        <div>
          <h2>{currentTab.title}</h2>
          <p>{currentTab.subtitle}</p>
        </div>
        <button class="icon-btn" aria-label="Close settings" on:click={() => modal.set(null)}>
          <Icon name="close" size={18} />
        </button>
      </header>

      <div class="panel-body">
        {#if tab === "account"}
          <div class="profile">
            <Avatar user={$me} size={58} />
            <div class="profile-copy">
              <h3>{$me?.display_name || $me?.username}</h3>
              <span>@{$me?.username}</span>
            </div>
          </div>

          <div class="field">
            <span class="field-label">Email</span>
            <div class="readonly">{$me?.email || "—"}</div>
          </div>

          <div class="section-title">Presence</div>
          <p class="section-note">
            Presence is shared with people who can already see you. It syncs to the web client too.
          </p>
          <div class="status-grid">
            {#each statuses as option}
              <button
                class:selected={status === option.id}
                on:click={() => chooseStatus(option.id)}
              >
                <i style={`background:${option.color}`}></i>
                {option.label}
                {#if status === option.id}<Icon name="check" size={13} />{/if}
              </button>
            {/each}
          </div>

          <div class="section-title">Session</div>
          <div class="session">
            <span class="session-icon"><Icon name="check" size={16} /></span>
            <div>
              <b>Stored in your OS credential vault</b>
              <p>Tokens stay encrypted outside the web layer.</p>
            </div>
          </div>
        {:else if tab === "security"}
          <SecuritySettings />
        {:else if tab === "appearance"}
          <div class="section-title first">Theme</div>
          <div class="theme-grid">
            <button class:selected={$theme === "dark"} on:click={() => chooseTheme("dark")}>
              <span class="preview dark">
                <i class="rail"></i>
                <i class="side"></i>
                <i class="chat"></i>
              </span>
              <span class="theme-name">Graphite <small>Dark</small></span>
              {#if $theme === "dark"}<span class="theme-check"><Icon name="check" size={13} /></span>{/if}
            </button>
            <button class:selected={$theme === "light"} on:click={() => chooseTheme("light")}>
              <span class="preview light">
                <i class="rail"></i>
                <i class="side"></i>
                <i class="chat"></i>
              </span>
              <span class="theme-name">Paper <small>Light</small></span>
              {#if $theme === "light"}<span class="theme-check"><Icon name="check" size={13} /></span>{/if}
            </button>
            <button class:selected={$theme === "system"} on:click={() => chooseTheme("system")}>
              <span class="preview system">
                <i class="rail"></i>
                <i class="side"></i>
                <i class="chat"></i>
              </span>
              <span class="theme-name">System <small>Match OS</small></span>
              {#if $theme === "system"}<span class="theme-check"><Icon name="check" size={13} /></span>{/if}
            </button>
          </div>

          <div class="section-title">Accent</div>
          <div class="accent-row">
            {#each accentOptions as option}
              <button
                class="accent-dot"
                class:selected={$prefs.accent === option.id}
                style={`--dot:rgb(${option.rgb})`}
                title={option.label}
                aria-label={option.label}
                on:click={() => chooseAccent(option.id)}
              >
                {#if $prefs.accent === option.id}<Icon name="check" size={13} />{/if}
              </button>
            {/each}
          </div>

          <div class="section-title">Layout</div>
          <div class="setting-rows">
            <div class="setting-row">
              <div>
                <b>Density</b>
                <p>How tightly messages stack.</p>
              </div>
              <div class="segmented">
                <button
                  class:selected={$prefs.density === "cozy"}
                  on:click={() => chooseDensity("cozy")}>Cozy</button
                >
                <button
                  class:selected={$prefs.density === "compact"}
                  on:click={() => chooseDensity("compact")}>Compact</button
                >
              </div>
            </div>
            <div class="setting-row">
              <div>
                <b>Message text</b>
                <p>Scale the conversation without changing the rest of the UI.</p>
              </div>
              <div class="segmented">
                <button
                  class:selected={$prefs.textSize === "small"}
                  on:click={() => chooseTextSize("small")}>Small</button
                >
                <button
                  class:selected={$prefs.textSize === "default"}
                  on:click={() => chooseTextSize("default")}>Default</button
                >
                <button
                  class:selected={$prefs.textSize === "large"}
                  on:click={() => chooseTextSize("large")}>Large</button
                >
              </div>
            </div>
            <div class="setting-row">
              <div>
                <b>Reduce motion</b>
                <p>Turn off animations and smooth scrolling.</p>
              </div>
              <button
                class="toggle"
                class:on={$prefs.reduceMotion}
                aria-pressed={$prefs.reduceMotion}
                aria-label="Reduce motion"
                on:click={() => setPreference("reduceMotion", !$prefs.reduceMotion)}
              ><span></span></button>
            </div>
            <div class="setting-row">
              <div>
                <b>Start with member list</b>
                <p>Open wide server views with the roster visible.</p>
              </div>
              <button
                class="toggle"
                class:on={$prefs.startWithMembers}
                aria-pressed={$prefs.startWithMembers}
                aria-label="Start with member list"
                on:click={toggleMemberStart}
              ><span></span></button>
            </div>
          </div>
          <button class="reset" on:click={resetAppearance}>Reset appearance</button>
        {:else if tab === "notifications"}
          <div class="section-title first">Desktop notifications</div>
          <div class="setting-rows">
            <div class="setting-row">
              <div>
                <b>Native notifications</b>
                <p>Show a system notification when Flackle is in the background.</p>
              </div>
              <button
                class="toggle"
                class:on={$prefs.notifications}
                aria-pressed={$prefs.notifications}
                aria-label="Native notifications"
                on:click={() => setPreference("notifications", !$prefs.notifications)}
              ><span></span></button>
            </div>
            <div class="setting-row" class:disabled={!$prefs.notifications}>
              <div>
                <b>Only mentions and DMs</b>
                <p>Skip notifications for ordinary channel messages.</p>
              </div>
              <button
                class="toggle"
                class:on={$prefs.mentionsOnly}
                aria-pressed={$prefs.mentionsOnly}
                aria-label="Only mentions and DMs"
                disabled={!$prefs.notifications}
                on:click={() => setPreference("mentionsOnly", !$prefs.mentionsOnly)}
              ><span></span></button>
            </div>
          </div>
          <p class="section-note">
            Notifications are only sent when the Flackle window is hidden or not focused.
          </p>
        {:else if tab === "behavior"}
          <div class="section-title first">Messages</div>
          <div class="setting-rows">
            <div class="setting-row">
              <div>
                <b>Send with</b>
                <p>Choose whether Enter sends or adds a new line.</p>
              </div>
              <div class="segmented">
                <button
                  class:selected={$prefs.sendKey === "enter"}
                  on:click={() => chooseSendKey("enter")}>Enter</button
                >
                <button
                  class:selected={$prefs.sendKey === "mod-enter"}
                  on:click={() => chooseSendKey("mod-enter")}>Mod+Enter</button
                >
              </div>
            </div>
            <div class="setting-row">
              <div>
                <b>Time format</b>
                <p>Use a twelve or twenty-four hour clock.</p>
              </div>
              <div class="segmented">
                <button
                  class:selected={$prefs.timeFormat === "12h"}
                  on:click={() => chooseTimeFormat("12h")}>12-hour</button
                >
                <button
                  class:selected={$prefs.timeFormat === "24h"}
                  on:click={() => chooseTimeFormat("24h")}>24-hour</button
                >
              </div>
            </div>
            <div class="setting-row">
              <div>
                <b>Typing indicators</b>
                <p>Show when other people are typing in the open channel.</p>
              </div>
              <button
                class="toggle"
                class:on={$prefs.showTyping}
                aria-pressed={$prefs.showTyping}
                aria-label="Typing indicators"
                on:click={() => setPreference("showTyping", !$prefs.showTyping)}
              ><span></span></button>
            </div>
          </div>

          <div class="section-title">Keybinds</div>
          <p class="section-note">
            Click Change, then press the key combination. Escape cancels. Keybinds work anywhere in the app.
          </p>
          <div class="keybind-rows">
            {#each keybindDefinitions as definition (definition.id)}
              <div class="keybind-row">
                <div>
                  <b>{definition.label}</b>
                  <p>{definition.description}</p>
                </div>
                <div class="keybind-actions">
                  <div class="keycaps" aria-label={formatKeybind($prefs.keybinds[definition.id])}>
                    {#each keybindTokens($prefs.keybinds[definition.id]) as token}
                      <kbd>{token}</kbd>
                    {/each}
                  </div>
                  <button
                    class="keybind-change"
                    class:capturing={capturing === definition.id}
                    on:click={() => beginCapture(definition.id)}
                  >
                    {capturing === definition.id ? "Press keys…" : "Change"}
                  </button>
                  <button
                    class="keybind-reset"
                    title="Reset to default"
                    aria-label={`Reset ${definition.label} keybind`}
                    on:click={() => clearKeybind(definition.id)}
                  >
                    <Icon name="reload" size={13} />
                  </button>
                </div>
              </div>
            {/each}
          </div>
          <button class="reset" on:click={resetKeybinds}>Reset all keybinds</button>
        {:else}
          <div class="about-hero">
            <span class="about-mark"><Mark size={38} /></span>
            <div>
              <h3>Flackle Desktop</h3>
              <span>Version 0.1.0 · “Ember”</span>
            </div>
          </div>

          <p class="about-copy">
            A native desktop client for Flackle, written in Rust with a Tauri shell and a
            Svelte interface. It talks to Flackle's REST and gateway APIs directly — no
            browser shim, no bundled Chromium.
          </p>

          <div class="specs">
            <div><span>Core</span><b>flackle-core 0.1.0</b></div>
            <div><span>Shell</span><b>Tauri 2 · WebView2 / WebKitGTK / WKWebView</b></div>
            <div><span>Interface</span><b>Svelte 5 · TypeScript</b></div>
            <div><span>Protocol</span><b>REST + NDJSON gateway</b></div>
          </div>

          <div class="section-title">Links</div>
          <div class="link-rows">
            <button on:click={() => open("https://github.com/gaxolotl/flackle-selfbot")}>
              <span><b>flackle-selfbot</b><small>The MIT-licensed library this port is based on</small></span>
              <Icon name="reply" size={14} />
            </button>
            <button on:click={() => open("https://flackle.com")}>
              <span><b>flackle.com</b><small>Open Flackle in your browser</small></span>
              <Icon name="reply" size={14} />
            </button>
          </div>

          <div class="disclaimer">
            <Icon name="alert" size={15} />
            <p>
              Unofficial client. Not affiliated with Flackle or Folkward LLC. Use it on
              your own account and only where selfbots are permitted. Voice audio is not
              available because the upstream API does not expose an SFU.
            </p>
          </div>
        {/if}
      </div>
    </section>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 120;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(8px);
    display: grid;
    place-items: center;
    padding: 28px;
  }
  .modal {
    width: min(780px, calc(100vw - 56px));
    height: min(620px, calc(100vh - 56px));
    background: var(--card);
    border: 1px solid var(--line-strong);
    border-radius: 17px;
    box-shadow: var(--shadow);
    display: flex;
    overflow: hidden;
    animation: rise 160ms ease-out;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.995);
    }
  }
  .nav {
    width: 194px;
    flex: none;
    background: var(--sidebar);
    border-right: 1px solid var(--line);
    padding: 14px 9px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .nav-head {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 4px 8px 16px;
    border-bottom: 1px solid var(--line);
    margin-bottom: 10px;
  }
  .nav-mark {
    width: 30px;
    height: 30px;
    border-radius: 9px;
    background: var(--accent-dim);
    color: var(--accent);
    display: grid;
    place-items: center;
    border: 1px solid rgba(var(--accent-rgb), 0.18);
  }
  .nav-head div {
    display: flex;
    flex-direction: column;
    gap: 0;
  }
  .nav-head b {
    font-size: 12.5px;
    font-weight: 650;
  }
  .nav-head small {
    font-size: 9.5px;
    color: var(--muted);
  }
  .nav > button {
    height: 34px;
    border: 0;
    background: transparent;
    border-radius: 7px;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 9px;
    color: var(--muted);
    font-size: 12px;
    text-align: left;
  }
  .nav > button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .nav > button.active {
    background: var(--active);
    color: var(--text);
  }
  .nav-icon {
    width: 22px;
    display: grid;
    place-items: center;
    color: var(--muted);
  }
  .nav > button.active .nav-icon {
    color: var(--accent);
  }
  .nav-spacer {
    flex: 1;
  }
  .nav > button.nav-signout {
    color: #b06a6a;
  }
  .nav > button.nav-signout:hover {
    color: #ff8a8a;
    background: rgba(var(--accent-rgb), 0.08);
  }
  .nav-version {
    font-size: 8.5px;
    color: #555b6a;
    padding: 8px 9px 0;
    margin: 0;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .panel {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .panel-head {
    height: 72px;
    flex: none;
    border-bottom: 1px solid var(--line);
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 18px 18px 0 22px;
  }
  .panel-head h2 {
    margin: 0;
    font-size: 17px;
    letter-spacing: -0.03em;
  }
  .panel-head p {
    margin: 2px 0 0;
    font-size: 11px;
    color: var(--muted);
  }
  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: 20px 22px 26px;
  }
  .profile {
    display: flex;
    align-items: center;
    gap: 14px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    border-radius: 13px;
    padding: 14px;
  }
  .profile-copy {
    flex: 1;
    min-width: 0;
  }
  .profile-copy h3 {
    margin: 0;
    font-size: 17px;
    letter-spacing: -0.025em;
  }
  .profile-copy span {
    font-size: 11.5px;
    color: var(--muted);
  }
  .field {
    margin-top: 14px;
  }
  .field-label {
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .readonly {
    margin-top: 5px;
    height: 36px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--input);
    display: flex;
    align-items: center;
    padding: 0 11px;
    font-size: 12px;
    color: var(--text-dim);
  }
  .section-title {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
    margin: 24px 0 8px;
  }
  .section-title.first {
    margin-top: 0;
  }
  .section-note {
    font-size: 11px;
    color: var(--muted);
    margin: 0 0 10px;
  }
  .status-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 7px;
  }
  .status-grid button {
    height: 38px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel-2);
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    color: var(--text-dim);
    font-size: 11.5px;
  }
  .status-grid button:hover {
    border-color: var(--line-strong);
    color: var(--text);
  }
  .status-grid button.selected {
    border-color: rgba(var(--accent-rgb), 0.34);
    background: var(--accent-dim);
    color: var(--text);
  }
  .status-grid button i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex: none;
  }
  .status-grid button :global(svg) {
    margin-left: auto;
    color: var(--accent);
  }
  .session {
    display: flex;
    gap: 11px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    border-radius: 11px;
    padding: 12px;
  }
  .session-icon {
    width: 27px;
    height: 27px;
    border-radius: 50%;
    background: rgba(56, 211, 159, 0.1);
    color: var(--green);
    display: grid;
    place-items: center;
    flex: none;
  }
  .session b {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text);
  }
  .session p {
    margin: 3px 0 0;
    font-size: 10.5px;
    line-height: 1.5;
    color: var(--muted);
  }
  .theme-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 9px;
  }
  .theme-grid button {
    border: 1px solid var(--line);
    background: var(--panel-2);
    border-radius: 11px;
    padding: 8px;
    text-align: left;
    position: relative;
  }
  .theme-grid button:hover {
    border-color: var(--line-strong);
  }
  .theme-grid button.selected {
    border-color: rgba(var(--accent-rgb), 0.4);
    box-shadow: 0 0 0 2px rgba(var(--accent-rgb), 0.08);
  }
  .preview {
    height: 72px;
    border-radius: 7px;
    border: 1px solid var(--line);
    display: grid;
    grid-template-columns: 12px 30px 1fr;
    overflow: hidden;
    margin-bottom: 8px;
  }
  .preview i {
    display: block;
  }
  .preview.dark i.rail {
    background: #08090c;
  }
  .preview.dark i.side {
    background: #0d0f14;
  }
  .preview.dark i.chat {
    background: #0f1116;
  }
  .preview.light i.rail {
    background: #e9e6e0;
  }
  .preview.light i.side {
    background: #f7f5f1;
  }
  .preview.light i.chat {
    background: #fbfaf7;
  }
  .preview.system {
    background: #0f0f14;
  }
  .preview.system i.rail {
    background: #08090c;
  }
  .preview.system i.side {
    background: #0d0f14;
  }
  .preview.system i.chat {
    background: #17171d;
  }
  .theme-name {
    display: flex;
    align-items: baseline;
    gap: 5px;
    font-size: 11.5px;
    font-weight: 600;
  }
  .theme-name small {
    font-size: 9px;
    color: var(--muted);
    font-weight: 400;
  }
  .theme-check {
    position: absolute;
    right: 12px;
    bottom: 13px;
    color: var(--accent);
  }
  .accent-row {
    display: flex;
    align-items: center;
    gap: 9px;
  }
  .accent-dot {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: var(--dot);
    color: white;
    display: grid;
    place-items: center;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.18);
  }
  .accent-dot:hover {
    transform: scale(1.06);
  }
  .accent-dot.selected {
    box-shadow: 0 0 0 2px var(--card), 0 0 0 3px var(--dot);
  }
  .setting-rows {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--line);
    border-radius: 11px;
    overflow: hidden;
  }
  .setting-row {
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 11px 12px;
    background: var(--panel-2);
    min-height: 54px;
  }
  .setting-row + .setting-row {
    border-top: 1px solid var(--line);
  }
  .setting-row > div:first-child {
    flex: 1;
    min-width: 0;
  }
  .setting-row b {
    font-size: 11.5px;
    font-weight: 650;
    color: var(--text);
  }
  .setting-row p {
    margin: 2px 0 0;
    font-size: 10px;
    line-height: 1.45;
    color: var(--muted);
  }
  .setting-row.disabled {
    opacity: 0.48;
  }
  .segmented {
    display: flex;
    flex: none;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--line);
    border-radius: 7px;
    background: var(--input);
  }
  .segmented button {
    height: 25px;
    padding: 0 8px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--muted);
    font-size: 10.5px;
    font-weight: 550;
    white-space: nowrap;
  }
  .segmented button:hover {
    color: var(--text);
  }
  .segmented button.selected {
    background: var(--panel-3);
    color: var(--text);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  }
  .toggle {
    width: 36px;
    height: 21px;
    flex: none;
    border: 0;
    border-radius: 99px;
    background: var(--panel-3);
    padding: 2px;
    display: flex;
    align-items: center;
    transition: background 120ms ease;
  }
  .toggle span {
    width: 17px;
    height: 17px;
    border-radius: 50%;
    background: #8b8b97;
    transition: transform 130ms ease, background 130ms ease;
  }
  .toggle.on {
    background: var(--accent);
  }
  .toggle.on span {
    transform: translateX(15px);
    background: white;
  }
  .toggle:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .keybind-rows {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--line);
    border-radius: 11px;
    overflow: hidden;
  }
  .keybind-row {
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 58px;
    padding: 9px 11px;
    background: var(--panel-2);
  }
  .keybind-row + .keybind-row {
    border-top: 1px solid var(--line);
  }
  .keybind-row > div:first-child {
    flex: 1;
    min-width: 0;
  }
  .keybind-row b {
    display: block;
    font-size: 11.5px;
    font-weight: 650;
    color: var(--text);
  }
  .keybind-row p {
    margin: 2px 0 0;
    font-size: 10px;
    line-height: 1.4;
    color: var(--muted);
  }
  .keybind-actions {
    display: flex;
    align-items: center;
    gap: 5px;
    flex: none;
  }
  .keycaps {
    display: flex;
    align-items: center;
    gap: 3px;
    margin-right: 3px;
  }
  .keycaps kbd {
    min-width: 22px;
    height: 23px;
    padding: 0 6px;
    border: 1px solid var(--line-strong);
    border-bottom-width: 2px;
    border-radius: 5px;
    background: var(--card);
    color: var(--text-dim);
    font-family: "Satoshi", "Manrope", system-ui, sans-serif;
    font-size: 9.5px;
    font-weight: 650;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    white-space: nowrap;
  }
  .keybind-change,
  .keybind-reset {
    height: 25px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--panel-3);
    color: var(--text-dim);
    font-size: 9.5px;
    font-weight: 600;
    padding: 0 8px;
  }
  .keybind-change:hover,
  .keybind-reset:hover {
    background: var(--hover);
    color: var(--text);
  }
  .keybind-change.capturing {
    border-color: rgba(var(--accent-rgb), 0.48);
    background: var(--accent-dim);
    color: var(--accent);
  }
  .keybind-reset {
    width: 25px;
    padding: 0;
    display: grid;
    place-items: center;
  }
  .reset {
    margin-top: 14px;
    padding: 4px 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 10.5px;
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .reset:hover {
    color: var(--text);
  }
  .about-hero {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 16px;
  }
  .about-mark {
    width: 60px;
    height: 60px;
    border-radius: 17px;
    background: var(--accent-dim);
    border: 1px solid rgba(var(--accent-rgb), 0.22);
    color: var(--accent);
    display: grid;
    place-items: center;
  }
  .about-hero h3 {
    margin: 0;
    font-size: 20px;
    letter-spacing: -0.035em;
  }
  .about-hero span:not(.about-mark) {
    font-size: 11px;
    color: var(--muted);
  }
  .about-copy {
    font-size: 11.5px;
    line-height: 1.6;
    color: var(--text-dim);
    margin: 0 0 18px;
    max-width: 480px;
  }
  .specs {
    border: 1px solid var(--line);
    border-radius: 11px;
    overflow: hidden;
  }
  .specs div {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
    background: var(--panel-2);
  }
  .specs div + div {
    border-top: 1px solid var(--line);
  }
  .specs span {
    width: 68px;
    flex: none;
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--muted);
  }
  .specs b {
    font-size: 10.5px;
    font-weight: 500;
    color: var(--text-dim);
  }
  .link-rows {
    border: 1px solid var(--line);
    border-radius: 11px;
    overflow: hidden;
  }
  .link-rows button {
    width: 100%;
    border: 0;
    background: var(--panel-2);
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    color: var(--muted);
    text-align: left;
  }
  .link-rows button + button {
    border-top: 1px solid var(--line);
  }
  .link-rows button:hover {
    background: var(--panel-3);
    color: var(--text);
  }
  .link-rows button span {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .link-rows b {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text);
  }
  .link-rows small {
    font-size: 9.5px;
    color: var(--muted);
  }
  .link-rows button :global(svg) {
    transform: rotate(180deg);
  }
  .disclaimer {
    display: flex;
    gap: 10px;
    margin-top: 18px;
    padding: 11px 12px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--accent-dim);
    color: #d68a8a;
  }
  .disclaimer p {
    margin: 0;
    font-size: 10px;
    line-height: 1.55;
    color: var(--text-dim);
  }
</style>
