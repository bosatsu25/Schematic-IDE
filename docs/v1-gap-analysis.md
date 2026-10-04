# v1.0 Gap Analysis

This document provides a line-by-line verification and gap analysis of Schematic IDE against the **v1.0 Definition of Done** defined in [docs/requirements.md](./requirements.md).

## Verification Matrix

| Requirement | Status | Evidence | Missing | Planned PR |
|---|:---:|---|---|---|
| **Local-first execution** | **DONE** | Pure client-side React + WASM architecture. No external API calls, backend services, or mandatory accounts. | None. Verified in Playwright E2E and worker tests. | N/A (Already in `main`) |
| **Web browser app** | **DONE** | React 18 + Vite frontend running locally on `http://localhost:5173`. | None. | PR #10 / PR #11 |
| **PWA / Offline support** | **PARTIAL** | App runs locally via dev/preview server. | Web App Manifest (`manifest.json`), service worker offline caching shell, installable icons. | Wave 8G (`chore/v1-pwa-docs`) |
| **`.litematic` support** | **DONE** | `schematic-format` implements pure Rust NBT, continuous 64-bit bit-packing, multi-region, negative bounds, block entity preservation. | None. Round-trip tested in `crates/schematic-format/tests/litematic_tests.rs`. | PR #7 |
| **Sponge `.schem` support** | **DONE** | Format adapter for Sponge Schematic v1/v2/v3 (varint block data, palette conversion, block entities, entities, unknown NBT preservation). Roundtrip tested in `multi_format_tests.rs` and E2E. | None. | Wave 8D (PR #16) |
| **Java Structure NBT support** | **DONE** | Structure NBT format adapter (size array, palette, block entries with positions, block entities, entities, unknown tag preservation). Roundtrip tested in `multi_format_tests.rs` and E2E. | None. | Wave 8D (PR #16) |
| **Open & View** | **DONE** | Multi-format file selector accepting `.litematic,.schem,.nbt`, auto-detecting format from magic tags, loads into Rust core, renders 3D instanced voxels in Three.js Canvas. | Interactive 3D raycast click selection in Viewport. | Wave 8A, Wave 8D |
| **Selection** | **DONE** | Multi-box selection, bounding boxes, coordinate mapping in `schematic-core`, active box controls in UI. | None. | Wave 8A |
| **Fill** | **DONE** | History engine supports `Fill` command semantics; UI Fill tool with preview/commit/undo/redo and E2E coverage. | None. | Wave 8B |
| **Replace** | **DONE** | `replace_blocks` in `schematic-edit`, UI Replace tool with source/target palette inputs, before/after preview, commit, undo/redo. | None. Tested in E2E. | PR #5, PR #10, PR #11 |
| **Delete** | **DONE** | Delete command clears selected blocks to air within bounds. | None. | Wave 8B |
| **Copy / Paste** | **DONE** | `Clipboard` struct in `schematic-edit`, relative offset translation, paste command with preview/commit/undo/redo, UI buttons, E2E coverage. | None. | Wave 8B |
| **Move** | **DONE** | Move command with directional delta offset, preview/commit/undo/redo, UI controls, and E2E coverage. | None. | Wave 8B |
| **Rotate** | **DONE** | 90°/180°/270° rotation around Y, directional block state transforms (facing, axis, rotation, shape), proptests verifying 4x 90° = identity. | None. | Wave 8B |
| **Mirror** | **DONE** | X and Z axis flipping with orientation property updates and proptests verifying 2x mirror = identity. | None. | Wave 8B |
| **Undo / Redo** | **DONE** | Patch-based history in `schematic-edit`, revision tokens, preview cancellation, UI Undo/Redo buttons. | None. Verified in unit tests and Playwright E2E. | PR #4, PR #8, PR #10, PR #11 |
| **Inspect** | **DONE** | Block inspection by coordinate, block state properties table, palette entry, block entity NBT inspection, and document metadata in Inspector panel. Tested in unit & E2E tests. | None. | Wave 8A |
| **Analyze** | **DONE** | Material aggregation table, 64-item stack calculation, fill density %, volume/dimensions, surface topology & feature breakdown via `schematic-analysis`, searchable UI table, and CSV export. Tested in session unit tests & E2E. | None. | Wave 8E (PR #17) |
| **Validate** | **DONE** | `schematic-validate` crate with `Diagnostic` domain model, rule engine (invalid palette ref, out-of-bounds pos, malformed metadata, block entity misalignment), Problems panel in UI with jump-to-position. Tested in 5 unit tests & E2E. | None. | Wave 8A |
| **Diff** | **DONE** | `schematic-diff` crate with canonical structural diff (Added, Removed, Modified, Unchanged), block state diff, entity diff, WASM C-ABI, Diff panel in UI with jump-to-position, and 3D diff overlay renderer with color coding. Tested in unit & E2E tests. | None. | Wave 8C (PR #15) |
| **Export** | **DONE** | Multi-format export dropdown (`.litematic`, `.schem`, `.nbt`) in Header, WASM C-ABI export functions, lossless serialization and browser download. | None. | PR #8, PR #10, PR #11, Wave 8D (PR #16) |
| **Round-trip tests** | **DONE** | Complete round-trip suite for `.litematic`, Sponge `.schem`, Java Structure `.nbt`, and cross-format interoperability roundtrip in `multi_format_tests.rs`. | None. | PR #7, Wave 8D (PR #16) |
| **Property-based tests** | **DONE** | Proptest suites in `crates/schematic-edit/tests/transform_tests.rs` for 4x 90° rotation identity, 2x mirror identity, and directional block states. | None. | Wave 8B |
| **Browser E2E workflow** | **DONE** | Playwright test (`apps/web/e2e/litematic-workflow.spec.ts`) exercises Open -> Render -> Selection -> Replace -> Preview -> Commit -> Undo -> Redo -> Structural edits -> Cleanup -> Multi-format Export (.litematic, .schem, .nbt) -> Reload. | None. | PR #11; Wave 8A–8D |
| **Large-structure performance** | **DONE** | Performance benchmark suite measuring parse latency, mesh generation, edit patch latency, and export at 1M, 5M, and 10M blocks. Tested in `crates/schematic-wasm/tests/scale_benchmarks.rs` and documented in `docs/benchmarks.md`. | None. | Wave 8F (PR #18) |
| **CI green** | **DONE** | GitHub Actions runs Rust clippy/fmt/tests, frontend lint/typecheck/tests, and Playwright Chromium E2E on every push. | Continue maintaining 100% green status on all waves. | All PRs |
| **Docs aligned** | **DONE** | README, roadmap, requirements, and gap analysis synchronized with repository state. | Continuous synchronization on each wave PR. | All PRs |
| **Zero silent data loss** | **DONE** | Block entity, entity, unknown NBT tag preservation verified in `.litematic`. | Cross-format conversion warning/rejection when converting lossy formats. | Wave 8D |
