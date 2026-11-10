/**
 * Known Minecraft Java Edition world DataVersion numbers, as stored in
 * schematic files (`MinecraftDataVersion` in .litematic, `DataVersion` in
 * Sponge .schem and structure .nbt), mapped to the release they belong to.
 *
 * Source: https://minecraft.wiki/w/Data_version
 */
export const KNOWN_DATA_VERSIONS: ReadonlyMap<number, string> = new Map([
  [1343, '1.12.2'],
  [1631, '1.13.2'],
  [1976, '1.14.4'],
  [2230, '1.15.2'],
  [2586, '1.16.5'],
  [2730, '1.17.1'],
  [2975, '1.18.2'],
  [3120, '1.19.2'],
  [3337, '1.19.4'],
  [3465, '1.20.1'],
  [3578, '1.20.2'],
  [3700, '1.20.4'],
  [3839, '1.20.6'],
  [3953, '1.21'],
  [3955, '1.21.1'],
  [4189, '1.21.4'],
]);

/**
 * Human-readable label for a DataVersion.
 *
 * - Exact matches return the release name (`3465` -> `"1.20.1"`).
 * - Other versions are labelled relative to the closest older known release,
 *   e.g. `3500` -> `"1.20.1+ (3500)"` (usually a snapshot or minor release).
 * - Versions older than every known release give `"pre-1.12.2 (n)"`.
 * - Non-positive or non-integer input gives `"unknown"`.
 */
export function dataVersionLabel(dataVersion: number): string {
  if (!Number.isInteger(dataVersion) || dataVersion <= 0) {
    return 'unknown';
  }
  const exact = KNOWN_DATA_VERSIONS.get(dataVersion);
  if (exact) {
    return exact;
  }

  let closestOlder: string | undefined;
  for (const [version, label] of KNOWN_DATA_VERSIONS) {
    if (version < dataVersion) {
      closestOlder = label;
    }
  }
  if (closestOlder === undefined) {
    const oldest = KNOWN_DATA_VERSIONS.values().next().value as string;
    return `pre-${oldest} (${dataVersion})`;
  }
  return `${closestOlder}+ (${dataVersion})`;
}
