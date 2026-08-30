#!/usr/bin/env node
/**
 * Usage:
 *   node scripts/build.mjs [--debug] [--target <triple>] [--bundles <list>]
 *
 *   --debug            Build without optimisations and keep debug symbols.
 *   --target <triple>  Rust target triple, e.g. aarch64-apple-darwin. Must
 *                      belong to the host OS; cross-architecture is fine.
 *   --bundles <list>   Comma-separated bundle formats (msi, nsis, deb, rpm,
 *                      appimage, app, dmg). Defaults to the platform's usual set.
 */
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const PLATFORM_LABELS = { win32: "Windows", darwin: "macOS", linux: "Linux" };
const label = (platform) => PLATFORM_LABELS[platform] ?? platform;

/** `universal-apple-darwin` is a Tauri construct, not a real rustup target. */
const UNIVERSAL_MACOS = "universal-apple-darwin";
const UNIVERSAL_COMPONENTS = ["aarch64-apple-darwin", "x86_64-apple-darwin"];

/** Maps a Rust target triple to the `process.platform` that is able to build it. */
function platformOfTriple(triple) {
  if (triple.includes("windows")) return "win32";
  if (triple.includes("apple-darwin")) return "darwin";
  if (triple.includes("linux")) return "linux";
  return null;
}

function fail(message, hint) {
  console.error(`\n  error  ${message}`);
  if (hint) console.error(`         ${hint}`);
  console.error("");
  process.exit(1);
}

function parseArgs(argv) {
  const options = { debug: false, target: null, bundles: null, passthrough: [] };

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--debug") {
      options.debug = true;
    } else if (arg === "--target") {
      options.target = argv[++i];
    } else if (arg.startsWith("--target=")) {
      options.target = arg.slice("--target=".length);
    } else if (arg === "--bundles") {
      options.bundles = argv[++i];
    } else if (arg.startsWith("--bundles=")) {
      options.bundles = arg.slice("--bundles=".length);
    } else {
      options.passthrough.push(arg);
    }
  }

  if (options.target === undefined || options.bundles === undefined) {
    fail("--target and --bundles each require a value.");
  }
  return options;
}

/** Verifies the Rust toolchain is on PATH before handing off to the Tauri CLI. */
function assertCargoAvailable() {
  const probe = spawnSync("cargo", ["--version"], { stdio: "ignore" });
  if (probe.status !== 0) {
    fail(
      "`cargo` was not found on PATH.",
      "Install the Rust toolchain from https://rustup.rs and reopen your shell.",
    );
  }
}

/** Rejects a target belonging to another OS, which Tauri cannot cross-compile. */
function assertTargetIsBuildable(target) {
  if (!target) return;

  const required = platformOfTriple(target);
  if (required === null) {
    fail(`Unrecognised target triple: ${target}`);
  }
  if (required !== process.platform) {
    fail(
      `Cannot build ${label(required)} bundles from ${label(process.platform)}.`,
      "Tauri links the system webview and cannot cross-compile between operating " +
        "systems. Push a tag to run the `release` workflow, which builds all three.",
    );
  }

  const probe = spawnSync("rustup", ["target", "list", "--installed"], {
    encoding: "utf8",
  });
  // Without rustup there is nothing to verify against; let cargo report it instead.
  if (probe.status !== 0) return;

  const installed = new Set(probe.stdout.split(/\r?\n/).map((line) => line.trim()));
  const needed = target === UNIVERSAL_MACOS ? UNIVERSAL_COMPONENTS : [target];
  const missing = needed.filter((triple) => !installed.has(triple));

  if (missing.length > 0) {
    fail(
      `Rust target${missing.length > 1 ? "s" : ""} not installed: ${missing.join(", ")}`,
      `Run: rustup target add ${missing.join(" ")}`,
    );
  }
}

function main() {
  const options = parseArgs(process.argv.slice(2));

  assertCargoAvailable();
  assertTargetIsBuildable(options.target);

  const args = ["build"];
  if (options.debug) args.push("--debug");
  if (options.target) args.push("--target", options.target);
  if (options.bundles) args.push("--bundles", options.bundles);
  args.push(...options.passthrough);

  const profile = options.debug ? "debug" : "release";
  console.log(
    `\n  Building ${profile} bundle for ${label(process.platform)}` +
      `${options.target ? ` (${options.target})` : ""}\n`,
  );

  const cli = require.resolve("@tauri-apps/cli/tauri.js");
  const result = spawnSync(process.execPath, [cli, ...args], {
    cwd: rootDir,
    stdio: "inherit",
  });

  if (result.error) fail(result.error.message);
  if (result.status !== 0) process.exit(result.status ?? 1);

  const bundleDir = join(
    rootDir,
    "src-tauri",
    "target",
    ...(options.target ? [options.target] : []),
    profile,
    "bundle",
  );
  if (existsSync(bundleDir)) {
    console.log(`\n  Bundles written to ${bundleDir}\n`);
  }
}

main();
