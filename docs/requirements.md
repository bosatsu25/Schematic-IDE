# Schematic IDE Requirements

## Product intent

Schematic IDE is a local-first browser-based IDE for Minecraft structure files. It is designed to help users open, inspect, edit, validate, diff, repair, and export `.litematic`, `.schem`, and Java Structure NBT files without requiring a mandatory backend or login.

## Clean-room principle

This repository intentionally avoids copying another implementation's source code, class structure, component organization, APIs, or naming. This project is designed from first principles around the product goals in this repository and the constraints of large Minecraft structure files.

## Core user workflows

- Open structure files from local disk or drag-and-drop imports
- View a structure in 3D in the browser
- Select regions / blocks / entities / block entities
- Fill, replace, delete, and transform sections
- Inspect metadata, block state, and NBT details
- Validate and repair correctness issues
- Compare different versions or regions via diff tools
- Export normalized or converted files

## v1.0 Definition of Done

See [docs/v1-gap-analysis.md](./v1-gap-analysis.md) for detailed item-by-item status, evidence, and remaining deliverables.

- [DONE] **Web/PWA app launches locally in browser**: Local Web app launches and runs in browser; PWA manifest, installable icons, and offline service worker caching shell verified (Wave 8G).
- [DONE] **Supports `.litematic`, `.schem`, and Java Structure NBT**: All three formats supported with lossless parse/export, cross-format diff, and auto-detection (PR #7, PR #16).
- [DONE] **Enables open, view, select, fill, replace, delete, copy, paste, move, rotate, mirror, undo, redo, inspect, analyze, validate, diff, and export**:
  - Open, view, select, replace, delete, undo, redo, cleanup analysis, export, inspect, validate: **DONE**
  - Fill, copy, paste, move, rotate, mirror: **DONE** (Wave 8B)
  - Diff: **DONE** (Wave 8C)
  - Multi-format: **DONE** (Wave 8D)
  - Material & structural analysis: **DONE** (Wave 8E)
- [DONE] **Keeps structure data local-first**: Runs entirely in the user's browser client and WASM runtime; zero server upload or telemetry.
- [DONE] **Includes round-trip and property tests**: Complete round-trip suites for `.litematic`, `.schem`, `.nbt`, and proptest suites for 4x rotation and 2x mirror inverse properties.
- [DONE] **Includes E2E coverage for a practical editing workflow**: Playwright E2E browser tests pass on Chromium in CI (PR #11, PR #13–#17).
- [DONE] **Includes large-structure performance tests**: Performance benchmark suite at 1M, 5M, and 10M blocks implemented in `crates/schematic-wasm/tests/scale_benchmarks.rs` and documented in `docs/benchmarks.md` (Wave 8F).
- [DONE] **CI stays green**: GitHub Actions runs format, clippy, unit tests, frontend checks, and browser E2E tests cleanly on all commits.
- [DONE] **Architecture and README remain aligned with implementation**: Updated and maintained continuously.
- [DONE] **No material silent data-loss issues are introduced**: NBT preservation, negative bounds preservation, unknown tag passthrough verified.

## Non-goals for Phase 0

The initial baseline intentionally focuses on repository structure, architecture documentation, validation strategy, and a clean workspace foundation. Functional feature implementation begins in subsequent phases.
