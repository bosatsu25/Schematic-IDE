# ADR 0007: Lossless round-trip is a required invariant

## Status

Accepted

## Context

Many schematics contain metadata, unknown NBT fields, entities, and block entity details that must survive save cycles.

## Decision

Format adapters will preserve unknown fields and compare semantic equivalence after parse → serialize → parse cycles using round-trip fixtures.

## Consequences

- Safer tool usage for real-world schematics
- More complex but more trustworthy format handling
- A stronger regression net around compatibility and metadata retention
