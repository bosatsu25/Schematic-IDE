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
});
