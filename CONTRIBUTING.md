# Contributing to GeoWatcher

Thanks for taking the time to contribute.

## Getting set up

You need Node.js 20.19+, a stable Rust toolchain from [rustup](https://rustup.rs), and
your platform's Tauri prerequisites — the [README](README.md#requirements) lists them.
`npm run doctor` reports what is missing.

```bash
npm install
npm run dev
```

## Before you open a pull request

Run the full check suite. It is byte-for-byte what CI runs, so a green result locally
means a green result on GitHub:

```bash
npm run verify
```

Most style problems fix themselves:

```bash
npm run check:fix   # Biome: lint + format the TypeScript
npm run rust:fmt    # rustfmt the Rust
```

## Conventions

- **Formatting is not a matter of taste.** Biome owns the TypeScript, rustfmt owns the
  Rust. Do not hand-format around them, and do not add a competing formatter config.
- **Clippy warnings are errors in CI.** If a lint is genuinely wrong for a piece of code,
  `#[allow(...)]` it with a comment explaining why, rather than loosening the whole crate.
- **Commit messages** follow [Conventional Commits](https://www.conventionalcommits.org):
  `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`. The prefix is what makes the
  changelog readable later.
- **Keep pull requests focused.** One concern per PR; unrelated cleanups belong in their
  own commit or their own PR.

## Where things live

Frontend code is in `src/`, the Rust side in `src-tauri/src/`. A feature that crosses the
boundary usually means a `#[tauri::command]` in `src-tauri/src/lib.rs`, registered in the
`invoke_handler!` list and called via `invoke()` from TypeScript.

If your command reaches outside the app — filesystem, network, shell, notifications — it
also needs a matching permission in `src-tauri/capabilities/default.json`. Grant the
narrowest permission that works; these are the app's actual security boundary, not
boilerplate.

## Reporting bugs

Open an issue with the app version, your OS and version, and the steps to reproduce.
The output of `npm run doctor` is very helpful for build problems.

For anything security-sensitive, follow [SECURITY.md](SECURITY.md) instead of opening
a public issue.

## Licensing

By contributing, you agree that your contributions are dual licensed under MIT and
Apache-2.0, matching the license of the project.
