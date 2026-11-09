# Development Loop

## Repeatable cycle

1. Define the design objective or failing behavior.
2. Add or refine tests to capture the requirement.
3. Implement the minimal change to satisfy the test.
4. Run focused verification for the affected scope.
5. Commit with a conventional commit message.
6. Push the branch.
7. Open and validate the PR.
8. Merge only when CI passes and the acceptance standard is met.
9. Update main and continue to the next phase.

## Building the WASM engine

The browser app loads the Rust engine from `apps/web/public/schematic_wasm.wasm`.
That file is generated, ignored by version control, and never committed:

- `pnpm build:wasm` (root) runs `scripts/build-wasm.mjs`, which executes
  `cargo build -p schematic-wasm --target wasm32-unknown-unknown --release` and
  copies `target/wasm32-unknown-unknown/release/schematic_wasm.wasm` into
  `apps/web/public/` (only when the bytes changed).
- `apps/web`'s `dev`, `build`, `test`, and `test:e2e` scripts call it first, so
  a fresh checkout or a Rust change is picked up automatically. Vitest's engine
  tests read the binary from `public/`, so it must exist before `pnpm test`.
- Requires the target: `rustup target add wasm32-unknown-unknown`.
  `SKIP_WASM_BUILD=1` reuses an existing binary without invoking cargo.
- CI's frontend job installs Rust with the wasm32 target and runs
  `pnpm build:wasm` before lint, test, build, and E2E.

## Decision records

Architecture decisions and material changes should be captured in the ADR folder so that the reasoning remains discoverable over time.

## Guardrails

- Keep each phase small and reviewable.
- Avoid broad speculative work before a failing test or acceptance check exists.
- Do not add hidden migration risk without explicit justification.
- Keep the format adapters and core domain model separate.
- Preserve unknown data instead of silently stripping it.
