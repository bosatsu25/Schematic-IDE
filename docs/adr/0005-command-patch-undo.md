# ADR 0005: Command + patch undo/redo

## Status

Accepted

## Context

Full-document snapshots are too expensive for large editing sessions.

## Decision

Operations will be represented as commands that emit reversible patches. History will track patches and their inverse operations instead of duplicating whole document state.

Each patch is scoped to one region and internal storage chunk and contains only
changed block positions with their before/after palette indices. Applying or
reverting a patch first validates all expected block states, so a conflict
cannot partially apply a multi-chunk patch set.

## Consequences

- Lower memory use
- Faster undo/redo
- More precise history semantics and diff tracking
