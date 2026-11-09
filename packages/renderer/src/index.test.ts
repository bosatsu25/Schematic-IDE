import { describe, expect, it } from 'vitest';
import * as THREE from 'three';
import {
  VoxelRenderer,
  getBlockColor,
  decodeFaceStream,
  buildChunkGeometry,
  packFace,
  RegionViewData,
} from './index';

/** Encodes chunks the way the engine does ([x, y, z, attr] faces are packed). */
function encode(chunks: { chunk: [number, number, number]; faces: number[][] }[]): ArrayBuffer {
  const ints: number[] = [chunks.length];
  for (const c of chunks) {
    ints.push(...c.chunk, c.faces.length);
    for (const [x, y, z, attr] of c.faces) ints.push(packFace(x, y, z, attr & 7, attr >> 3));
  }
  return new Int32Array(ints).buffer;
}

const face = (x: number, y: number, z: number, pal: number, dir: number) => [x, y, z, (pal << 3) | dir];

const view = (over: Partial<RegionViewData> = {}): RegionViewData => ({
  region_id: 'R',
  origin: [10, 0, 0],
  size: [32, 16, 16],
  palette: ['minecraft:stone', 'minecraft:granite'],
  preview_positions: [],
  ...over,
});

describe('block colors', () => {
  it('maps known and unknown blocks to deterministic colors', () => {
    expect(getBlockColor('minecraft:stone')).toBe('#7d7d7d');
    expect(getBlockColor('minecraft:stone[variant=smooth]')).toBe('#7d7d7d');
    const a = getBlockColor('custom:modded_block');
    expect(getBlockColor('custom:modded_block')).toBe(a);
    expect(a.startsWith('hsl(')).toBe(true);
  });
});

describe('face stream', () => {
  it('decodes chunks and faces', () => {
    const decoded = decodeFaceStream(
      encode([
        { chunk: [0, 0, 0], faces: [face(1, 2, 3, 70000, 4)] },
        { chunk: [1, 0, 0], faces: [] },
      ]),
    );
    expect(decoded).toHaveLength(2);
    expect(decoded[0].chunk).toEqual([0, 0, 0]);
    expect(Array.from(decoded[0].faces)).toEqual(face(1, 2, 3, 70000, 4));
    expect(decoded[1].faces.length).toBe(0);
  });

  it('builds 4 vertices / 2 triangles per face at world position with outward normals', () => {
    const colors = new Float32Array([1, 1, 1]);
    const geom = buildChunkGeometry(new Int32Array(face(0, 0, 0, 0, 2)), [5, 6, 7], colors);
    expect(geom.getAttribute('position').count).toBe(4);
    expect(geom.getIndex()!.count).toBe(6);
    const pos = geom.getAttribute('position');
    for (let i = 0; i < 4; i++) expect(pos.getY(i)).toBe(7); // top face of block y=6
    // Triangle winding faces +Y.
    const [a, b, c] = [0, 1, 2].map((i) => new THREE.Vector3().fromBufferAttribute(pos, i));
    const n = new THREE.Vector3().subVectors(b, a).cross(new THREE.Vector3().subVectors(c, a));
    expect(n.y).toBeGreaterThan(0);
  });
});

describe('VoxelRenderer (headless)', () => {
  it('rebuilds only the chunks contained in an incremental update', () => {
    const r = new VoxelRenderer();
    r.updateRegion(
      view(),
      encode([
        { chunk: [0, 0, 0], faces: [face(0, 0, 0, 0, 0)] },
        { chunk: [1, 0, 0], faces: [face(16, 0, 0, 1, 0)] },
      ]),
      true,
    );
    expect(r.chunkCount('R')).toBe(2);

    // Partial update: chunk 1 becomes empty, chunk 0 untouched.
    r.updateRegion(view(), encode([{ chunk: [1, 0, 0], faces: [] }]), false);
    expect(r.chunkCount('R')).toBe(1);

    // Full update replaces everything.
    r.updateRegion(view(), encode([{ chunk: [2, 0, 0], faces: [face(32, 0, 0, 0, 1)] }]), true);
    expect(r.chunkCount('R')).toBe(1);
    r.dispose();
  });

  it('holds several regions and drops removed ones', () => {
    const r = new VoxelRenderer();
    r.updateRegion(view(), encode([{ chunk: [0, 0, 0], faces: [face(0, 0, 0, 0, 0)] }]), true);
    r.updateRegion(
      view({ region_id: 'B', origin: [-20, 0, 0] }),
      encode([{ chunk: [0, 0, 0], faces: [face(0, 0, 0, 0, 0)] }]),
      true,
    );
    expect(r.regionIds().sort()).toEqual(['B', 'R']);
    r.setRegionStyle('B', { dimmed: true });
    r.setRegionStyle('R', { visible: false });
    r.retainRegions(['B']);
    expect(r.regionIds()).toEqual(['B']);
    r.setSelectionBoxes([
      { min: [0, 0, 0], max: [1, 1, 1] },
      { min: [5, 5, 5], max: [5, 5, 5] },
    ]);
    r.setHover([1, 2, 3]);
    r.setEntityMarkers([{ position: [0.5, 1, 0.5], kind: 'minecraft:pig' }]);
    r.setDiffOverlay({ added_positions: [[0, 0, 0]] });
    r.dispose();
  });

  it('maps a face hit to the block behind it in world and local coordinates', () => {
    const r = new VoxelRenderer();
    r.updateRegion(view(), encode([]), true);
    // Hitting the top face (+Y) of block (12, 3, 4) lands on y = 4.
    const hit = r.hitToBlock('R', new THREE.Vector3(12.3, 4, 4.7), new THREE.Vector3(0, 1, 0));
    expect(hit).toEqual({ regionId: 'R', world: [12, 3, 4], local: [2, 3, 4], normal: [0, 1, 0] });
    // Hitting the -X face of block (12, 3, 4) lands on x = 12.
    const side = r.hitToBlock('R', new THREE.Vector3(12, 3.5, 4.5), new THREE.Vector3(-1, 0, 0));
    expect(side!.world).toEqual([12, 3, 4]);
    r.dispose();
  });

  it('picks through the camera onto a rendered face', () => {
    const r = new VoxelRenderer();
    // Single block at local (0,0,0) => world (10,0,0); emit its top face.
    r.updateRegion(view({ size: [1, 1, 1] }), encode([{ chunk: [0, 0, 0], faces: [face(0, 0, 0, 0, 2)] }]), true);
    const cam = r.getCamera();
    cam.position.set(10.5, 10, 0.5);
    cam.lookAt(10.5, 0, 0.5);
    cam.updateMatrixWorld();
    const hit = r.pick(0, 0);
    expect(hit?.world).toEqual([10, 0, 0]);
    expect(hit?.normal).toEqual([0, 1, 0]);
    r.dispose();
  });
});
