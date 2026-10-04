# v1.0 Gap Analysis

This document provides a line-by-line verification and gap analysis of Schematic IDE against the **v1.0 Definition of Done** defined in [docs/requirements.md](./requirements.md).

## Verification Matrix

| Requirement | Status | Evidence | Missing | Planned PR |
|---|:---:|---|---|---|
| **Local-first execution** | **DONE** | Pure client-side React + WASM architecture. No external API calls, backend services, or mandatory accounts. | None. Verified in Playwright E2E and worker tests. | N/A (Already in `main`) |
| **Web browser app** | **DONE** | React 18 + Vite frontend running locally on `http://localhost:5173`. | None. | PR #10 / PR #11 |
| **PWA / Offline support** | **PARTIAL** | App runs locally via dev/preview server. | Web App Manifest (`manifest.json`), service worker offline caching shell, installable icons. | Wave 8G (`chore/v1-pwa-docs`) |
| **`.litematic` support** | **DONE** | `schematic-format` implements pure Rust NBT, continuous 64-bit bit-packing, multi-region, negative bounds, block entity preservation. | None. Round-trip tested in `crates/schematic-format/tests/litematic_tests.rs`. | PR #7 |
| **Sponge `.schem` support** | **NOT STARTED** | Format specification analyzed. | Format adapter for Sponge Schematic v2/v3 (varint block data, palettes, block entities). | Wave 8D (`feature/phase-8d-multi-format`) |
| **Java Structure NBT support** | **NOT STARTED** | NBT parser available in `schematic-format`. | Structure NBT format adapter (size array, palette, block entries with positions). | Wave 8D (`feature/phase-8d-multi-format`) |
| **Open & View** | **DONE** | File input loads `.litematic`, sends bytes to WASM worker, parses document, renders 3D instanced voxels in Three.js Canvas. | Multi-format file selector (handling `.schem` and `.nbt`). | Wave 8D |
| **Selection** | **DONE** | Multi-box selection, bounding boxes, coordinate mapping in `schematic-core`, active box controls in UI. | Interactive 3D raycast click selection in Viewport. | Wave 8A |
| **Fill** | **PARTIAL** | History engine supports `Fill` command semantics. | Dedicated Fill UI tool and batch fill execution over active Selection. | Wave 8B (`feature/phase-8b-structural-editing`) |
| **Replace** | **DONE** | `replace_blocks` in `schematic-edit`, UI Replace tool with source/target palette inputs, before/after preview, commit, undo/redo. | None. Tested in E2E. | PR #5, PR #10, PR #11 |
| **Delete** | **DONE** | Delete command clears selected blocks to air within bounds. | Dedicated Delete button in Tools panel. | Wave 8B |
| **Copy / Paste** | **NOT STARTED** | Selection boxes supported. | Clipboard struct in `schematic-edit`, relative offset translation, block entities handling, paste preview. | Wave 8B (`feature/phase-8b-structural-editing`) |
| **Move** | **NOT STARTED** | Selection boxes supported. | Cut selection + offset paste command with preview/commit/undo/redo. | Wave 8B (`feature/phase-8b-structural-editing`) |
| **Rotate** | **NOT STARTED** | Domain coordinates support transformations. | 90°/180°/270° rotation around Y, directional block state transforms (facing, axis, stairs/doors/slabs), invertibility tests. | Wave 8B (`feature/phase-8b-structural-editing`) |
| **Mirror** | **NOT STARTED** | Coordinates support inversion. | X and Z axis flipping with orientation property updates and invertibility tests. | Wave 8B (`feature/phase-8b-structural-editing`) |
| **Undo / Redo** | **DONE** | Patch-based history in `schematic-edit`, revision tokens, preview cancellation, UI Undo/Redo buttons. | None. Verified in unit tests and Playwright E2E. | PR #4, PR #8, PR #10, PR #11 |
| **Inspect** | **PARTIAL** | Region list, block counts, and palette rendered in UI Sidebar. | Block position raycast/click inspection, block state properties table, raw NBT / block entity inspection. | Wave 8A (`feature/phase-8a-inspection-validation`) |
| **Analyze** | **PARTIAL** | Surface topology, higher-order features, protrusion evidence, and conservative island cleanup in `schematic-analysis`. | Material list aggregate counts, structural dimensions/air counts, UI analysis tabs. | Wave 8E (`feature/phase-8e-analysis-ui`) |
| **Validate** | **PARTIAL** | `schematic-validate` crate exists (placeholder). | `Diagnostic` domain model, rule engine (invalid palette ref, out-of-bounds pos, malformed metadata, block entity misalignment), Problems panel. | Wave 8A (`feature/phase-8a-inspection-validation`) |
| **Diff** | **NOT STARTED** | Patch and chunk comparison primitives exist. | Canonical structural diff engine (Added, Removed, Changed, Unchanged), block state diff, 3D diff overlay renderer. | Wave 8C (`feature/phase-8c-diff`) |
| **Export** | **DONE** | Export button serializes canonical document to `.litematic` via WASM and triggers browser download. | Export format dropdown for `.schem` and `.nbt`. | PR #8, PR #10, PR #11; Wave 8D |
| **Round-trip tests** | **PARTIAL** | Complete round-trip suite for `.litematic` in `crates/schematic-format/tests/litematic_tests.rs`. | Round-trip suites for `.schem` and Java Structure `.nbt`, cross-format round-trip tests. | Wave 8D |
| **Property-based tests** | **PARTIAL** | Comprehensive boundary tests in `crates/schematic-core/tests/domain.rs`. | Proptest suites for rotate/mirror invertibility (4x rotate = ID, 2x mirror = ID) and directional block states. | Wave 8B |
| **Browser E2E workflow** | **DONE** | Playwright test (`apps/web/e2e/litematic-workflow.spec.ts`) exercises Open -> Render -> Selection -> Replace -> Preview -> Commit -> Undo -> Redo -> Cleanup -> Export -> Reload. | Extended E2E covering multi-format and new tools. | PR #11; Wave 8A–8E |
| **Large-structure performance** | **NOT STARTED** | Chunked sparse storage designed for scalability. | Formal benchmark suite measuring parse, mesh generation, edit patch latency, and export at 1M, 5M, and 10M blocks. | Wave 8F (`perf/v1-large-structure`) |
| **CI green** | **DONE** | GitHub Actions runs Rust clippy/fmt/tests, frontend lint/typecheck/tests, and Playwright Chromium E2E on every push. | Continue maintaining 100% green status on all waves. | All PRs |
| **Docs aligned** | **DONE** | README, roadmap, requirements, and gap analysis synchronized with repository state. | Continuous synchronization on each wave PR. | All PRs |
| **Zero silent data loss** | **DONE** | Block entity, entity, unknown NBT tag preservation verified in `.litematic`. | Cross-format conversion warning/rejection when converting lossy formats. | Wave 8D |
