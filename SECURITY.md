# Security policy

## Reporting a vulnerability

Do not open a public issue containing credentials, cookies, access tokens, refresh tokens, MFA
secrets, private messages, or account-identifying screenshots.

Until a private security contact is configured for this project, report security concerns to the
maintainer through a private channel and include:

- A short description and impact.
- The affected version or commit.
- Reproduction steps that use synthetic or redacted data.
- Any suggested mitigation.

## Client security expectations

- Passwords and MFA secrets must never be persisted or logged.
- Session tokens must remain in the OS credential vault or the restricted fallback file.
- Do not weaken the Content Security Policy to make an unrelated feature work.
- External URLs should open in the system browser.
- Redact account data before attaching logs or screenshots.
