# Roadmap

## Reconciled Status

- **Phase 0 — Product / Architecture Baseline**: Complete (PR #1).
- **Phase 1 — Workspace & Quality Gates**: Complete (PR #1, PR #11).
- **Phase 2 — Core Domain (`schematic-core`)**: Complete (PR #2, PR #5).
- **Phase 3 — Edit Engine (`schematic-edit`)**: Complete (PR #4, PR #5, PR #6).
- **Phase 4 — Litematic Format Adapter (`schematic-format`)**: Complete (PR #7).
- **Phase 5 — WASM Session Boundary (`schematic-wasm`)**: Complete (PR #8).
- **Phase 6 — Three.js Renderer (`@schematic-ide/renderer`)**: Complete (PR #10).
- **Phase 7 — First Vertical Slice & E2E (`apps/web` & Playwright)**: Complete (PR #10, PR #11).
- **Phase 8+ — IDE Hardening & v1.0 Completion**: In progress.

---

## Completed Phases

### Phase 0 — Product / Architecture Baseline
**Complete.** Repository structure, architecture specifications, clean-room principles, and ADRs established.

### Phase 1 — Workspace & Quality Gates
**Complete.** Cargo + pnpm workspaces, strict TypeScript, ESLint, Vitest, Rust clippy/fmt/test in GitHub Actions, and Playwright E2E browser test harness.

### Phase 2 — Core Domain (`schematic-core`)
**Complete.** Format-independent domain model, chunk-oriented block storage, coordinates, Selection/SelectionBox multi-box unions, and palette identity.

### Phase 3 — Edit Engine (`schematic-edit`)
**Complete.** Patch-based command history (PatchSet), undo/redo, bounded replacement, EditWorkspace, revision tokens for stale prevention, and VoxelWeave conservative island cleanup integration.

### Phase 4 — Litematic Format (`schematic-format`)
**Complete.** Pure Rust NBT parser and serializer, continuous 64-bit bit-packing/unpacking, negative-size bounds normalization, and lossless preservation of unknown tags, block entities, and entities.

### Phase 5 — WASM Boundary (`schematic-wasm`)
**Complete.** Session C-ABI, allocation/deallocation boundary, thread-local response buffers, Web Worker wrapper, and typed TypeScript worker client.

### Phase 6 — Renderer Foundation (`@schematic-ide/renderer`)
**Complete.** Three.js instanced mesh voxel renderer, chunked updates, bounding box overlays, and deterministic block state palette coloring.

### Phase 7 — First Vertical Slice (`apps/web` & E2E)
**Complete.** Interactive editor UI with viewport, selection controls, replace tool, VoxelWeave cleanup tool, before/after preview, and Playwright E2E browser tests passing on Chromium in CI.

---

## Phase 8+ — IDE Hardening & v1.0 Completion

### Wave 8A — Inspector & Diagnostics
- **`schematic-validate` crate implementation**:
  - Diagnostic domain model: `Diagnostic { severity, code, message, region, position, fixability }`.
  - Core validation rules: out-of-bounds blocks, palette index corruption, malformed metadata, block entity misalignment, duplicate regions.
- **IDE Inspection UI**:
  - Active region details, selected block position, block ID, block state properties, palette entry, block entity NBT details, document metadata.
- **Problems Panel**:
  - Filterable list of validation diagnostics with position links and severity badges.

### Wave 8B — Structural Editing
- **Advanced Editing Operations**:
  - **Fill**: Fill selected region/boxes with specified block state.
  - **Copy / Paste**: Region-relative clipboard capturing blocks, block states, and block entities with safe translation.
  - **Move**: Cut selection and translate with full undo/redo.
  - **Rotate**: 90/180/270 degree rotation around Y axis with directional blockstate property transformation (facing, axis, shape, half, rotation).
  - **Mirror**: X and Z axis flipping with orientation property updates.
- **Correctness & Tests**:
  - Invertibility tests (4x 90° rotate = identity, 2x mirror = identity).
  - Property-based tests for directional Minecraft block states.

### Wave 8C — Canonical Structural Diff
- **Structural Diff Engine**:
  - Comparison between two Documents or two Regions in canonical domain.
  - Categories: Added, Removed, Changed, Unchanged.
  - Compares block state IDs, properties, and block entities.
- **3D Diff Visualizer**:
  - Color-coded overlay in Three.js renderer (Green = Added, Red = Removed, Amber = Changed).
  - Diff summary and change list in the web UI.

### Wave 8D — Multi-format Adapters
- **Sponge `.schem` Adapter**:
  - Schematic v2/v3 support, varint block data, metadata, palettes, block entities, entities.
- **Java Structure `.nbt` Adapter**:
  - Vanilla structure NBT format, size array, palette, block entries with pos and state index.
- **Lossless Inter-format Round-trip**:
  - Round-trip tests and explicit warnings / conversion rejection for unsupported cross-format features (no silent data loss).

### Wave 8E — Material & Structural Analysis UI
- **Material List**:
  - Aggregated block state counts across document, region, and selection.
  - Searchable and exportable material breakdown.
- **Structural Statistics**:
  - Total dimensions, volume, occupied block count, air count, palette size, entity count, block entity count.
- **Integrated Surface & Feature Analysis**:
  - Expose `schematic-analysis` metrics to the UI.

### Wave 8F — Performance Hardening
- **Benchmark Suite**:
  - Synthetic benchmarks at 1M, 5M, and 10M block scale.
  - Measure parse latency, memory footprint, mesh build time, edit patch latency, and export time.
- **Chunked Pipeline Tuning**:
  - Keep Web Worker message transfers zero-copy where possible and avoid blocking the UI thread.

### Wave 8G — PWA & Offline Application Shell
- **PWA Deployment**:
  - Web App Manifest (`manifest.json`), service worker for offline asset caching.
  - Fully local-first guarantee: zero outbound telemetry or backend requirements.

---

## Phase 9+ — Advanced Features (Future Roadmap)

Features conceived during VoxelWeave research are tracked for Phase 9+ and will not block v1.0:

- Surface Relax / Mesh Smoothing
- Procedural Gradients & Color Blending
- Procedural Patterns & Dithering
- Contour Correction & Edge Aligners
- ObjToSchematic 3D Model Importer Integration
