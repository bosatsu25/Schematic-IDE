import * as THREE from 'three';
import { getBlockColor } from '@schematic-ide/minecraft-data';

/** Faces of one 16³ storage chunk, as produced by the engine. */
export interface ChunkFaces {
  chunk: [number, number, number];
  /** `[x, y, z, (paletteIndex << 3) | direction]` per face, region-local. */
  faces: Int32Array;
}

/**
 * Decodes the engine's binary face stream: `[chunkCount]`, then per chunk
 * `[cx, cy, cz, faceCount, ...faceCount packed u32]` where each face packs
 * the in-chunk offset (x bits 0–3, y 4–7, z 8–11), direction (12–14) and
 * palette index (15–31). Faces are expanded to region-local
 * `[x, y, z, (palette << 3) | dir]` for geometry building.
 */
export function decodeFaceStream(buffer: ArrayBuffer): ChunkFaces[] {
  const ints = new Int32Array(buffer);
  const out: ChunkFaces[] = [];
  if (ints.length === 0) return out;
  let i = 1;
  for (let c = 0; c < ints[0]; c++) {
    const chunk: [number, number, number] = [ints[i], ints[i + 1], ints[i + 2]];
    const n = ints[i + 3];
    i += 4;
    const faces = new Int32Array(n * 4);
    const bx = chunk[0] * 16;
    const by = chunk[1] * 16;
    const bz = chunk[2] * 16;
    for (let f = 0; f < n; f++) {
      const v = ints[i + f] >>> 0;
      faces[f * 4] = bx + (v & 15);
      faces[f * 4 + 1] = by + ((v >>> 4) & 15);
      faces[f * 4 + 2] = bz + ((v >>> 8) & 15);
      faces[f * 4 + 3] = ((v >>> 15) << 3) | ((v >>> 12) & 7);
    }
    out.push({ chunk, faces });
    i += n;
  }
  return out;
}

/** Packs a face the way the engine does (used by tests and tools). */
export function packFace(x: number, y: number, z: number, dir: number, palette: number): number {
  return ((x & 15) | ((y & 15) << 4) | ((z & 15) << 8) | (dir << 12) | (palette << 15)) | 0;
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
const NORMALS = [
  [1, 0, 0],
  [-1, 0, 0],
  [0, 1, 0],
  [0, -1, 0],
  [0, 0, 1],
  [0, 0, -1],
];
/** Baked directional shading so faces read without dynamic lights. */
const SHADE = [0.8, 0.8, 1.0, 0.5, 0.65, 0.65];

/** Linear RGB triples per palette index. */
export function paletteColors(palette: string[]): Float32Array {
  const colors = new Float32Array(palette.length * 3);
  const c = new THREE.Color();
  palette.forEach((name, i) => {
    c.set(name ? getBlockColor(name) : '#ff00ff');
    colors[i * 3] = c.r;
    colors[i * 3 + 1] = c.g;
    colors[i * 3 + 2] = c.b;
  });
  return colors;
}

/**
 * Builds one merged, indexed geometry for a chunk's visible faces, placed in
 * world space at `origin` + region-local block coordinates.
 */
export function buildChunkGeometry(
  faces: Int32Array,
  origin: [number, number, number],
  colors: Float32Array,
): THREE.BufferGeometry {
  const count = faces.length / 4;
  const positions = new Float32Array(count * 12);
  const normals = new Float32Array(count * 12);
  const vertexColors = new Float32Array(count * 12);
  const indices = count * 4 > 65535 ? new Uint32Array(count * 6) : new Uint16Array(count * 6);

  for (let f = 0; f < count; f++) {
    const x = origin[0] + faces[f * 4];
    const y = origin[1] + faces[f * 4 + 1];
    const z = origin[2] + faces[f * 4 + 2];
    const attr = faces[f * 4 + 3];
    const dir = attr & 7;
    const pal = attr >> 3;
    const shade = SHADE[dir];
    const r = (colors[pal * 3] ?? 1) * shade;
    const g = (colors[pal * 3 + 1] ?? 0) * shade;
    const b = (colors[pal * 3 + 2] ?? 1) * shade;
    const n = NORMALS[dir];
    for (let v = 0; v < 4; v++) {
      const corner = CORNERS[dir][v];
      const o = (f * 4 + v) * 3;
      positions[o] = x + corner[0];
      positions[o + 1] = y + corner[1];
      positions[o + 2] = z + corner[2];
      normals[o] = n[0];
      normals[o + 1] = n[1];
      normals[o + 2] = n[2];
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
  geometry.setAttribute('normal', new THREE.BufferAttribute(normals, 3));
  geometry.setAttribute('color', new THREE.BufferAttribute(vertexColors, 3));
  geometry.setIndex(new THREE.BufferAttribute(indices, 1));
  geometry.computeBoundingSphere();
  geometry.computeBoundingBox();
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
