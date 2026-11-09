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
- Validate and repair correctness issues (automatic, undoable fixes)
- Compare different versions or regions via diff tools
- Export normalized or converted files

## v1.0 Definition of Done

Status, evidence and remaining gaps per item are tracked in
[docs/v1-gap-analysis.md](./v1-gap-analysis.md). An item is DONE only when it is
implemented, covered by automated tests, and has no open gap.

- [DONE] **Web/PWA app launches locally in browser**: manifest, installable icons, build-time versioned offline precache, verified offline by E2E.
- [PARTIAL] **Supports `.litematic`, `.schem`, and Java Structure NBT**: all three read and write block entity / entity payloads. Sponge v1–v3 is supported. Negative-size Litematica regions are not yet verified against a real file (#37).
- [PARTIAL] **Enables open, view, select, fill, replace, delete, copy, paste, move, rotate, mirror, undo, redo, inspect, analyze, validate, diff, and export**: every operation is implemented, with drag & drop, viewport picking, multi-region display, entity editing, repair and cross-format diff. Analysis is slow on very large structures (#36).
- [DONE] **Keeps structure data local-first**: everything runs in the browser and WASM; nothing is uploaded and there is no telemetry.
- [DONE] **Includes round-trip and property tests**: format and cross-format round trips include block entity / entity payloads and multi-region flattening; proptests cover rotation/mirror identities.
- [DONE] **Includes E2E coverage for a practical editing workflow**: three Playwright specs (editing lifecycle, IDE features on a multi-region document, offline PWA).
- [PARTIAL] **Includes large-structure performance tests**: dense 1M/5M/10M fixtures, native release benchmarks and a browser harness; results and bottlenecks are in [docs/benchmarks.md](./benchmarks.md) (#36, #38).
- [DONE] **CI stays green**: Rust fmt/clippy/tests, WASM built from source, frontend checks, Playwright dev + preview projects.
- [DONE] **Architecture and README remain aligned with implementation**: reconciled in #34.
- [DONE] **No material silent data-loss issues are introduced**: payloads follow edits and survive every format; multi-region export flattens instead of truncating; lossy conversions require confirmation.

## Non-goals for Phase 0

The initial baseline intentionally focuses on repository structure, architecture documentation, validation strategy, and a clean workspace foundation. Functional feature implementation begins in subsequent phases.
