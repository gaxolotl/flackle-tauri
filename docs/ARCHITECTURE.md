# Architecture

## Overview

```text
┌──────────────────────────────────────────────┐
│ Svelte 5 + TypeScript UI                     │
│ workspace state · components · preferences   │
└──────────────────────┬───────────────────────┘
                       │ Tauri invoke/events
┌──────────────────────▼───────────────────────┐
│ Tauri 2 shell                                │
│ commands · tray · notifications · keyring    │
└──────────────────────┬───────────────────────┘
                       │ Rust calls
┌──────────────────────▼───────────────────────┐
│ flackle-core                                 │
│ auth · REST · uploads · gateway · retries    │
└──────────────────────┬───────────────────────┘
                       │ HTTPS / WSS
                 Flackle services
```

## Frontend

`src/` contains the Svelte 5 interface and its client-side state:

- `workspace.ts` coordinates bootstrap data, navigation, unread state, notifications, and
  gateway events.
- `api.ts` provides the Tauri command boundary and browser/demo fallbacks.
- `components/` contains the shell, chat, settings, profiles, friends, dialogs, and server
  management UI.
- `app.css` owns the handwritten graphite/ember theme, light theme, typography, spacing, and
  accessibility states.

The browser preview deliberately uses sample data when Tauri is unavailable so UI work does
not require a live account.

## Native shell

`src-tauri/` owns platform integration:

- Tauri commands expose authenticated operations to the frontend.
- The tray and window lifecycle support notifications and close-to-tray behavior.
- Session tokens are persisted to the OS credential vault first, with a restricted file fallback.
- Gateway events are forwarded into the Svelte workspace store.
- The Content Security Policy limits network access to the app, Tauri IPC, and Flackle's
  endpoints.

## Protocol core

`crates/flackle-core/` intentionally has no Tauri dependency. It implements:

- Cookie-based authentication, CSRF handling, MFA, refresh-token rotation, and retries.
- REST resources for servers, channels, messages, conversations, friends, roles, moderation,
  uploads, and security settings.
- NDJSON/WebSocket gateway connection, heartbeat, reconnection, and event decoding.
- Rate limiting and message acknowledgement behavior.

Keeping this layer independent makes it easier to test, reuse in another shell, and build for
x64 and ARM64 without changing frontend code.

## Security boundaries

The UI never receives a raw password after login. Tauri owns session persistence and the Rust
client owns authenticated requests. External links are sent to the system browser rather than
loaded into the application webview.

Do not add logging around credentials, cookies, access tokens, refresh tokens, MFA secrets, or
uploaded private content.
