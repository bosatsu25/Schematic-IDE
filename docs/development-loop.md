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

## Decision records

Architecture decisions and material changes should be captured in the ADR folder so that the reasoning remains discoverable over time.

## Guardrails

- Keep each phase small and reviewable.
- Avoid broad speculative work before a failing test or acceptance check exists.
- Do not add hidden migration risk without explicit justification.
- Keep the format adapters and core domain model separate.
- Preserve unknown data instead of silently stripping it.
