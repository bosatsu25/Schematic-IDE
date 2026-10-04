# Schematic IDE

A local-first, browser-based IDE for Minecraft structures.

## Status

- **Phases 0–7 Complete**:
  - Core domain, patch-based edit engine, and VoxelWeave selection/analysis/cleanup integration.
  - Lossless `.litematic` format adapter (pure Rust NBT + continuous 64-bit bit-packing).
  - Standalone WASM session engine (`schematic-wasm`) and Web Worker bridge.
  - Three.js voxel renderer with instanced mesh rendering and deterministic color mapping.
  - Interactive browser editor shell with viewport, region selection, bounded replacement, and VoxelWeave cleanup.
  - Playwright browser E2E test verifying the complete `.litematic` editing lifecycle in CI.
- **Phase 8+ In Progress (v1.0 Completion)**:
  - **Wave 8A**: Inspector & Diagnostics (`schematic-validate`, Diagnostic domain, Problems panel).
  - **Wave 8B**: Structural Editing (Fill, Copy/Paste, Move, Rotate, Mirror with directional blockstate transforms).
  - **Wave 8C**: Canonical Structural Diff & 3D diff overlay.
  - **Wave 8D**: Multi-format adapters (Sponge `.schem`, Java Structure `.nbt`).
  - **Wave 8E**: Material list & structural analysis UI.
  - **Wave 8F/8G**: Performance benchmarks (1M–10M block scale) & PWA offline application shell.

## Repository structure

- `apps/web` — browser application shell (React, Vite, Three.js, Web Worker)
- `crates/*` — Rust core, format, validation, edit, analysis, and WASM crates
- `packages/*` — renderer, Minecraft data, UI, and shared libraries
- `docs/` — requirements, architecture, ADRs, roadmap, gap analysis, and migrations
- `tests/` — fixtures and verification suites

## Design principles

- Single source of truth in the Rust core
- Chunk-oriented, patch-based editing model
- Lossless parse/serialize round-trips
- Format adapters instead of hard-coded file assumptions
- WASM and worker boundaries for heavy operations
- Zero silent data loss

See [docs/architecture.md](./docs/architecture.md), [docs/roadmap.md](./docs/roadmap.md), and the ADRs under [docs/adr/](./docs/adr/) for detailed design specifications.
