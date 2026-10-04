import { describe, expect, it } from 'vitest';
import { VoxelRenderer, getBlockColor } from './index';

describe('VoxelRenderer', () => {
  it('maps known and unknown blocks to deterministic colors', () => {
    expect(getBlockColor('minecraft:stone')).toBe('#7d7d7d');
    expect(getBlockColor('minecraft:stone[variant=smooth]')).toBe('#7d7d7d');
    expect(getBlockColor('minecraft:granite')).toBe('#9f6b53');

    const customColor1 = getBlockColor('custom:modded_block');
    const customColor2 = getBlockColor('custom:modded_block');
    expect(customColor1).toBe(customColor2);
    expect(customColor1.startsWith('hsl(')).toBe(true);
  });

  it('instantiates scene, camera and groups without crashing in headless environment', () => {
    const renderer = new VoxelRenderer();
    expect(renderer.getScene()).toBeDefined();
    expect(renderer.getCamera()).toBeDefined();

    // Renders sample region data
    renderer.renderRegion({
      origin: [0, 0, 0],
      size: [16, 16, 16],
      palette: ['minecraft:stone', 'minecraft:granite'],
      blocks: [0, 0, 0, 0, 1, 1, 1, 1],
      preview_diff: {
        modified_positions: [[1, 1, 1]],
      },
    });

    // Sets selection box
    renderer.setSelectionBox({
      min: [0, 0, 0],
      max: [5, 5, 5],
    });

    // Clears selection box
    renderer.setSelectionBox(null);

    renderer.dispose();
  });
});
