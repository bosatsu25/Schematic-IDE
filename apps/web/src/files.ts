/** File extensions the engine can open. */
export const SCHEMATIC_EXTENSIONS = ['.litematic', '.schem', '.nbt'] as const;

/** `accept` attribute value for file inputs. */
export const SCHEMATIC_ACCEPT = SCHEMATIC_EXTENSIONS.join(',');

export function isSchematicFile(name: string): boolean {
  const lower = name.toLowerCase();
  return SCHEMATIC_EXTENSIONS.some((ext) => lower.endsWith(ext));
}
