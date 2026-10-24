# Agent rules for Flackle Desktop

## 1. Never test the app

The user is the only person who tests Flackle Desktop. Do not launch the app, run a
playtest, take your own UI screenshots, or claim something works because you ran it.

Compiling, `svelte-check`, `cargo check`, and release builds are allowed. Running the
actual app is not.

## 2. Use Chrome DevTools for anything the selfbot does not answer

Check `reference/flackle-selfbot` first. If the endpoint, payload, permission,
notification rule, context-menu behavior, icon, or UI flow is not clearly covered there,
open Flackle in Chrome DevTools and inspect the live app, network requests, DOM, and API
responses.

Do not guess. Do not invent an API shape.

The user may already be logged into Flackle in Chrome DevTools. Read-only inspection is
allowed. Never post, delete, invite, leave, block, report, or change account/server data
on the live account unless the user explicitly asks.

## 3. Match Flackle, not generic Discord assumptions

Use Flackle's actual semantics for:

- server/channel/member/role permissions
- default notification level (a server-owner/admin setting for new members)
- unread and ping behavior
- channel icons, categories, and drag-and-drop
- message rendering, mentions, replies, and message links
- profile cards, banners, roles, and moderation actions

Use the official assets, fonts, colors, and icon sets from flackle.com where possible.

## 4. UI rules

- No gradients.
- No native `alert`, `confirm`, or `prompt` dialogs. Use in-app dialogs/menus.
- Keep the dark-first "graphite + ember" theme and the existing design language.
- Icons must be centered and must not be clipped by rounded containers.
- Pings are numeric, visible, and may extend outside an icon frame.
- Do not add tiny decorative text or fake controls that have no behavior.
- Channel names and messages may contain emoji; render them with the bundled Twemoji set.
- Show the full reply content, not a one-line truncation.
- Never show presence/status dots on message avatars.
- Always land on the newest message when opening a channel.
- Keep the composer correctly sized after edit/reply cancellation.

## 5. Sessions and security

- Persist sessions to disk first, then the OS keyring, and persist rotated refresh tokens.
- Never silently discard a session unless Flackle explicitly reports it expired.
- Never store user passwords or MFA secrets.
- Do not log credentials, tokens, or report details.

## 6. Build and delivery

- After changes, run `svelte-check`, `cargo check`, and produce the Windows installer with
  `npm run tauri build` unless the user says otherwise.
- Do not run the built app.
- Report the installer paths and a concise summary of what changed.
