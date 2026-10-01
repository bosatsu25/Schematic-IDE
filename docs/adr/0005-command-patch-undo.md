# ADR 0005: Command + patch undo/redo

## Status

Accepted

## Context

Full-document snapshots are too expensive for large editing sessions.

## Decision

Operations will be represented as commands that emit reversible patches. History will track patches and their inverse operations instead of duplicating whole document state.

## Consequences

- Lower memory use
- Faster undo/redo
- More precise history semantics and diff tracking
