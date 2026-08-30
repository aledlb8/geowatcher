#!/usr/bin/env node
/**
 * Downloads the ffmpeg binary GeoWatcher bundles as a Tauri sidecar.
 *
 * Tauri names sidecars `<name>-<target-triple>`, and picks the one matching the
 * build target, so CI runs this once per platform in the build matrix.
 *
 * Usage:
 *   node scripts/fetch-ffmpeg.mjs [--target <triple>] [--force]
 *
 * LICENSING: Windows and Linux use LGPL builds, which a permissively licensed
 * application can ship. No LGPL macOS build is published, so the macOS binaries
 * here are GPL — distributing them makes the whole bundle subject to the GPL.
 * The script says which licence it fetched every time.
 */
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, renameSync, rmSync, statSync } from "node:fs";
import { writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = join(rootDir, "src-tauri", "binaries");

/** BtbN publishes LGPL builds; the macOS sources are GPL. */
const BTBN = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest";

const SOURCES = {
  "x86_64-pc-windows-msvc": {
    url: `${BTBN}/ffmpeg-n9.0-latest-win64-lgpl-9.0.zip`,
    licence: "LGPL",
  },
  "aarch64-pc-windows-msvc": {
    url: `${BTBN}/ffmpeg-n9.0-latest-winarm64-lgpl-9.0.zip`,
    licence: "LGPL",
  },
  "x86_64-unknown-linux-gnu": {
    url: `${BTBN}/ffmpeg-n9.0-latest-linux64-lgpl-9.0.tar.xz`,
    licence: "LGPL",
  },
  "aarch64-unknown-linux-gnu": {
    url: `${BTBN}/ffmpeg-n9.0-latest-linuxarm64-lgpl-9.0.tar.xz`,
    licence: "LGPL",
  },
  "x86_64-apple-darwin": {
    url: "https://evermeet.cx/ffmpeg/getrelease/zip",
    licence: "GPL",
  },
  "aarch64-apple-darwin": {
    url: "https://www.osxexperts.net/ffmpeg711arm.zip",
    licence: "GPL",
  },
};

function fail(message, hint) {
  console.error(`\n  error  ${message}`);
  if (hint) console.error(`         ${hint}`);
  console.error("");
  process.exit(1);
}

/** Best guess at the host's Rust target triple. */
function hostTriple() {
  const arch = process.arch === "arm64" ? "aarch64" : "x86_64";
  if (process.platform === "win32") return `${arch}-pc-windows-msvc`;
  if (process.platform === "darwin") return `${arch}-apple-darwin`;
  return `${arch}-unknown-linux-gnu`;
}

function parseArgs(argv) {
  const options = { target: hostTriple(), force: false };
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === "--target") options.target = argv[++i];
    else if (argv[i].startsWith("--target=")) options.target = argv[i].slice(9);
    else if (argv[i] === "--force") options.force = true;
  }
  if (!options.target) fail("--target requires a value.");
  return options;
}

/** Recursively finds the extracted ffmpeg executable. */
function findBinary(dir, wanted) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      const found = findBinary(path, wanted);
      if (found) return found;
    } else if (entry.name === wanted) {
      return path;
    }
  }
  return null;
}

/** Unpacks the archive.
 *
 * macOS and Linux use tar: bsdtar reads zip, GNU tar reads .tar.xz. On Windows
 * the tar on PATH may be MSYS GNU tar, which cannot read zip at all, so
 * PowerShell does the job instead.
 */
function extract(archive, into) {
  const result =
    process.platform === "win32"
      ? spawnSync(
          "powershell",
          [
            "-NoProfile",
            "-Command",
            `Expand-Archive -LiteralPath '${archive}' -DestinationPath '${into}' -Force`,
          ],
          { stdio: "inherit" },
        )
      : // Named relatively so no tar reads an absolute path as a remote host.
        spawnSync("tar", ["-xf", basename(archive)], { cwd: into, stdio: "inherit" });

  if (result.status !== 0) {
    fail(`could not extract ${archive}`);
  }
}

/** Tauri wants one universal sidecar for a universal macOS build. */
async function buildUniversal(force) {
  const target = "universal-apple-darwin";
  const destination = join(outDir, `ffmpeg-${target}`);
  if (existsSync(destination) && !force) {
    console.log(`  ffmpeg already present: ${destination}
`);
    return;
  }

  const parts = ["x86_64-apple-darwin", "aarch64-apple-darwin"];
  for (const part of parts) {
    await fetchOne(part, force);
  }

  const inputs = parts.map((part) => join(outDir, `ffmpeg-${part}`));
  const result = spawnSync("lipo", ["-create", ...inputs, "-output", destination], {
    stdio: "inherit",
  });
  if (result.status !== 0) {
    fail("lipo could not combine the macOS binaries");
  }
  spawnSync("chmod", ["+x", destination]);
  console.log(`  wrote ${destination}
`);
}

async function main() {
  const { target, force } = parseArgs(process.argv.slice(2));
  if (target === "universal-apple-darwin") {
    await buildUniversal(force);
    return;
  }
  await fetchOne(target, force);
}

async function fetchOne(target, force) {
  const source = SOURCES[target];
  if (!source) {
    fail(`No ffmpeg source for ${target}.`, `Known: ${Object.keys(SOURCES).join(", ")}`);
  }

  const windows = target.includes("windows");
  const exeName = windows ? "ffmpeg.exe" : "ffmpeg";
  const destination = join(outDir, `ffmpeg-${target}${windows ? ".exe" : ""}`);

  if (existsSync(destination) && !force) {
    console.log(`  ffmpeg already present: ${destination}`);
    console.log("  pass --force to replace it\n");
    return;
  }

  console.log(`\n  target   ${target}`);
  console.log(`  licence  ${source.licence}`);
  if (source.licence === "GPL") {
    console.log("  note     Shipping a GPL ffmpeg makes the whole bundle GPL.");
    console.log("           No LGPL macOS build is published upstream.");
  }
  console.log(`  source   ${source.url}\n`);

  const work = join(outDir, `.tmp-${target}`);
  rmSync(work, { recursive: true, force: true });
  mkdirSync(work, { recursive: true });

  const response = await fetch(source.url, { redirect: "follow" });
  if (!response.ok) {
    fail(`download failed with HTTP ${response.status}`, source.url);
  }

  const archive = join(work, source.url.endsWith(".tar.xz") ? "ffmpeg.tar.xz" : "ffmpeg.zip");
  await writeFile(archive, Buffer.from(await response.arrayBuffer()));
  extract(archive, work);

  const found = findBinary(work, exeName);
  if (!found) {
    fail(`no ${exeName} inside the archive`, work);
  }

  mkdirSync(outDir, { recursive: true });
  rmSync(destination, { force: true });
  renameSync(found, destination);
  if (!windows) {
    spawnSync("chmod", ["+x", destination]);
  }
  rmSync(work, { recursive: true, force: true });

  const megabytes = (statSync(destination).size / 1024 / 1024).toFixed(1);
  console.log(`  wrote ${destination} (${megabytes} MB)\n`);
}

main().catch((error) => fail(error.message));
