# Platform support

Flackle Desktop is written around portable Rust and web technologies, while Tauri connects the
application to each operating system's native webview and packaging system.

## Support matrix

| Platform | Target(s) | Build status | Required native pieces |
| --- | --- | --- | --- |
| Windows | `x86_64-pc-windows-msvc` | Current release build | WebView2 Runtime, MSVC C++ toolchain |
| Windows ARM64 | `aarch64-pc-windows-msvc` | Supported target | ARM64 MSVC toolchain, WebView2 Runtime |
| macOS Intel | `x86_64-apple-darwin` | Supported target | Xcode Command Line Tools |
| macOS Apple Silicon | `aarch64-apple-darwin` | Supported target | Xcode Command Line Tools |
| macOS universal | `universal-apple-darwin` | Release target | Xcode, signing, notarization for distribution |
| Linux x64 | `x86_64-unknown-linux-gnu` | Supported target | WebKitGTK 4.1 and GTK/appindicator development packages |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | Supported target | ARM64 sysroot/linker plus the destination's WebKitGTK libraries |

ARM support refers to 64-bit ARM/AArch64. Other architectures may be possible with additional
Tauri and dependency work, but are not release commitments.

## Windows

- WebView2 Runtime is required. Current Windows installations normally have it already; the
  Evergreen installer can be used on clean systems.
- Building uses the MSVC Rust toolchain and Visual Studio Build Tools with the Desktop
  development with C++ workload.
- The project can produce NSIS and MSI bundles. Code signing is strongly recommended before
  publishing installers.

## macOS

- Xcode Command Line Tools are required for local builds; Xcode is recommended for universal
  and signed release builds.
- Apple Silicon uses the native `aarch64-apple-darwin` target.
- A universal package combines Intel and Apple Silicon builds.
- Gatekeeper-friendly distribution requires Developer ID signing and notarization.

## Linux

On Debian/Ubuntu-like distributions, the Tauri prerequisites are typically installed with:

```bash
sudo apt update
sudo apt install \
  build-essential curl wget file libssl-dev libxdo-dev \
  libayatana-appindicator3-dev librsvg2-dev \
  libwebkit2gtk-4.1-dev
```

Package names vary by distribution. The target system needs compatible WebKitGTK and GTK
runtime libraries even when the application is built successfully.

## Why the same code supports ARM

The frontend is architecture-neutral JavaScript and the protocol layer is Rust. Tauri's native
shell and Cargo's target-specific build pipeline supply the platform-specific pieces. ARM64
builds still need a matching native webview package, linker, and release runner; ARM support is
not achieved by copying an x64 installer to an ARM machine.
