# ADR 0006: Format adapters separate domain logic from file formats

## Status

Accepted

## Context

Multiple Minecraft structure formats differ in metadata, packing, and region conventions. Hard-coding support into the core would create brittle logic.

## Decision

The core domain model is format-agnostic. Adapters implement parse, serialize, and conversion logic for each supported file type.

## Consequences

- Extensible support for `.litematic`, `.schem`, and Java Structure NBT
- Clear compatibility contracts
- Easier testing and round-trip validation
