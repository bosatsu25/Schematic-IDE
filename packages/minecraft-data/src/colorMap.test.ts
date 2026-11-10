import { describe, expect, it } from 'vitest';
import { getBlockColor } from './index';

describe('getBlockColor', () => {
  it('returns the palette color for known blocks', () => {
    expect(getBlockColor('minecraft:stone')).toBe('#7d7d7d');
    expect(getBlockColor('minecraft:granite')).toBe('#9f6b53');
    expect(getBlockColor('minecraft:grass_block')).toBe('#5b8731');
  });

  it('ignores block-state properties', () => {
    expect(getBlockColor('minecraft:stone[variant=smooth]')).toBe('#7d7d7d');
    expect(getBlockColor('minecraft:chest[facing=north,type=single]')).toBe(
      getBlockColor('minecraft:chest'),
    );
  });

  it('derives a deterministic HSL fallback for unknown blocks', () => {
    const a = getBlockColor('custom:modded_block');
    expect(a).toBe(getBlockColor('custom:modded_block'));
    expect(a).toBe(getBlockColor('custom:modded_block[powered=true]'));
    expect(a).toMatch(/^hsl\(\d{1,3}, 45%, 55%\)$/);
    expect(getBlockColor('custom:other_block')).not.toBe(a);
  });
});
