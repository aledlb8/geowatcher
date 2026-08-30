# Security Policy

## Supported versions

This project is pre-1.0. Security fixes are applied to the latest release only.

## Reporting a vulnerability

**Please do not open a public issue for a security vulnerability.**

Report it privately through GitHub's
[private vulnerability reporting](https://github.com/aledlb8/geowatcher/security/advisories/new),
which notifies the maintainers without disclosing the issue publicly.

Please include:

- a description of the issue and its impact,
- the steps or proof of concept needed to reproduce it,
- the affected version, and your OS and version.

You can expect an acknowledgement within a few days. Once the issue is confirmed, a fix
and a coordinated disclosure will be arranged with you. Credit is given in the advisory
unless you would rather stay anonymous.

## Scope

The Tauri security model matters here. Especially relevant are:

- **Capabilities** — anything granted in `src-tauri/capabilities/` widens what the
  frontend is allowed to reach. Over-broad grants are legitimate findings.
- **Tauri commands** — every `#[tauri::command]` is an entry point callable from
  frontend JavaScript and must validate its own input.
- **The CSP** in `src-tauri/tauri.conf.json`, which governs what the webview may load.

Vulnerabilities in Tauri itself should go to the
[Tauri project](https://github.com/tauri-apps/tauri/security/policy).
