# Building Flackle Desktop

## 1. Install prerequisites

Install the tools in the [platform support matrix](PLATFORM-SUPPORT.md), then verify the
toolchains:

```bash
node --version
npm --version
rustc --version
cargo --version
```

Node.js 20+ and Rust stable are recommended. The repository is an npm workspace for the Svelte
frontend and a Cargo workspace for the Rust crates.

## 2. Install JavaScript dependencies

Use the lockfile for reproducible installs:

```bash
npm ci
```

Use `npm install` only when intentionally changing dependencies and updating
`package-lock.json`.

## 3. Validate the workspace

```bash
npm run check       # Svelte and TypeScript diagnostics
npm run check:rust  # cargo check --workspace
npm run check:all   # both checks
```

These checks do not launch the desktop application.

## 4. Frontend development

The browser preview is useful for UI work and uses the local sample workspace when it is not
running inside Tauri:

```bash
npm run dev
```

Open the URL printed by Vite. Add `?login` to preview the login/MFA screens.

To run the actual native shell during interactive development:

```bash
npm run tauri dev
```

## 5. Production builds

Build the frontend:

```bash
npm run build
```

Build native release bundles for the current host:

```bash
npm run build:release
```

Equivalent Tauri command:

```bash
npm run tauri build
```

Typical output is written under `target/release/bundle/`. The exact installers depend on the
host operating system and configured Tauri targets.

## 6. Cross-compiling

Install the Rust target and the platform linker/toolchain before invoking Tauri. Examples:

```bash
# Windows ARM64, from a Windows host with the ARM64 MSVC toolchain
rustup target add aarch64-pc-windows-msvc
npm run tauri build -- --target aarch64-pc-windows-msvc

# Linux ARM64, from a Linux host with the matching cross-linker/sysroot
rustup target add aarch64-unknown-linux-gnu
npm run tauri build -- --target aarch64-unknown-linux-gnu

# macOS universal, from macOS with Xcode installed
npm run tauri build -- --target universal-apple-darwin
```

Cross-compilation is not only a Rust target selection: native webview, GTK, signing, and
packaging dependencies must also be available for the destination platform. Native CI runners
are recommended for public artifacts.

## 7. Generated assets

The normal build uses the generated files already checked into the workspace. Regeneration
requires the source asset directories described by each script:

```bash
npm run generate:emoji
npm run generate:channel-icons
```

`build-emoji.mjs` expects a Twemoji SVG directory and emoji metadata JSON. `build-channel-icons.mjs`
expects the Heroicons 24/solid package directory. See the comments at the top of each script
for `TWEMOJI_DIR`, `EMOJI_JSON`, and `HEROICONS_DIR` overrides.

## Troubleshooting

### Linux build cannot find WebKitGTK or GTK libraries

Install the development packages listed in
[`PLATFORM-SUPPORT.md`](PLATFORM-SUPPORT.md), then run the build again. Runtime libraries are
also required on the machine that launches the packaged application.

### Windows build cannot find the linker

Install Visual Studio Build Tools with the Desktop development with C++ workload and reopen
the terminal so the MSVC environment is available. WebView2 Runtime is also required at run
time.

### macOS build cannot be signed

Unsigned local builds are fine for development. Distribution builds need an Apple Developer ID,
entitlements, signing identities, and notarization credentials; see
[`RELEASING.md`](RELEASING.md).

### The browser preview shows sample data

That is expected when the frontend is outside Tauri. Use `npm run tauri dev` for native IPC,
session storage, notifications, and gateway behavior.
