# Contributing

## Workflow

1. Read the root `README.md` and the relevant document in `docs/`.
2. Keep UI behavior aligned with Flackle's observed API semantics; do not invent endpoint
   payloads when the protocol reference or live read-only inspection can answer the question.
3. Keep the Rust core independent of Tauri where possible.
4. Do not commit credentials, cookies, tokens, MFA secrets, private account data, `target/`,
   `dist/`, `node_modules/`, or local session files.
5. Run the checks before submitting a change:

   ```bash
   npm run check:all
   npm run build
   ```

6. Describe platform-specific changes and any untested target in the change summary.

## UI conventions

- Keep the graphite/ember visual language and dark-first design.
- Do not add gradients or native alert/confirm/prompt dialogs.
- Preserve keyboard access, visible focus states, and readable contrast.
- Use bundled Twemoji for user-facing emoji content.
- Keep moderation, notification, unread, and permission behavior consistent with Flackle.

## Testing boundaries

Static checks and release builds are safe to run locally. Live account actions must be
intentional and performed only with an account authorized for the test. Never use real
credentials in fixtures, logs, screenshots, or issue reports.
