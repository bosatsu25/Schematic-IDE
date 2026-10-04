const KNOWN_COLORS: Record<string, string> = {
  'minecraft:stone': '#7d7d7d',
  'minecraft:granite': '#9f6b53',
  'minecraft:diorite': '#bcbcbc',
  'minecraft:andesite': '#878787',
  'minecraft:dirt': '#866043',
  'minecraft:grass_block': '#5b8731',
  'minecraft:glass': '#a0d0e0',
  'minecraft:oak_planks': '#a07344',
  'minecraft:cobblestone': '#6b6b6b',
  'minecraft:sand': '#ded78a',
  'minecraft:chest': '#a26a27',
  'minecraft:item_frame': '#82542a',
};

export function getBlockColor(blockState: string): string {
  const baseName = blockState.split('[')[0];
  if (KNOWN_COLORS[baseName]) {
    return KNOWN_COLORS[baseName];
  }

  // Hash-based deterministic fallback
  let hash = 0;
  for (let i = 0; i < baseName.length; i++) {
    hash = (hash << 5) - hash + baseName.charCodeAt(i);
    hash |= 0;
  }
  const hue = Math.abs(hash) % 360;
  return `hsl(${hue}, 45%, 55%)`;
}
