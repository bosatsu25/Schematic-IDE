# ADR 0003: WASM boundary and worker isolation

## Status

Accepted

## Context

Parsing, validation, and large edits can block the browser main thread. This is unacceptable for large structures or complex operations.

## Decision

Heavy operations are routed through Rust compiled to WebAssembly and invoked from web workers, with the UI layer only coordinating view state and commands.

## Consequences

- Main-thread responsiveness is preserved
- Large operations are more testable
- Messaging layer complexity increases
