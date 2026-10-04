import { describe, expect, it, beforeAll } from 'vitest';
import * as fs from 'fs';
import * as path from 'path';
import { SchematicEngine } from './schematicEngine';

describe('SchematicEngine WASM wrapper', () => {
  let engine: SchematicEngine;
  let sampleBytes: Uint8Array;

  beforeAll(async () => {
    const wasmPath = path.resolve(__dirname, '../../public/schematic_wasm.wasm');
    const wasmBuffer = fs.readFileSync(wasmPath);

    engine = new SchematicEngine();
    await engine.init(wasmBuffer);

    const fixturePath = path.resolve(__dirname, '../../public/sample.litematic');
    sampleBytes = new Uint8Array(fs.readFileSync(fixturePath));
  });

  it('reports initial status as unloaded', () => {
    const status = engine.getStatus();
    expect(status.loaded).toBe(false);
    expect(status.has_preview).toBe(false);
    expect(status.can_undo).toBe(false);
  });

  it('loads litematic document and extracts metadata and regions', () => {
    const summary = engine.loadLitematic(sampleBytes);
    expect(summary.name).toBe('WasmTestSchematic');
    expect(summary.author).toBe('TestBot');
    expect(summary.regions.length).toBe(1);
    expect(summary.regions[0].name).toBe('MainRegion');
    expect(summary.regions[0].non_air_blocks).toBe(12);

    const status = engine.getStatus();
    expect(status.loaded).toBe(true);
    expect(status.is_dirty).toBe(false);
  });

  it('fetches region mesh data with compact voxels', () => {
    const mesh = engine.getRegionMesh('MainRegion');
    expect(mesh.region_id).toBe('MainRegion');
    expect(mesh.palette.length).toBeGreaterThanOrEqual(2);
    // 12 blocks * 4 ints (x, y, z, pal_idx) = 48 ints
    expect(mesh.blocks.length).toBe(48);
    expect(mesh.preview_diff).toBeUndefined();
  });

  it('runs preview replace and updates preview diff', () => {
    const prev = engine.previewReplace({
      region_id: 'MainRegion',
      selection: {
        min: [0, 0, 0],
        max: [10, 5, 5],
      },
      from_block: 'minecraft:stone',
      to_block: 'minecraft:granite',
    });

    expect(prev.changed_count).toBe(11);
    expect(prev.can_commit).toBe(true);

    const status = engine.getStatus();
    expect(status.has_preview).toBe(true);

    const meshWithPrev = engine.getRegionMesh('MainRegion');
    expect(meshWithPrev.preview_diff).toBeDefined();
    expect(meshWithPrev.preview_diff?.modified_positions.length).toBe(11);
  });

  it('commits preview, tests undo, redo, and exports lossless litematic', () => {
    // Commit
    const commitRes = engine.commitPreview();
    expect(commitRes.can_undo).toBe(true);
    expect(commitRes.has_preview).toBe(false);
    expect(commitRes.is_dirty).toBe(true);

    // Undo
    const undoRes = engine.undo();
    expect(undoRes.can_undo).toBe(false);
    expect(undoRes.can_redo).toBe(true);
    expect(undoRes.is_dirty).toBe(false);

    // Redo
    const redoRes = engine.redo();
    expect(redoRes.can_undo).toBe(true);
    expect(redoRes.can_redo).toBe(false);
    expect(redoRes.is_dirty).toBe(true);

    // Export
    const exportedBytes = engine.exportLitematic();
    expect(exportedBytes.length).toBeGreaterThan(0);

    // Reload in a new engine instance
    const engine2 = new SchematicEngine();
    const wasmPath = path.resolve(__dirname, '../../public/schematic_wasm.wasm');
    const wasmBuffer = fs.readFileSync(wasmPath);
    return engine2.init(wasmBuffer).then(() => {
      const summary2 = engine2.loadLitematic(exportedBytes);
      expect(summary2.name).toBe('WasmTestSchematic');
      expect(summary2.regions[0].name).toBe('MainRegion');
    });
  });

  it('inspects block, document, and validates document diagnostics', () => {
    // 1. Inspect block
    const block = engine.inspectBlock({
      region_id: 'MainRegion',
      x: 0,
      y: 1,
      z: 1,
    });
    expect(block).toBeDefined();
    expect(block?.block_id).toBe('minecraft:granite'); // because replace was redone!
    expect(block?.local_position).toEqual([0, 1, 1]);

    // 2. Inspect document
    const doc = engine.inspectDocument();
    expect(doc.metadata.name).toBe('WasmTestSchematic');
    expect(doc.regions.length).toBe(1);
    expect(doc.regions[0].palette_size).toBeGreaterThanOrEqual(2);

    // 3. Validate document
    const diags = engine.validateDocument();
    expect(Array.isArray(diags)).toBe(true);
    const errors = diags.filter((d) => d.severity === 'Error');
    expect(errors.length).toBe(0);
  });

  it('executes structural editing: fill, copy, paste, move, rotate, mirror', () => {
    // 1. Fill 2x1x2 with oak_planks
    const fillRes = engine.previewFill({
      region_id: 'MainRegion',
      selection: {
        min: [2, 2, 2],
        max: [3, 2, 3],
      },
      block: 'minecraft:oak_planks',
    });
    expect(fillRes.changed_count).toBe(4);
    engine.commitPreview();

    // 2. Copy selection
    const copyCount = engine.copySelection({
      region_id: 'MainRegion',
      selection: {
        min: [2, 2, 2],
        max: [3, 2, 3],
      },
    });
    expect(copyCount).toBe(4);

    // 3. Paste at (6, 2, 6)
    const pasteRes = engine.previewPaste({
      region_id: 'MainRegion',
      target: [6, 2, 6],
    });
    expect(pasteRes.changed_count).toBe(4);
    engine.commitPreview();

    // 4. Move pasted blocks by (0, 1, 0)
    const moveRes = engine.previewMove({
      region_id: 'MainRegion',
      selection: {
        min: [6, 2, 6],
        max: [7, 2, 7],
      },
      delta: [0, 1, 0],
    });
    expect(moveRes.changed_count).toBe(8);
    engine.commitPreview();

    // 5. Clear half and Rotate by 90 deg
    engine.previewFill({
      region_id: 'MainRegion',
      selection: {
        min: [6, 3, 7],
        max: [7, 3, 7],
      },
      block: 'minecraft:air',
    });
    engine.commitPreview();

    const rotateRes = engine.previewRotate({
      region_id: 'MainRegion',
      selection: {
        min: [6, 3, 6],
        max: [7, 3, 7],
      },
      angle_deg: 90,
    });
    expect(rotateRes.can_commit).toBe(true);
    engine.commitPreview();

    // 6. Mirror along X
    const mirrorRes = engine.previewMirror({
      region_id: 'MainRegion',
      selection: {
        min: [6, 3, 6],
        max: [7, 3, 7],
      },
      axis: 'x',
    });
    expect(mirrorRes.can_commit).toBe(true);
    engine.commitPreview();

    // 7. Undo
    const undoHist = engine.undo();
    expect(undoHist.can_undo).toBe(true);
  });

  it('computes canonical structural diff against source and another file', () => {
    // Reload fresh sample document
    engine.loadLitematic(sampleBytes);

    // Initial diff with source should be empty (0 changes)
    const initialDiff = engine.diffWithSource();
    expect(initialDiff.total_added).toBe(0);
    expect(initialDiff.total_removed).toBe(0);
    expect(initialDiff.total_modified).toBe(0);
    expect(initialDiff.total_unchanged).toBe(12);

    // Perform an edit: fill a new block at (14, 14, 14)
    engine.previewFill({
      region_id: 'MainRegion',
      selection: {
        min: [14, 14, 14],
        max: [14, 14, 14],
      },
      block: 'minecraft:emerald_block',
    });
    engine.commitPreview();

    // Now diff with source has 1 added block
    const editedDiff = engine.diffWithSource();
    expect(editedDiff.total_added).toBe(1);
    expect(editedDiff.block_diffs.length).toBe(1);
    expect(editedDiff.block_diffs[0].kind).toBe('Added');
    expect(editedDiff.block_diffs[0].position).toEqual([14, 14, 14]);
    expect(editedDiff.block_diffs[0].after?.name).toBe('minecraft:emerald_block');

    // Diffing current edited document with original sampleBytes produces 1 added block
    const fileDiff = engine.diffWithLitematic(sampleBytes);
    expect(fileDiff.total_added).toBe(1);
  });

  it('supports multi-format cross-export and import for Sponge .schem and Structure .nbt', async () => {
    engine.loadLitematic(sampleBytes);

    // Export to Sponge .schem
    const spongeBytes = engine.exportSponge();
    expect(spongeBytes.length).toBeGreaterThan(0);

    // Load Sponge into a second engine
    const engineSponge = new SchematicEngine();
    const wasmPath = path.resolve(__dirname, '../../public/schematic_wasm.wasm');
    const wasmBuffer = fs.readFileSync(wasmPath);
    await engineSponge.init(wasmBuffer);
    const spongeSummary = engineSponge.loadLitematic(spongeBytes);
    expect(spongeSummary.regions.length).toBe(1);
    expect(spongeSummary.regions[0].non_air_blocks).toBe(12);

    // Export from Sponge engine to Structure .nbt
    const structureBytes = engineSponge.exportStructure();
    expect(structureBytes.length).toBeGreaterThan(0);

    // Load Structure into a third engine
    const engineStruct = new SchematicEngine();
    await engineStruct.init(wasmBuffer);
    const structSummary = engineStruct.loadLitematic(structureBytes);
    expect(structSummary.regions.length).toBe(1);
    expect(structSummary.regions[0].non_air_blocks).toBe(12);

    // Diff between Sponge engine and Structure engine
    const diff = engineStruct.diffWithLitematic(spongeBytes);
    expect(diff.total_added).toBe(0);
    expect(diff.total_removed).toBe(0);
    expect(diff.total_modified).toBe(0);
  });
});
