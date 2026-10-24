<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import * as api from "../api";
  import type { LoginOutcome, SelfUser } from "../types";
  import Icon from "./Icon.svelte";
  import Mark from "./Mark.svelte";

  export let onSuccess: (user: SelfUser) => void;

  const dispatch = createEventDispatcher<{ done: SelfUser }>();

  let email = "";
  let password = "";
  let code = "";
  let mfaToken = "";
  let step: "credentials" | "mfa" = "credentials";
  let busy = false;
  let error = "";
  let showPassword = false;

  async function submitCredentials(event: SubmitEvent) {
    event.preventDefault();
    if (busy) return;
    error = "";
    if (!email.trim() || !password) {
      error = "Email and password are required.";
      return;
    }
    busy = true;
    try {
      const outcome: LoginOutcome = await api.login(email.trim(), password);
      if (outcome.status === "mfa_required") {
        mfaToken = outcome.mfa_token;
        step = "mfa";
        return;
      }
      finish(outcome.user);
    } catch (reason) {
      error = reason instanceof Error ? reason.message : String(reason);
    } finally {
      busy = false;
    }
  }

  async function submitMfa(event: SubmitEvent) {
    event.preventDefault();
    if (busy || !code.trim()) return;
    error = "";
    busy = true;
    try {
      const user = await api.loginMfa(mfaToken, code.trim());
      finish(user);
    } catch (reason) {
      error = reason instanceof Error ? reason.message : String(reason);
    } finally {
      busy = false;
    }
  }

  function finish(user: SelfUser) {
    onSuccess(user);
    dispatch("done", user);
  }

  function useDemo() {
    email = "gosho@example.com";
    password = "demo-account";
  }
</script>

<div class="login">
  <div class="backdrop">
    <img class="art" src="/brand/flackle-backdrop.svg" alt="" aria-hidden="true" />
  </div>

  <section class="pitch">
    <div class="wordmark">
      <span class="mark"><Mark size={30} /></span>
      <span>Flackle</span>
    </div>
    <h1>Community,<br /><span>without the weight.</span></h1>
    <p class="lede">
      A native desktop client built in Rust. Fast to open, quiet in the
      background, and yours to keep on the taskbar.
    </p>
  </section>

  <section class="pane">
    <div class="card">
      <header>
        <div class="mark small"><Mark size={22} /></div>
        <div>
          <h2>{step === "credentials" ? "Welcome back" : "Two-factor check"}</h2>
          <p>
            {step === "credentials"
              ? "Sign in to your Flackle account."
              : "Enter the code from your authenticator app."}
          </p>
        </div>
      </header>

      {#if step === "credentials"}
        <form on:submit={submitCredentials}>
          <label>
            <span>Email</span>
            <input
              type="email"
              bind:value={email}
              placeholder="you@example.com"
              autocomplete="username"
              spellcheck="false"
              autofocus
            />
          </label>
          <label>
            <span>Password</span>
            <div class="secret">
              <input
                type={showPassword ? "text" : "password"}
                bind:value={password}
                placeholder="••••••••••••"
                autocomplete="current-password"
              />
              <button
                type="button"
                class="reveal"
                on:click={() => (showPassword = !showPassword)}
                tabindex="-1"
              >
                {showPassword ? "hide" : "show"}
              </button>
            </div>
          </label>
          {#if error}<p class="error"><Icon name="alert" size={14} />{error}</p>{/if}
          <button class="primary" type="submit" disabled={busy}>
            {#if busy}<span class="spinner"></span> Signing in…{:else}Sign in{/if}
          </button>
        </form>
        {#if !api.inTauri}
          <button class="demo" type="button" on:click={useDemo}>Use a demo account</button>
        {/if}
      {:else}
        <form on:submit={submitMfa}>
          <label>
            <span>Authentication code</span>
            <input
              type="text"
              bind:value={code}
              placeholder="000 000"
              inputmode="numeric"
              autocomplete="one-time-code"
              maxlength="12"
              autofocus
            />
          </label>
          {#if error}<p class="error"><Icon name="alert" size={14} />{error}</p>{/if}
          <button class="primary" type="submit" disabled={busy}>
            {#if busy}<span class="spinner"></span> Checking…{:else}Continue{/if}
          </button>
          <button
            class="link"
            type="button"
            on:click={() => {
              step = "credentials";
              code = "";
              error = "";
            }}
          >
            Use a different account
          </button>
        </form>
      {/if}
    </div>
  </section>
</div>

<style>
  .login {
    height: 100%;
    display: grid;
    grid-template-columns: minmax(0, 1.15fr) minmax(380px, 0.85fr);
    background: var(--bg);
    position: relative;
    overflow: hidden;
  }
  .backdrop {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .art {
    position: absolute;
    left: -18%;
    top: 50%;
    width: min(1300px, 105vw);
    transform: translateY(-50%);
    opacity: 0.9;
    pointer-events: none;
    user-select: none;
  }
  .pitch {
    position: relative;
    padding: 64px clamp(40px, 7vw, 96px);
    display: flex;
    flex-direction: column;
    justify-content: center;
    max-width: 720px;
  }
  .wordmark {
    display: flex;
    align-items: center;
    gap: 12px;
    font-size: 21px;
    font-weight: 700;
    letter-spacing: -0.035em;
    margin-bottom: 54px;
  }
  .mark {
    color: var(--brand);
    display: flex;
    filter: drop-shadow(0 6px 24px rgba(var(--accent-rgb), 0.24));
  }
  h1 {
    font-size: clamp(42px, 5.2vw, 68px);
    line-height: 0.98;
    letter-spacing: -0.055em;
    margin: 0 0 24px;
    font-weight: 650;
  }
  h1 span {
    color: var(--muted);
  }
  .lede {
    font-size: 15.5px;
    line-height: 1.65;
    color: var(--text-dim);
    max-width: 460px;
    margin: 0;
  }
  .pane {
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: stretch;
    padding: 48px clamp(32px, 5vw, 72px) 48px 0;
    z-index: 2;
  }
  .card {
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: 22px;
    padding: 30px;
    box-shadow: 0 30px 90px rgba(0, 0, 0, 0.45), inset 0 1px 0 rgba(255, 255, 255, 0.035);
    max-width: 430px;
    width: 100%;
    margin-left: auto;
  }
  header {
    display: flex;
    gap: 14px;
    align-items: flex-start;
    margin-bottom: 26px;
  }
  .mark.small {
    width: 42px;
    height: 42px;
    border-radius: 12px;
    background: var(--accent-dim);
    border: 1px solid rgba(var(--accent-rgb), 0.22);
    display: grid;
    place-items: center;
    flex: none;
  }
  h2 {
    font-size: 20px;
    letter-spacing: -0.03em;
    margin: 2px 0 4px;
  }
  header p {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  label > span {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: #8b91a1;
  }
  input {
    width: 100%;
    height: 44px;
    background: #0a0c11;
    border: 1px solid var(--line-strong);
    border-radius: 11px;
    color: var(--text);
    padding: 0 13px;
    font-size: 14px;
    outline: none;
    transition: border-color 120ms ease, box-shadow 120ms ease;
  }
  input:focus {
    border-color: rgba(var(--accent-rgb), 0.55);
    box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.1);
  }
  input::placeholder {
    color: #4d5362;
  }
  .secret {
    position: relative;
  }
  .reveal {
    position: absolute;
    right: 8px;
    top: 50%;
    transform: translateY(-50%);
    border: 0;
    background: transparent;
    color: #6f7688;
    font-size: 11px;
    padding: 6px;
    cursor: pointer;
  }
  .reveal:hover {
    color: var(--text);
  }
  .primary {
    height: 44px;
    margin-top: 6px;
    border: 0;
    border-radius: 11px;
    background: var(--accent);
    color: white;
    font-weight: 650;
    font-size: 13.5px;
    letter-spacing: 0.01em;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 9px;
    transition: transform 120ms ease, background 120ms ease;
  }
  .primary:hover:not(:disabled) {
    background: #f46a6a;
  }
  .primary:active:not(:disabled) {
    transform: translateY(1px);
  }
  .primary:disabled {
    opacity: 0.7;
    cursor: default;
  }
  .spinner {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid rgba(255, 255, 255, 0.35);
    border-top-color: white;
    animation: spin 700ms linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .demo {
    align-self: flex-start;
    margin-top: 8px;
    padding: 5px 0;
    background: transparent;
    border: 0;
    color: var(--muted);
    font-size: 11.5px;
    cursor: pointer;
  }
  .demo:hover {
    color: var(--text);
  }
  .link {
    background: transparent;
    border: 0;
    color: var(--muted);
    font-size: 11.5px;
    cursor: pointer;
    padding: 4px;
  }
  .link:hover {
    color: var(--text);
  }
  .error {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 0;
    color: #ff8a8a;
    font-size: 12px;
    background: rgba(var(--accent-rgb), 0.08);
    border: 1px solid rgba(var(--accent-rgb), 0.18);
    border-radius: 8px;
    padding: 8px 10px;
  }
  @media (max-width: 980px) {
    .login {
      grid-template-columns: 1fr;
      overflow-y: auto;
    }
    .pitch {
      display: none;
    }
    .pane {
      padding: 28px;
      min-height: 100%;
    }
    .card {
      margin: 0 auto;
    }
  }
</style>
