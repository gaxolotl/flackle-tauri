# Flackle Desktop

<p align="center">
  <strong>A fast, native desktop client for Flackle.</strong><br>
  Rust core · Tauri 2 · Svelte 5 · Windows · macOS · Linux · ARM64
</p>

Flackle Desktop is an unofficial, resource-conscious desktop client for
[Flackle](https://flackle.com). It combines a Rust protocol and gateway layer with a
handwritten Svelte interface and Tauri's native webview shell. The shipped application does
not bundle Node, Bun, or a second Chromium runtime.

> **Project status:** The Windows x64 release bundle is the currently built artifact. macOS,
> Linux, Windows ARM64, and macOS Apple Silicon are supported build targets; signing,
> notarization, and platform-specific packaging are still required for public distribution.

## Highlights

- Native-feeling desktop shell with a custom title bar, system tray, close-to-tray behavior,
  and native notifications.
- Servers, categories, text channels, voice-channel metadata, stats channels, direct messages,
  group conversations, members, roles, invites, bans, and audit logs.
- Message history, pagination, editing, deletion, replies, reactions, attachments, previews,
  role mentions, typing indicators, unread state, pings, and live gateway updates.
- Friends and direct messaging with incoming/outgoing requests, blocking, and user context
  menus.
- Profile cards, roles, permission-aware controls, moderation actions, and server/channel
  management.
- Stats channels with live member, online, and voice counts, activity and active-user charts,
  a yearly activity calendar, an hourly message heatmap, and Flackle's general/elevated stats
  permissions.
- TOTP two-factor authentication, backup codes, active-session management, security events,
  and session revocation.
- Bundled Twemoji rendering, searchable emoji picker, customizable keybinds, density, text
  size, reduced motion, notification rules, custom typing phrases and emoji styles, and
  dark/light/system themes.
- Optional enhancements with real behavior and a settings toggle: link previews, inline video
  with a custom Flackle-style player and picture-in-picture, live composer preview, character
  counter, mention avatars, quick reactions, who-reacted tooltips, attachment expiry labels,
  and keep-current-channel.

## Why it is lighter

Compared with the official desktop client, Flackle Desktop is designed to use substantially
fewer resources in normal desktop use:

1. **Rust handles the network core.** REST, gateway reconnects, retries, uploads, and session
   persistence run outside the JavaScript UI.
2. **The app uses the operating system webview.** Tauri avoids shipping a separate Node/Bun
   runtime or a second embedded Chromium browser.
3. **The release profile is size-conscious.** Workspace release builds use optimization for
   size, link-time optimization, one codegen unit, symbol stripping, and an abort-on-panic
   profile.
4. **Assets are selective.** Emoji and icon data are generated into the application instead
   of loading a large remote UI bundle at runtime.

The project does not currently publish a controlled, apples-to-apples memory benchmark, so it
does not claim a fixed percentage improvement. See
[`docs/BENCHMARKING.md`](docs/BENCHMARKING.md) for a repeatable comparison method.

## Platform support

| Platform | Architectures | Status | Distribution notes |
| --- | --- | --- | --- |
| Windows | x64, ARM64 | Supported build targets | NSIS and MSI bundles; WebView2 is required |
| macOS | Intel x64, Apple Silicon ARM64 | Supported build targets | Universal builds need Xcode; public releases need Developer ID signing and notarization |
| Linux | x64, ARM64/AArch64 | Supported build targets | AppImage/deb/rpm output depends on the selected Tauri target and installed system libraries |

ARM support means 64-bit ARM/AArch64 release targets. The project does not currently promise
32-bit ARM packages. Detailed prerequisites and target commands are in
[`docs/PLATFORM-SUPPORT.md`](docs/PLATFORM-SUPPORT.md).

## Development quick start

### Requirements

- Node.js 20 or newer and npm.
- Rust stable and Cargo.
- Windows: Microsoft C++ Build Tools and WebView2 Runtime.
- macOS: Xcode Command Line Tools.
- Linux: WebKitGTK 4.1 development libraries, a C compiler/linker, OpenSSL, and the Tauri
  GTK/appindicator dependencies.

Install dependencies and run the static checks:

```bash
npm ci
npm run check:all
```

Build the frontend only:

```bash
npm run build
```

Run the browser UI with sample data:

```bash
npm run dev
```

Run the native development shell:

```bash
npm run tauri dev
```

Create release installers for the current machine:

```bash
npm run build:release
```

The complete setup, troubleshooting, and cross-compilation instructions are in
[`docs/BUILDING.md`](docs/BUILDING.md).

## Current limitations

- Voice audio is not available because Flackle does not expose an SFU endpoint to this client.
- Account creation, billing, and bulk moderation are outside the current scope.
- Public macOS distribution still needs Apple Developer signing and notarization.
- Windows code signing is recommended before distributing installers.
- The updater is not connected to a release channel yet.

## Project layout

```text
crates/flackle-core/   Rust REST, auth, uploads, and WebSocket gateway client
src-tauri/             Tauri commands, tray, notifications, and session storage
src/                   Svelte 5 + TypeScript interface
public/                Fonts, emoji artwork, and other bundled UI assets
scripts/               Generated asset tooling
reference/             Protocol and web-client investigation material
docs/                  Build, platform, architecture, release, and benchmark notes
```

## Documentation

- [Documentation index](docs/README.md)
- [Building and development](docs/BUILDING.md)
- [Platform and ARM support](docs/PLATFORM-SUPPORT.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Release checklist](docs/RELEASING.md)
- [Resource benchmarking](docs/BENCHMARKING.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)

## Security and privacy

- Sessions are persisted to the OS credential vault when available, with a restricted file
  fallback.
- The webview Content Security Policy allows only the app, Tauri IPC, and Flackle's HTTPS/WSS
  endpoints.
- External links open in the system browser.
- Passwords and MFA secrets are not stored by the application.
- Use this client only where your account and Flackle's rules permit it.

## Attribution

Protocol work is based on [`flackle-selfbot`](https://github.com/gaxolotl/flackle-selfbot)
(MIT). Brand assets come from flackle.com. Manrope and Satoshi are bundled under their
respective font licenses. Emoji artwork is Twemoji (CC-BY 4.0). See [`NOTICE`](NOTICE).

Flackle Desktop is not affiliated with Flackle or Folkward LLC.

## License

MIT. See [`LICENSE`](LICENSE).
