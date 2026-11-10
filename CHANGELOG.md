# Changelog

All notable changes to Schematic IDE are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

First public release: a local-first, browser-based IDE for Minecraft structure
files. Everything runs in the browser (Rust engine compiled to WebAssembly);
no file leaves the device.

### Features

- **Formats**: open and export Litematica `.litematic`, Sponge `.schem`
  (v1–v3 read, v2/v3 write) and Java Structure `.nbt`, with block entity and
  entity data preserved. Format is detected from the file contents.
- **View**: 3D viewport with orbit / pan / zoom, all regions shown at their
  positions, per-region visibility, entity markers.
- **Select**: click a block in the viewport, Shift+click to extend a box,
  Ctrl/⌘+click to add boxes; or type coordinates.
- **Edit** (all previewed before commit, all undoable): fill, replace, delete,
  copy / paste, move, rotate, mirror, small-island cleanup. Chest contents and
  other block entity data, and entities, follow the blocks.
- **Inspect**: block state, block entity NBT, document metadata, an Entities
  panel to list, move and delete entities.
- **Validate & repair**: diagnostics with one-click, undoable automatic fixes.
- **Diff**: against the opened file or any other schematic, with a 3D overlay.
- **Analyze**: material list with stack counts and CSV copy, structure
  statistics.
- **Export safety**: before a cross-format export the app lists everything the
  target format cannot hold and writes nothing until confirmed.
- **Offline**: installable PWA; works without a network after the first visit
  and can be hosted under any path.

### Robustness

- Corrupt or hostile files produce an error message instead of crashing the
  engine (covered by mutation tests on all three formats).
- If the engine does crash, it restarts and the app says so.

### Known limitations

- Negative-size Litematica regions follow the layout used by independent
  readers of the format, but have not yet been checked against a file written
  by Litematica itself ([#37](https://github.com/bosatsu25/Schematic-IDE/issues/37)).
- Surface / feature analysis is skipped for regions over 500,000 blocks
  ([#36](https://github.com/bosatsu25/Schematic-IDE/issues/36)).
- Very large structures (millions of blocks) load, but rendering them is slow
  without a capable GPU ([#38](https://github.com/bosatsu25/Schematic-IDE/issues/38)).
- Structure `.nbt` export is limited to 16,777,216 cells; Sponge `.schem` to
  65,535 blocks per axis.
- Blocks are drawn as flat-colored cubes, not with Minecraft textures or
  models.
- Rotation and mirroring remap the vanilla orientation properties (`facing`,
  `axis`, `rotation`, the `north`/`east`/`south`/`west` connection flags, rail
  and stair shapes, door hinges, double-chest halves). Orientation properties
  that only modded blocks use are left unchanged.

[0.1.0]: https://github.com/bosatsu25/Schematic-IDE/releases/tag/v0.1.0
