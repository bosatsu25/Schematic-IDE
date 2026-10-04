# VoxelWeave Retirement Checklist

This checklist tracks the requirements for safely retiring independent development of VoxelWeave and preparing the GitHub repository for archival.

## 1. Safety & Data Preservation Gates

- [x] **Git History Bundle**: Complete repository history saved to `C:\Users\tkmnk\Backups\VoxelWeave\2026-10-04\voxelweave-all.bundle` (SHA-256: `BE7F8EE6FEB758FBC7C68C6A4B016966FCF8089EE27DF43C1E396F7CD234BDD4`).
- [x] **Restore Verification**: Restored into `restore-test\VoxelWeave` and verified via `git fsck --full` (0 errors) and matching commit tree.
- [x] **Untracked / Work-in-Progress Assets**: Untracked configurations, profiles, and logs from `run/` archived under `untracked/run/`.
- [x] **GitHub Issues & PRs Export**: All 23 issues, comments, 15+ pull requests, review comments, releases, and repository metadata exported to JSON under `github/` with SHA-256 manifest.
- [x] **Wiki & LFS Check**: Verified no GitHub Wiki exists (`has_wiki: false`) and no Git LFS pointers are configured.
- [x] **ZIP Archive Cataloged**: Downloaded `VoxelWeave-main.zip` cataloged (SHA-256: `AAEC020C1A076C037CF8F406F2C10CD7E0409A02AD19474D72164EEFADC0F227`).

## 2. Feature Parity & Porting Gates

- [x] **Coordinate & Selection Domain**: Multi-box unions, bounds intersections, and safe coordinate mappings ported to `schematic-core` / `schematic-edit`.
- [x] **Edit Engine**: Reversible patch history, chunk-sparse operations, preview/commit/undo/redo, and bounded block replacement ported to `schematic-edit`.
- [x] **Analysis Engine**: Conservative 6-neighbor surface topology, higher-order feature classification, and bounded connected protrusion analysis ported to `schematic-analysis`.
- [x] **Conservative Cleanup**: Disconnected island cleanup with shape-feature preservation vetoes and snapshot provenance checks ported to `schematic-edit`.
- [x] **Provenance & Stale Result Prevention**: Revision tokens guarantee that topology/cleanup results cannot be applied to altered or undone document states.

## 3. Quality & Regression Gates

- [x] **Rust Format & Linting**: `cargo fmt --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass cleanly.
- [x] **Workspace Test Suite**: `cargo test --workspace --all-targets` passes all tests across all crates.
- [x] **Web & Shared Packages**: `pnpm lint`, `pnpm typecheck`, `pnpm test`, and `pnpm build` pass with zero errors.
- [x] **Independent Checkout Verification**: Repository builds and passes all tests without external references to VoxelWeave directories.

## 4. Archival Recommendation

- [ ] **GitHub Archive**: Once integrated and merged to `main`, set the `bosatsu25/VoxelWeave` repository to **Archived** (read-only) via GitHub repository settings.
- [ ] **README Notice**: Update VoxelWeave's `README.md` to point users to [Schematic IDE](https://github.com/bosatsu25/Schematic-IDE) as the unified structure editor.
- [ ] **No Permanent Deletion**: Keep repository archived and local backups retained so issues, discussions, and commit history remain accessible if needed.
