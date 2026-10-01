# Schematic IDE Architecture

## Summary

Schematic IDE is organized around a clear separation between:

- a Rust core that owns the canonical structure model and file parsing logic
- a WASM boundary used by the browser app to access that model safely
- a web UI built with React and TypeScript that stores only view-state and interaction state
- an edit engine that applies small patch-based operations instead of full snapshots
- renderer and validation layers that operate on chunk- or region-oriented views to keep performance predictable

## Core principles

### 1. Single source of truth

The canonical structure document is owned by Rust core crates. The UI receives lightweight derived models or query results rather than duplicated full document state.

### 2. Chunk-oriented storage

Large structures are treated as collections of chunks. Edits are tracked per-chunk and patch-based to allow dirty-chunk rebuilds, partial undo/redo, and memory-efficient diffing.

### 3. Command + patch model

Commands transform a document and produce a reversible patch set. This avoids full-document snapshots and keeps history scoped to changed regions.

### 4. Lossless format handling

Unknown NBT fields are preserved across parse/serialize cycles. Format adapters keep format-specific details separate from domain logic.

### 5. Browser safety

Heavy parsing, validation, conversion, and mesh generation are routed through web workers and the wasm boundary so the main thread remains responsive.

## Repository layout

```
apps/
  web/
crates/
  schematic-core/
  schematic-format/
  schematic-edit/
  schematic-validate/
  schematic-wasm/
packages/
  renderer/
  minecraft-data/
  ui/
  shared/
tests/
  fixtures/
  roundtrip/
  compatibility/
  performance/
  e2e/
docs/
  adr/
```

## Execution model

1. File is opened with a format adapter.
2. Format adapter converts to Rust domain model.
3. UI queries metadata and derived view models.
4. User actions create commands and patches.
5. Rust core validates and updates chunks.
6. Renderer consumes dirty chunks or a render model.
7. Export serializes via the selected format adapter.

## Risks and mitigations

- Large structures can cause memory pressure. Mitigation: chunked data, sparse storage, and dirty region rebuilds.
- Format compatibility differences can cause data loss. Mitigation: adapter contracts and round-trip fixtures.
- UI thread overload. Mitigation: worker + WASM boundary and small message payloads.

## Phase roadmap

- Phase 0: baseline and repository structure
- Phase 1: workspace and quality gates
- Phase 2: core domain types and coordinate semantics
- Phase 3: edit engine and patch history
- Phase 4: litematic format adapter and fixtures
- Phase 5: web assembly boundary and worker integration
- Phase 6: renderer foundation
- Phase 7: first editor vertical slice
- Phase 8+: inspection, validation, structural editing, diff, multi-format, and performance hardening
