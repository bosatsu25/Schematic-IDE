# v1.0 Gap Analysis

Item-by-item verification of Schematic IDE against the **v1.0 Definition of
Done** in [docs/requirements.md](./requirements.md).

## Status rules

- **DONE**: implemented, covered by automated tests (named in *Evidence*), and
  no known gap. A row with anything in *Known gaps* cannot be DONE.
- **PARTIAL**: works, but a known gap is tracked in a linked issue.
- **TODO**: not implemented.

Earlier revisions of this file marked every row DONE even where the *Missing*
column listed open work. That was corrected in #34; see *History* below.

## Verification matrix

| Requirement | Status | Evidence | Known gaps |
|---|:---:|---|---|
| **Local-first execution** | **DONE** | Client-side React + WASM, no backend or account. Worker and E2E tests run fully offline after load. | — |
| **Web browser app** | **DONE** | React 18 + Vite app; `apps/web/e2e/*.spec.ts`. | — |
| **PWA / offline** | **DONE** | Build-time versioned precache (`apps/web/service-worker/`), update banner, `apps/web/e2e/offline.preview.spec.ts` (offline reload + file open against the production build). | — |
| **`.litematic` support** | **PARTIAL** | Lossless NBT + bit-packing; block entity / entity payloads via the core model; `crates/schematic-format/tests/litematic_tests.rs`, `multi_format_tests.rs`. | Negative-size regions are read from the minimum corner, as independent readers of the format do (test with hand-built NBT); a file written by Litematica itself has not been checked yet (#37). |
| **Sponge `.schem` support** | **DONE** | v1/v2/v3 parse, v2/v3 export, block entity / entity payloads, 65535-per-axis guard; `multi_format_tests.rs` (payload round trip v2 + v3). | — |
| **Java Structure NBT support** | **DONE** | Block `nbt` and entity `nbt` payloads; `multi_format_tests.rs`. | — |
| **Open & view** | **DONE** | File picker and drag & drop (#29); all regions rendered at their origins with visibility toggles and inactive dimming (#28); orbit controls; `e2e/ide-features.spec.ts`. | — |
| **Selection** | **DONE** | Viewport raycast picking: click, Shift+click box extend, Ctrl/⌘+click multi-box (#24); coordinate inputs; multi-box reaches every edit command (`SelectionBounds.extra_boxes`); `crates/schematic-wasm/tests/mesh_tests.rs`, `e2e/ide-features.spec.ts`. | — |
| **Fill / Replace / Delete** | **DONE** | Patch-based commands; block entities dropped when their block is cleared or changes type, entities removed by Delete; `crates/schematic-edit/tests/entity_edit_tests.rs`, E2E. | — |
| **Copy / Paste / Move / Rotate / Mirror** | **DONE** | Rotation and mirroring remap `facing`, `axis`, `rotation`, connection flags, rail and stair shapes, door hinges and double-chest halves. Clipboard carries block entities and entities. Move/Rotate/Mirror carry them along, including entity yaw and hanging-entity `Facing`; pasted entities drop `UUID`. 4× rotate and 2× mirror identities hold for entities too (#21); `entity_edit_tests.rs`, `transform_tests.rs` proptests. | — |
| **Undo / Redo** | **DONE** | `PatchSet` carries block, block-entity and entity changes with conflict validation; `history.rs`, `entity_edit_tests.rs`, session tests, E2E. | — |
| **Inspect** | **DONE** | Block inspection reads block entity NBT from the edited document; Entities tab lists block entities and entities, shows entity NBT, and moves/deletes entities (#31); session + E2E tests. | — |
| **Analyze** | **PARTIAL** | Materials, stacks, metrics, CSV export; session + E2E tests. Runs lazily when the Analysis tab opens. | Surface/feature analysis is skipped above 500,000 blocks per region because it is slow and memory-heavy (41 s / 6.6 GB native at 10M before the guard); materials stay exact (#36). |
| **Validate & repair** | **DONE** | `schematic-validate` rules, including orphan / out-of-region block entities. Automatically fixable diagnostics get a previewable, undoable *Fix* / *Fix all* (#25); `session_tests.rs`, E2E. | — |
| **Diff** | **DONE** | Canonical structural diff, including block entity payload changes, against the source or any `.litematic` / `.schem` / `.nbt` file (picker and drop target) (#30); diff + E2E tests. | — |
| **Export** | **DONE** | Three formats. Multi-region documents are flattened for `.schem` / `.nbt` (#20). Every export first lists what the target cannot represent and writes nothing until the user confirms (#22); session tests, E2E. | — |
| **Round-trip tests** | **DONE** | Format-level and cross-format chain round trips including block entity / entity payloads and multi-region flattening (`multi_format_tests.rs`), plus a session-level chest-move → export → reload test. | Real-world Litematica fixture still missing (#37). |
| **Property-based tests** | **DONE** | `crates/schematic-edit/tests/transform_tests.rs`. | — |
| **Browser E2E workflow** | **DONE** | `litematic-workflow.spec.ts` (edit lifecycle, loss dialog, multi-format export/reload), `ide-features.spec.ts` (picking, multi-region, entities, repair, cross-format diff, drag & drop), `offline.preview.spec.ts`. | — |
| **Large-structure performance** | **PARTIAL** | Dense 55%-fill fixtures at 1M/5M/10M, release-only native benchmarks, browser harness (#26); face-culled chunk meshes with dirty-chunk updates and a packed binary transfer (#27). Numbers in [benchmarks.md](./benchmarks.md). | Analysis (#36). Triangle count still too high for interactive frames at 10M on software GL; greedy meshing / LOD and a hardware-GPU measurement are tracked in #38. |
| **CI green** | **DONE** | Rust fmt/clippy/tests; WASM built from source in CI (#23); frontend lint/typecheck/tests; Playwright dev + preview projects. | — |
| **Docs aligned** | **DONE** | This file, requirements, roadmap and README reconciled with the code (#34). | — |
| **Corrupt-file safety** | **DONE** | Declared sizes and lengths are never trusted for allocation; short block data is an error, not air. Mutation tests on all three formats at parser and session level (`crates/schematic-format/tests/robustness_tests.rs`, `crates/schematic-wasm/tests/robustness_tests.rs`). If the engine still traps, the worker restarts it and the app closes the lost document. | — |
| **Zero silent data loss** | **DONE** | Block entity / entity payloads survive edits and every format (#21). Multi-region export is flattened, not truncated (#20). Lossy conversions are listed and need confirmation (#22). | — |

## History

- **#34 (this revision).** Before it, every row read DONE. These claims were
  false in the code at the time:
  - Block entities and entities did not follow edits, and Sponge / Structure
    dropped their NBT.
  - `.schem` / `.nbt` export silently kept only the first region.
  - No conversion-loss reporting existed.
  - Raycast selection, repair, drag & drop and the entity UI were missing.
  - Cross-format diff was limited to `.litematic` in the UI.
  - Benchmarks measured mostly-air volumes in debug builds.
  - The WASM binary was hand-committed and never rebuilt in CI.

  Fixed in issues #20–#33.
