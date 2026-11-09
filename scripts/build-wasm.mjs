#!/usr/bin/env node
// Builds the `schematic-wasm` crate for wasm32-unknown-unknown and copies the
// resulting module into apps/web/public/ so Vite (dev + build) and Vitest can
// load it. Cargo's incremental build makes repeat invocations cheap, so the web
// app calls this before every dev/build/test run to keep the binary fresh.
//
// Environment:
//   SKIP_WASM_BUILD=1  Skip the cargo build and use the existing public binary
//                      (fails if it does not exist).
import { spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, readFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const target = 'wasm32-unknown-unknown';
const artifact = join(root, 'target', target, 'release', 'schematic_wasm.wasm');
const destination = join(root, 'apps', 'web', 'public', 'schematic_wasm.wasm');
const rel = (p) => relative(root, p);

function fail(message) {
  console.error(`[build-wasm] ${message}`);
  process.exit(1);
}

if (process.env.SKIP_WASM_BUILD === '1') {
  if (!existsSync(destination)) {
    fail(`SKIP_WASM_BUILD=1 but ${rel(destination)} does not exist.`);
  }
  console.log(`[build-wasm] SKIP_WASM_BUILD=1, using existing ${rel(destination)}`);
  process.exit(0);
}

const cargo = spawnSync(
  'cargo',
  ['build', '-p', 'schematic-wasm', '--target', target, '--release'],
  { cwd: root, stdio: 'inherit' },
);

if (cargo.error) {
  fail(
    `could not run cargo (${cargo.error.message}). Install Rust (https://rustup.rs) ` +
      `and the target with \`rustup target add ${target}\`.`,
  );
}
if (cargo.status !== 0) {
  fail(
    `cargo build failed (exit ${cargo.status}). If the target is missing, run ` +
      `\`rustup target add ${target}\`.`,
  );
}
if (!existsSync(artifact)) {
  fail(`cargo succeeded but ${rel(artifact)} was not produced.`);
}

const built = readFileSync(artifact);
if (existsSync(destination) && readFileSync(destination).equals(built)) {
  console.log(`[build-wasm] ${rel(destination)} is up to date`);
} else {
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(artifact, destination);
  console.log(`[build-wasm] copied ${rel(artifact)} -> ${rel(destination)}`);
}
