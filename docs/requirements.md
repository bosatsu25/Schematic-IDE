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

- [PARTIAL] **Web/PWA app launches locally in browser**: Local Web app launches and runs in browser; PWA manifest and offline service worker caching to be completed in Wave 8G.
- [PARTIAL] **Supports `.litematic`, `.schem`, and Java Structure NBT**: `.litematic` is fully supported with lossless parse/export; `.schem` and Java Structure `.nbt` to be completed in Wave 8D.
- [PARTIAL] **Enables open, view, select, fill, replace, delete, copy, paste, move, rotate, mirror, undo, redo, inspect, analyze, validate, diff, and export**:
  - Open, view, select, replace, delete, undo, redo, cleanup analysis, export: **DONE**
  - Fill, copy, paste, move, rotate, mirror: **Wave 8B**
  - Inspect, validate: **Wave 8A**
  - Diff: **Wave 8C**
  - Material & structural analysis: **Wave 8E**
- [DONE] **Keeps structure data local-first**: Runs entirely in the user's browser client and WASM runtime; zero server upload or telemetry.
- [PARTIAL] **Includes round-trip and property tests**: `.litematic` round-trip tests and domain tests exist; property-based rotation/mirror inverse tests to be completed in Wave 8B, multi-format round-trip in Wave 8D.
- [DONE] **Includes E2E coverage for a practical editing workflow**: Playwright E2E browser tests pass on Chromium in CI (PR #11).
- [NOT STARTED] **Includes large-structure performance tests**: Performance benchmark suite at 1M, 5M, and 10M blocks planned for Wave 8F.
- [DONE] **CI stays green**: GitHub Actions runs format, clippy, unit tests, frontend checks, and browser E2E tests cleanly on all commits.
- [DONE] **Architecture and README remain aligned with implementation**: Updated and maintained continuously.
- [DONE] **No material silent data-loss issues are introduced**: NBT preservation, negative bounds preservation, unknown tag passthrough verified.

## Non-goals for Phase 0

The initial baseline intentionally focuses on repository structure, architecture documentation, validation strategy, and a clean workspace foundation. Functional feature implementation begins in subsequent phases.
