# GeoWatcher

Watches [Geometry Dash](https://www.robtopgames.com/) streams on Twitch and alerts you when
someone gets far on a **real** attempt — not a Start Pos run.

Built with [Tauri 2](https://tauri.app): Rust handles stream capture and frame analysis,
React renders the channel table.

[![CI](https://github.com/aledlb8/geowatcher/actions/workflows/ci.yml/badge.svg)](https://github.com/aledlb8/geowatcher/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

## Stack

| Layer | Technology |
| --- | --- |
| Shell | Tauri 2.11 (Rust) |
| UI | React 19 + TypeScript 7 |
| Build | Vite 8 |
| Styling | Tailwind CSS 4 |
| Lint & format | Biome 2 (TS), rustfmt + Clippy (Rust) |

## Requirements

- **Node.js** 20.19 or newer
- **Rust** (stable) via [rustup](https://rustup.rs)
- Platform prerequisites listed in the [Tauri setup guide](https://tauri.app/start/prerequisites/):
  - **Windows** — WebView2 (preinstalled on Windows 11) and the MSVC C++ build tools
  - **macOS** — Xcode Command Line Tools
  - **Linux** — `libwebkit2gtk-4.1-dev`, `libxdo-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev` and `patchelf`

Run `npm run doctor` at any point to have Tauri report what it can and cannot find.

### ffmpeg

The app decodes streams with ffmpeg, bundled as a Tauri sidecar so users need
nothing installed. Fetch it once before building a bundle:

```bash
npm run ffmpeg                                       # for your platform
node scripts/fetch-ffmpeg.mjs --target <triple>      # for a specific one
```

It lands in `src-tauri/binaries/` (git-ignored, ~110 MB) and CI fetches it per
platform. During `npm run dev` no bundle exists, so the app falls back to
whatever `ffmpeg` is on your `PATH`.

> **Licensing.** Windows and Linux use [BtbN's](https://github.com/BtbN/FFmpeg-Builds)
> **LGPL** builds, which this permissively licensed app can ship. No LGPL macOS
> build is published upstream, so the macOS binaries are **GPL** — shipping one
> makes the whole bundle subject to the GPL. The script prints the licence every
> time it runs. If that matters to you, require macOS users to install ffmpeg
> themselves instead of bundling it.

## Getting started

```bash
git clone https://github.com/aledlb8/geowatcher.git
cd geowatcher
npm install
npm run dev
```

`npm run dev` starts Vite and opens the native window with hot-module reload. Editing
anything under `src/` updates the window instantly; editing anything under `src-tauri/`
recompiles the Rust binary and restarts the app.

## Scripts

| Script | Purpose |
| --- | --- |
| `npm run dev` | Run the desktop app with hot reload |
| `npm run dev:web` | Run only the frontend in a browser |
| `npm run build` | Build a release bundle for **your** platform |
| `npm run build:debug` | Same, unoptimised and with debug symbols |
| `npm run build:web` | Type-check and build the frontend only |
| `npm run ffmpeg` | Download the ffmpeg sidecar for this platform |
| `npm run check` | Biome lint + format check |
| `npm run check:fix` | Apply every safe Biome fix |
| `npm run typecheck` | TypeScript, no emit |
| `npm run rust:clippy` | Clippy with warnings denied |
| `npm run rust:fmt` | Format the Rust sources |
| `npm run verify` | Everything CI checks, in one command |
| `npm run doctor` | Report the local toolchain state |

Run `npm run verify` before opening a pull request — it is exactly what CI runs.

## Building for Windows, macOS and Linux

Tauri links against the system webview and uses each platform's native linker, so
**a machine can only produce bundles for the operating system it is running on.**
There is no cross-compilation path between Windows, macOS and Linux.

`npm run build` therefore bundles for the host, and refuses a `--target` belonging to
another OS rather than failing later with an opaque linker error:

```bash
npm run build                                  # host platform, release
npm run build -- --target aarch64-apple-darwin # host OS, different architecture
npm run build -- --bundles deb,appimage        # pick specific formats
```

Artifacts land in `src-tauri/target/<triple>/release/bundle/`.

All three platforms are built together in CI:

- **[`ci.yml`](.github/workflows/ci.yml)** builds a bundle on Windows, Linux and both
  macOS architectures for every push and pull request, and uploads them as run artifacts.
- **[`release.yml`](.github/workflows/release.yml)** builds installers for all three
  platforms and attaches them to a **draft** GitHub release when you push a version tag:

  ```bash
  npm version 0.2.0        # bumps package.json and tags
  git push --follow-tags
  ```

  Review the draft and publish it when you are happy. Bundle formats produced are
  `.msi`/`.exe` (Windows), `.dmg`/`.app` (macOS universal) and `.AppImage`/`.deb` (Linux).

> `src-tauri/tauri.conf.json` reads its version straight from `package.json`, so
> `npm version` is the single place a release version is set. The version in
> `src-tauri/Cargo.toml` only labels the Rust crate and does not affect the bundles.

## Code signing

The release workflow produces **unsigned** bundles. They install fine, but Windows
SmartScreen and macOS Gatekeeper will warn users until the binaries are signed.

Signing is configured entirely through repository secrets — no code changes are
needed. See the Tauri guides for
[Windows](https://tauri.app/distribute/sign/windows/) and
[macOS](https://tauri.app/distribute/sign/macos/), then add the secrets the
`tauri-action` step reads:

| Platform | Secrets |
| --- | --- |
| macOS | `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_TEAM_ID`, `APPLE_PASSWORD` |
| Windows | `WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD` |

The same applies to the built-in updater: it is not enabled here, and turning it on
requires a signing keypair (`npm run tauri signer generate`).

## Project layout

```
├── src/                    React frontend
│   ├── App.tsx             Root component
│   ├── main.tsx            React entry point
│   └── index.css           Tailwind entry point
├── src-tauri/              Rust backend
│   ├── src/lib.rs          Tauri commands and app setup
│   ├── src/main.rs         Desktop binary entry point
│   ├── capabilities/       Per-window permission grants
│   └── tauri.conf.json     Window, bundle and build configuration
├── scripts/build.mjs       Bundle build wrapper
├── scripts/fetch-ffmpeg.mjs  Downloads the ffmpeg sidecar
└── .github/workflows/      CI and release automation
```

## Adding a Tauri command

Rust functions become callable from TypeScript by way of `#[tauri::command]`:

```rust
// src-tauri/src/lib.rs
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}
```

Register it in the `invoke_handler` list, then call it from the frontend:

```ts
import { invoke } from "@tauri-apps/api/core";

const message = await invoke<string>("greet", { name: "world" });
```

Anything a command touches beyond the app's own memory — the filesystem, the network,
the shell — must also be granted in `src-tauri/capabilities/default.json`.

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) first.

## Security

Please do not report security issues in public issues. See [SECURITY.md](SECURITY.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in this project by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
