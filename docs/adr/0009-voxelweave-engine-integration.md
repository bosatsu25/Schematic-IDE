# ADR 0009: Integration of VoxelWeave Analysis & Editing Engine

## Status

Accepted

## Context

The VoxelWeave project developed algorithms for conservative surface analysis, feature recognition, protrusion tracing, and island cleanup in Java with Fabric integration. As development unifies into Schematic IDE, we must transition the mature algorithms and test suites into Schematic IDE's Rust workspace without dragging in Minecraft/Fabric dependencies or compromising browser safety.

## Decision

1. **Crate Organization**:
   - Pure selection and bounds logic are unified into `schematic-core`.
   - Editing operations, bounded replacement, and patch-based preview/undo/redo are provided in `schematic-edit`.
   - Topology and shape analysis algorithms (`SurfaceAnalyzer`, `SurfaceFeatureAnalyzer`, `ConnectedProtrusionAnalyzer`) are placed in a dedicated `schematic-analysis` crate, maintaining clean separation of analytical concerns from document storage and UI rendering.
   - Island cleanup planning is hosted in `schematic-edit::cleanup`, taking analysis results as input and producing reversible `PatchSet` instances.

2. **Chunk-Sparse Iteration**:
   - Rather than scanning full dense bounding boxes during block replacement and deletion, commands iterate over the occupied chunks intersecting the selection bounds. This resolves performance bottlenecks on sparse selections without altering results.

3. **Data Provenance & Stale Analysis Prevention**:
   - Analysis results are tagged with an immutable document snapshot token / revision number (`DocumentRevision`).
   - Cleanup planning and application require that the document revision matches the snapshot from which the analysis was produced. Any edit, undo, or redo increments or changes this token, rejecting stale analysis results.

4. **Coverage & Unknown Boundaries**:
   - Unregistered blocks in sparse chunks or coordinates outside the analyzed volume are treated as *unknown* (neither air nor solid), setting `unknown_faces` on adjoining cells. Components touching unknown data are flagged as incomplete and protected from cleanup.

## Consequences

- Full feature parity with VoxelWeave's tested algorithms within a high-performance, web-compatible Rust workspace.
- Guarantees that neither large sparse regions nor stale asynchronous worker results can cause hangs or data corruption.
- Clear path for browser UI and Web Worker exposure.
