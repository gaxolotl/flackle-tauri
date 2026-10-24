# Releasing Flackle Desktop

## Before building

1. Update the version in `package.json`, `src-tauri/tauri.conf.json`, and the workspace package
   metadata as appropriate.
2. Review user-facing changes and update `README.md` and the relevant docs.
3. Run the static checks:

   ```bash
   npm ci
   npm run check:all
   ```

4. Confirm that no credentials, session files, local logs, or generated build directories are
   included in the release source.

## Build artifacts

Build on a native runner for each destination platform:

```bash
npm run build:release
```

Artifacts are emitted under `target/release/bundle/`. Common outputs include:

- Windows: NSIS `.exe` and MSI `.msi` installers.
- macOS: `.app` and DMG output, depending on Tauri configuration.
- Linux: AppImage, deb, and rpm output, depending on installed packaging tools.

ARM64 artifacts must be built for the ARM64 target and must be smoke-checked on matching
hardware or a supported emulator before publication.

## Signing

- Windows: sign installers and the application executable with the project certificate before
  distribution.
- macOS: sign with Developer ID, enable hardened runtime as appropriate, notarize, and staple
  the ticket to the final artifact.
- Linux: publish checksums and, if used by the distribution channel, sign repository metadata.

The updater is not connected to a release channel yet. Do not advertise automatic updates until
the update endpoint, signature verification, rollback behavior, and release permissions are in
place.

## Release notes

Release notes should include:

- Version and supported platform/architecture artifacts.
- New features and behavior changes.
- Known limitations, especially API-dependent features such as voice audio.
- Checksums and signing status.
- Any migration or session-storage changes.
