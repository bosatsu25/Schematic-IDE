import * as THREE from 'three';
import { getBlockColor } from '@schematic-ide/minecraft-data';

/** Faces of one 16³ storage chunk, as produced by the engine. */
export interface ChunkFaces {
  chunk: [number, number, number];
  /**
   * One packed `u32` per face: in-chunk offset x (bits 0–3), y (4–7),
   * z (8–11), direction (12–14) and palette index (15–31). A view into the
   * transferred buffer; no copy is made.
   */
  faces: Uint32Array;
}

export interface UnpackedFace {
  /** Offset inside the chunk (0–15). */
  x: number;
  y: number;
  z: number;
  /** 0 +X, 1 −X, 2 +Y, 3 −Y, 4 +Z, 5 −Z. */
  dir: number;
  palette: number;
}

export function unpackFace(v: number): UnpackedFace {
  return {
    x: v & 15,
    y: (v >>> 4) & 15,
    z: (v >>> 8) & 15,
    dir: (v >>> 12) & 7,
    palette: v >>> 15,
  };
}

/** Packs a face the way the engine does (tests and tools). */
export function packFace(x: number, y: number, z: number, dir: number, palette: number): number {
  return ((x & 15) | ((y & 15) << 4) | ((z & 15) << 8) | (dir << 12) | (palette << 15)) >>> 0;
}

/**
 * Decodes the engine's binary face stream: `[chunkCount]`, then per chunk
 * `[cx, cy, cz, faceCount, ...faceCount packed u32]`.
 */
export function decodeFaceStream(buffer: ArrayBuffer): ChunkFaces[] {
  const ints = new Int32Array(buffer);
  const words = new Uint32Array(buffer);
  const out: ChunkFaces[] = [];
  if (ints.length === 0) return out;
  let i = 1;
  for (let c = 0; c < ints[0]; c++) {
    const chunk: [number, number, number] = [ints[i], ints[i + 1], ints[i + 2]];
    const n = ints[i + 3];
    i += 4;
    out.push({ chunk, faces: words.subarray(i, i + n) });
    i += n;
  }
  return out;
}

export const chunkKey = (c: [number, number, number]) => `${c[0]},${c[1]},${c[2]}`;

// Quad corners per direction (+X, -X, +Y, -Y, +Z, -Z), counter-clockwise
// when seen from outside so front faces point along the normal.
const CORNERS: number[][][] = [
  [[1, 0, 0], [1, 1, 0], [1, 1, 1], [1, 0, 1]],
  [[0, 0, 0], [0, 0, 1], [0, 1, 1], [0, 1, 0]],
  [[0, 1, 0], [0, 1, 1], [1, 1, 1], [1, 1, 0]],
  [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]],
  [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]],
  [[0, 0, 0], [0, 1, 0], [1, 1, 0], [1, 0, 0]],
];
/** Baked directional shading so faces read without dynamic lights. */
const SHADE = [0.8, 0.8, 1.0, 0.5, 0.65, 0.65];

/** sRGB bytes per palette index. */
export function paletteColors(palette: string[]): Uint8Array {
  const colors = new Uint8Array(palette.length * 3);
  const c = new THREE.Color();
  palette.forEach((name, i) => {
    c.set(name ? getBlockColor(name) : '#ff00ff');
    colors[i * 3] = Math.round(c.r * 255);
    colors[i * 3 + 1] = Math.round(c.g * 255);
    colors[i * 3 + 2] = Math.round(c.b * 255);
  });
  return colors;
}

/** Drops a CPU-side copy once three.js has uploaded it to the GPU. */
function releaseAfterUpload(this: THREE.BufferAttribute): void {
  (this as unknown as { array: ArrayLike<number> | null }).array = null;
}

/**
 * Builds one merged, indexed geometry for a chunk's visible faces in
 * chunk-local coordinates (0–16); place the mesh at the chunk's world corner.
 *
 * Memory per face: 12 B of `Uint8` positions and 12–24 B of indices are kept
 * (needed for picking); 12 B of `Uint8` colors are released after upload.
 * No normal attribute is stored: materials are unlit and picking derives
 * face normals from the triangle.
 */
export function buildChunkGeometry(
  faces: Uint32Array,
  colors: Uint8Array,
  options: { releaseColors?: boolean } = {},
): THREE.BufferGeometry {
  const count = faces.length;
  const positions = new Uint8Array(count * 12);
  const vertexColors = new Uint8Array(count * 12);
  const indices = count * 4 > 65535 ? new Uint32Array(count * 6) : new Uint16Array(count * 6);

  for (let f = 0; f < count; f++) {
    const v = faces[f];
    const x = v & 15;
    const y = (v >>> 4) & 15;
    const z = (v >>> 8) & 15;
    const dir = (v >>> 12) & 7;
    const pal = v >>> 15;
    const shade = SHADE[dir] ?? 1;
    const r = Math.round((colors[pal * 3] ?? 255) * shade);
    const g = Math.round((colors[pal * 3 + 1] ?? 0) * shade);
    const b = Math.round((colors[pal * 3 + 2] ?? 255) * shade);
    const corners = CORNERS[dir];
    for (let k = 0; k < 4; k++) {
      const corner = corners[k];
      const o = (f * 4 + k) * 3;
      positions[o] = x + corner[0];
      positions[o + 1] = y + corner[1];
      positions[o + 2] = z + corner[2];
      vertexColors[o] = r;
      vertexColors[o + 1] = g;
      vertexColors[o + 2] = b;
    }
    const base = f * 4;
    const io = f * 6;
    indices[io] = base;
    indices[io + 1] = base + 1;
    indices[io + 2] = base + 2;
    indices[io + 3] = base;
    indices[io + 4] = base + 2;
    indices[io + 5] = base + 3;
  }

  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
  const colorAttr = new THREE.BufferAttribute(vertexColors, 3, true);
  if (options.releaseColors) colorAttr.onUpload(releaseAfterUpload);
  geometry.setAttribute('color', colorAttr);
  geometry.setIndex(new THREE.BufferAttribute(indices, 1));
  geometry.computeBoundingBox();
  geometry.computeBoundingSphere();
  return geometry;
}

/** One merged line geometry outlining unit cells (world coordinates). */
export function buildCellOutlines(
  cells: ArrayLike<number>[],
  inflate = 0.02,
): THREE.BufferGeometry {
  const edges = [
    [0, 1], [1, 2], [2, 3], [3, 0],
    [4, 5], [5, 6], [6, 7], [7, 4],
    [0, 4], [1, 5], [2, 6], [3, 7],
  ];
  const lo = -inflate;
  const hi = 1 + inflate;
  const corner = (i: number) => [i & 1 ? hi : lo, i & 4 ? hi : lo, i & 2 ? hi : lo];
  // Order corners so 0-1-2-3 is the bottom ring and 4-7 the top ring.
  const ring = [0, 1, 3, 2, 4, 5, 7, 6].map(corner);
  const positions = new Float32Array(cells.length * edges.length * 6);
  let o = 0;
  for (const cell of cells) {
    for (const [a, b] of edges) {
      for (const p of [ring[a], ring[b]]) {
        positions[o++] = cell[0] + p[0];
        positions[o++] = cell[1] + p[1];
        positions[o++] = cell[2] + p[2];
      }
    }
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
  return geometry;
}
