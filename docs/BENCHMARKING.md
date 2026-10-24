# Resource benchmarking

Flackle Desktop is designed to use less memory, storage, and idle CPU than a heavier desktop
client, but a fair comparison requires matching conditions. Do not publish a fixed percentage
without recording the test method and results.

## Recommended comparison

Compare Flackle Desktop with the official client on the same machine, using the same account,
the same number of open servers/channels, and the same time window:

1. Start both clients from a cold boot.
2. Wait for the workspace to finish loading.
3. Record idle memory and CPU after five minutes.
4. Open the same busy channel and record memory and CPU again after five minutes.
5. Send and receive the same number of messages and attachments, then record peak memory.
6. Close both clients and record installed size and background processes.
7. Repeat at least three times and report median values.

## What to record

| Metric | Windows | macOS | Linux |
| --- | --- | --- | --- |
| Process memory | Task Manager / Working set | Activity Monitor / Memory | System Monitor or `ps` |
| CPU | Task Manager | Activity Monitor | `top`/`htop` |
| Installed size | Installed Apps or directory size | `.app`/Application Support size | package and cache size |
| Background processes | Task Manager | Activity Monitor | `ps`/systemd user processes |

Record the Flackle Desktop version, comparison-client version, OS version, architecture, display
scale, open channel, network condition, and whether hardware acceleration is enabled.

## Why the design should be lighter

- Rust owns networking and gateway work instead of keeping the protocol logic in a large JS
  runtime.
- Tauri reuses the operating system webview rather than shipping another full browser runtime.
- The release profile is optimized for size and strips symbols.
- The UI only loads the active workspace and bundles a focused emoji/icon data set.

These are architectural reasons to expect a lower footprint, not a substitute for a measured
benchmark.
