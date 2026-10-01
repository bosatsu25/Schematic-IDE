# ADR 0002: Local-first by default

## Status

Accepted

## Context

The product is a local editor for large structure files. Requiring a server would reduce performance and increase friction.

## Decision

Schematic IDE stores and manipulates schematic data locally in the browser and only uses remote services for optional deploys or optional sync features.

## Consequences

- No mandatory login or backend required
- Better privacy and performance
- Stronger need for browser-side memory management and worker isolation
