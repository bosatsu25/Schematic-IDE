# Roadmap

## Phase 0 — Product / Architecture Baseline

Establish repository structure, design documentation, and clean-room architecture guardrails.

## Phase 1 — Workspace & Quality Gates

Set up monorepo tooling, CI, linting, build, and TypeScript + Rust validation.

## Phase 2 — schematic-core

Implement domain types, coordinate semantics, selection logic, palette identity, and chunk organization.

## Phase 3 — Edit Engine

Create patch-based command history and undo/redo operations for structural edits.

## Phase 4 — Litematic Format

Add format adapter support for `.litematic`, metadata preservation, and round-trip fixtures.

## Phase 5 — WASM Boundary

Expose core operations through WASM and web workers for safe large-file processing.

## Phase 6 — Renderer Foundation

Build a browser 3D viewport with chunked mesh updates and selection overlays.

## Phase 7 — First Vertical Slice

Deliver the MVP workflow from open → render → edit → undo → save.

## Phase 8+ — IDE Inspection, Validation, Structural Editing, Diff, Multi-format, Performance

Expand the product into a complete local-first structure IDE with a production-ready PWA deployment pipeline.
