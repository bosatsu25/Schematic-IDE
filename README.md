# Schematic IDE

A local-first, browser-based IDE for Minecraft structures.

## Status

Phase 0 baseline: repository architecture, clean-room design guardrails, and workspace layout are in place.

## Repository structure

- `apps/web` — browser application shell
- `crates/*` — Rust core, format, validation, edit, and WASM crates
- `packages/*` — renderer, Minecraft data, UI, and shared libraries
- `docs/` — requirements, architecture, ADRs, roadmap, and development loop
- `tests/` — fixtures and verification suites

## Design principles

- Single source of truth in the Rust core
- Chunk-oriented, patch-based editing model
- Lossless parse/serialize round-trips
- Format adapters instead of hard-coded file assumptions
- WASM and worker boundaries for heavy operations

See [docs/architecture.md](./docs/architecture.md) and the ADRs under [docs/adr/](./docs/adr/) for the current architecture decisions.
