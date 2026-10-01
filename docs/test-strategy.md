# Test Strategy

## Objectives

The project must validate correctness, compatibility, and performance across format parsing, editing operations, and browser behavior.

## Layers

### Unit tests

- domain-type behavior
- coordinate conversion and normalization
- chunk indexing
- selection bounds
- palette identity

### Integration tests

- command + patch interaction
- serialization/deserialize round-trips
- validation rules on representative documents

### Contract tests

- format adapter compatibility with expected artifact structure
- WASM JS API agreement

### Golden tests

- empty document
- single-region structure
- multi-region structure
- negative-size regions
- block entities
- entities
- modern-version structure
- large structure sample

### Property tests

- share invariants across randomized coordinates and block states
- verify local-block/chunk/index coordinate round-trips over large generated
  coordinate sets
- verify fill/replace/delete and inverse operations preserve semantics
- verify generated edit → undo → redo sequences preserve semantic block states

### E2E tests

- open fixture
- render in 3D
- select region
- fill/replace/delete
- undo/redo
- export and reload

### Performance tests

- parse and memory benchmarks for 1M, 5M, 10M blocks
- edit latency
- undo/redo latency
- mesh generation timing

## Quality gates

The repository enforces that:

- `cargo test` passes
- `cargo clippy` passes
- `cargo fmt --check` passes
- `pnpm lint` passes
- `pnpm typecheck` passes
- `pnpm test` passes
- `pnpm build` passes

## Regression rule

Whenever a bug fix is introduced, a failing test must be written first to reproduce the issue and then fixed to ensure the regression stays prevented.
