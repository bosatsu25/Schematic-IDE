# VoxelWeave → Schematic IDE Integration Plan

## 1. Overview & Context

This document governs the migration and consolidation of the standalone [VoxelWeave](https://github.com/bosatsu25/VoxelWeave) project into [Schematic IDE](https://github.com/bosatsu25/Schematic-IDE).

VoxelWeave developed clean, conservative algorithms for:
- 6-neighbor surface topology analysis (`SurfaceAnalyzer`)
- Higher-order feature classification (`SurfaceFeatureAnalyzer`)
- TIP-origin bounded protrusion evidence gathering (`ConnectedProtrusionAnalyzer`)
- Conservative disconnected-island cleanup with feature-preservation vetoes (`DisconnectedIslandCleanupPlanner`)
- Preview/commit/cancel and undo/redo editing workspace (`EditWorkspace`)
- Multi-box selection and placement target geometry mapping (`Selection`, `OperationTarget`, `WorldBoundsMapping`)

Schematic IDE is a browser-centric, local-first web application backed by a modular Rust core. Rather than porting Fabric/Minecraft mod dependencies or duplicating Java runtime logic, VoxelWeave's pure domain algorithms and mathematical safety invariants are integrated directly into Schematic IDE's Rust workspace (`schematic-core`, `schematic-edit`, and `schematic-analysis`).

## 2. Invariants & Guardrails

1. **Rust Core as Single Source of Truth**: UI and web workers only hold derived view state or preview overlays. Document state and structure mutations are strictly owned by Rust crates.
2. **Chunk-Sparse Storage & Patch-Based History**: Edits are stored as reversible chunk-scoped patches, never whole-document snapshots.
3. **Explicit Coverage & Unknown Boundary Distinction**: In VoxelWeave, unrecorded/unloaded chunks are *unknown*, never implicitly treated as air. This distinction is strictly preserved: missing chunk data sets `unknown_faces` on boundary voxels, preventing dangerous false "surface" or false "island" classifications.
4. **Conservative Preservation**:
   - Incomplete components touching unknown data or continuing outside the operation target are never marked as small islands.
   - Any component containing a protected feature (such as TIP, THIN_FEATURE, EDGE, CORNER) is completely vetoed from island cleanup.
   - Tracing protrusions collects geometric evidence without making autonomous removal judgments (spires, decorative spikes, and antennas are not auto-deleted).
5. **Snapshot Provenance & Stale Analysis Rejection**:
   - Analysis results are tagged with an immutable document snapshot token / revision.
   - Any document mutation (or undo/redo) invalidates previous analysis results, preventing stale cleanup applications from corrupting new geometry.
6. **No Third-Party Mod Leaks**:
   - Fabric, Litematica, and MaLiLib runtime APIs are not ported into Rust core crates. In-game capture is cataloged as a separate capture adapter, while Schematic IDE processes files via `schematic-format`.

## 3. Phased Waves

- **Wave 0**: Inventory, migration ledgers, ADR 0009, backup & restore verification.
- **Wave 1**: Core selection domain, coordinate mapping, bounded block replacement, and workspace preview/commit/undo/redo in `schematic-edit`.
- **Wave 2**: Conservative analysis engine (`schematic-analysis`) implementing `SurfaceAnalyzer`, `SurfaceFeatureAnalyzer`, and `ConnectedProtrusionAnalyzer`.
- **Wave 3**: Feature-preserving cleanup planner (`DisconnectedIslandCleanupPlanner`) and integration with `EditWorkspace`.
- **Wave 4**: Browser workflow readiness, format adapter alignment, and audit.
- **Wave 5**: Comprehensive quality gates (`cargo fmt`, `clippy`, `test`, `pnpm lint`, `typecheck`, `test`, `build`), parity checks, and retirement checklist evaluation.
