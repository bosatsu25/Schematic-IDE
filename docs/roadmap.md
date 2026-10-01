# Roadmap

## Reconciled status

- Phase 0 is complete and merged to `main`.
- Phase 1 quality gates were already present in the Phase 0 baseline; no second
  quality-gates implementation is needed.
- Playwright is not configured. Browser E2E scaffolding is deferred until a
  browser workflow exists in the first vertical slice, avoiding a placeholder
  browser test setup without behavior to exercise.
- The current implementation branch is Phase 2 — Core Domain.

## Phase 0 — Product / Architecture Baseline

**Complete.** Establish repository structure, design documentation, and
clean-room architecture guardrails.

## Phase 1 — Workspace & Quality Gates

**Complete in Phase 0.** The repository already contains:

- pnpm and Cargo workspaces
- React + Vite, strict TypeScript, ESLint, and Vitest
- frontend lint, typecheck, test, and build scripts
- Rust format, clippy, and test checks in GitHub Actions
- a GitHub Actions workflow running all of those checks

Rust formatting is enforced by `cargo fmt --check`; there is no separate
frontend formatter command (frontend code is covered by ESLint). Playwright
browser E2E setup remains intentionally deferred as described above.

## Phase 2 — schematic-core

Implement a format-independent domain model, coordinate semantics, selection
logic, palette identity, and internal chunk organization. No parser, WASM,
renderer, or web dependency belongs in this phase.

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
