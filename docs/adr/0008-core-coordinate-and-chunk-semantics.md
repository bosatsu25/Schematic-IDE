# ADR 0008: Core coordinate and chunk semantics

## Status

Accepted

## Context

The core must represent negative world coordinates, convert between region
coordinates without overflow, and expose internal storage chunks without
coupling to file-format coordinate conventions or Minecraft world chunks.

## Decision

- Document/world positions use signed 32-bit coordinates. Region-local block
  positions use signed 64-bit coordinates and are validated against a region's
  non-negative `Size`.
- Region bounds are minimum-inclusive and maximum-exclusive. The exclusive
  endpoint is calculated in signed 64-bit arithmetic, and zero-size regions are
  representable but contain no blocks.
- Negative-size format encodings are normalized by the format adapter to a
  non-negative size and a corresponding origin before entering the core.
- Internal storage chunks have a fixed logical extent of `16 × 16 × 16`. They
  are not Minecraft world chunks. Chunk coordinates use Euclidean division,
  and linear indices use `x + 16*z + 256*y`.
- Block-state properties are stored in key order so equality, hashing, and
  palette deduplication do not depend on property input order.
- The initial storage API uses safe, ordinary indexed storage; packed-bit
  representations or unsafe optimizations require benchmark evidence.

## Consequences

- Negative coordinate conversion is well-defined on both sides of zero.
- The type system distinguishes world positions from local positions.
- Coordinate overflow is rejected at checked world/local conversion boundaries.
- Format adapters own any source-format-specific size normalization.
- Internal storage representation can evolve behind the `Chunk` API.
