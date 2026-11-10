# Schematic IDE

A local-first, browser-based IDE for Minecraft structures.

## Status

- **Phases 0–7 Complete**:
  - Core domain, patch-based edit engine, and VoxelWeave selection/analysis/cleanup integration.
  - Lossless `.litematic` format adapter (pure Rust NBT + continuous 64-bit bit-packing).
  - Standalone WASM session engine (`schematic-wasm`) and Web Worker bridge.
  - Three.js voxel renderer: face-culled per-chunk meshes, incremental chunk updates, orbit controls and raycast picking.
  - Interactive browser editor shell with viewport, region selection, bounded replacement, and VoxelWeave cleanup.
  - Playwright browser E2E test verifying the complete `.litematic` editing lifecycle in CI.
- **Waves 8A–8G**:
  - **Wave 8A**: Inspector & Diagnostics (`schematic-validate`, Diagnostic domain, Problems panel).
  - **Wave 8B**: Structural Editing (Fill, Copy/Paste, Move, Rotate, Mirror with directional blockstate transforms and proptests).
  - **Wave 8C**: Canonical Structural Diff & 3D diff overlay (Added, Removed, Modified).
  - **Wave 8D**: Multi-format adapters (Sponge `.schem`, Java Structure `.nbt`, cross-format diff).
  - **Wave 8E**: Material aggregation table, 64-item stack calculation, structural metrics, and CSV export.
  - **Wave 8F**: Performance benchmarks at 1M, 5M, and 10M block scale (re-done with dense fixtures in #26; see `docs/benchmarks.md`).
  - **Wave 8G**: PWA deployment, Web App Manifest, offline service worker shell (`sw.js`, generated at build time with a versioned precache), and installable icons.
- **v1 gap closure (issues #20–#34)**: block entity / entity NBT follows edits and survives every format, multi-region export, conversion-loss confirmation, viewport picking and multi-box selection, multi-region display, auto-repair, drag & drop, Entities panel, cross-format diff, WASM built in CI, realistic benchmarks. Open gaps: [#36](https://github.com/bosatsu25/Schematic-IDE/issues/36) (analysis performance; surface analysis is skipped above 500,000 blocks) [#37](https://github.com/bosatsu25/Schematic-IDE/issues/37) (Litematica negative-size verification) and [#38](https://github.com/bosatsu25/Schematic-IDE/issues/38) (rendering at 10M). See [docs/v1-gap-analysis.md](./docs/v1-gap-analysis.md).
- **First release preparation (v0.1.0)**: corrupt files produce errors instead of crashes, the engine restarts if it does crash, the build can be hosted under any path, and a tag-driven release workflow publishes the built site. See [CHANGELOG.md](./CHANGELOG.md) and [docs/release.md](./docs/release.md).
- **Phase 9+ (Future Roadmap)**:
  - Surface relax / smoothing, procedural gradients, dithering, and 3D model importer integrations (see [docs/roadmap.md](./docs/roadmap.md)).

## Using the app

1. Open a `.litematic`, `.schem` or `.nbt` file with **Open Schematic**, or drop
   it anywhere on the window. Files never leave your device.
2. Look around: drag to orbit, right-drag to pan, wheel to zoom.
3. Select: click a block, Shift+click to stretch a box from the first block,
   Ctrl/⌘+click to add another box. Coordinates can also be typed in the
   **Regions** tab.
4. Edit with the tools on the right (fill, replace, copy/paste, move, rotate,
   mirror, cleanup). Every edit shows a preview first; **Commit** applies it and
   **Undo** / **Redo** step through history.
5. Check the **Problems**, **Diff**, **Analysis** and **Entities** tabs as needed.
6. Pick a format and **Export**. If the target format cannot hold something,
   the app lists what would be lost before writing anything.

The version is shown in the status bar. Known limitations are listed in
[CHANGELOG.md](./CHANGELOG.md).

## Repository structure

- `apps/web` — browser application shell (React, Vite, Three.js, Web Worker)
- `crates/*` — Rust core, format, validation, edit, analysis, and WASM crates
- `packages/renderer` — Three.js voxel renderer
- `packages/minecraft-data` — Minecraft data tables (block colors, DataVersion → game version labels)
- `docs/` — requirements, architecture, ADRs, roadmap, gap analysis, and migrations

## Building and running

Prerequisites: Node 20+, pnpm 9, and Rust stable with the WASM target
(`rustup target add wasm32-unknown-unknown`).

```sh
pnpm install
pnpm build:wasm   # cargo build -p schematic-wasm --target wasm32-unknown-unknown --release
pnpm --filter @schematic-ide/web dev
```

The WASM module (`apps/web/public/schematic_wasm.wasm`) is a build output and is
not committed. `scripts/build-wasm.mjs` builds it from `crates/schematic-wasm` and
copies it into `apps/web/public/`. The web app's `dev`, `build`, `test`, and
`test:e2e` scripts run it automatically first; cargo's incremental build makes
repeat runs cheap. See [docs/development-loop.md](./docs/development-loop.md).

## Releasing and hosting

`pnpm build` produces a static site in `apps/web/dist` that uses only relative
URLs, so it can be served from a domain root or any sub-path. Pushing a `v*`
tag publishes a GitHub Release with the site attached (and optionally deploys
GitHub Pages). See [docs/release.md](./docs/release.md).

## License

[MIT](./LICENSE)

## Design principles

- Single source of truth in the Rust core
- Chunk-oriented, patch-based editing model
- Lossless parse/serialize round-trips
- Format adapters instead of hard-coded file assumptions
- WASM and worker boundaries for heavy operations
- Zero silent data loss

See [docs/architecture.md](./docs/architecture.md), [docs/roadmap.md](./docs/roadmap.md), and the ADRs under [docs/adr/](./docs/adr/) for detailed design specifications.
