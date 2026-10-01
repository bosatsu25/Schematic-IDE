# ADR 0001: Rust core owns the canonical structure model

## Status

Accepted

## Context

The app must handle large Minecraft structure files, many block manipulations, and format conversion safely. The UI should not become the source of truth for document state.

## Decision

A Rust core crate will own the canonical document model and editing logic. Browser-side code will receive derived view models and lightweight queries.

## Consequences

- Better correctness for data mutations
- Easier validation and round-trip guarantees
- Clear separation between UI state and structure state
- Requires WASM and worker boundaries for browser use
