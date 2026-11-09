import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { getBlockColor } from '@schematic-ide/minecraft-data';
import {
  buildCellOutlines,
  buildChunkGeometry,
  chunkKey,
  decodeFaceStream,
  paletteColors,
  packFace,
  unpackFace,
} from './chunkGeometry';

export {
  getBlockColor,
  decodeFaceStream,
  buildChunkGeometry,
  buildCellOutlines,
  chunkKey,
  packFace,
  unpackFace,
};
export type { ChunkFaces } from './chunkGeometry';

type Vec3 = [number, number, number];

/** Region metadata the renderer needs (mirrors the engine's `RegionView`). */
export interface RegionViewData {
  region_id: string;
  origin: Vec3;
  size: Vec3;
  palette: string[];
  /** Region-local positions changed by the pending preview. */
  preview_positions: Vec3[];
}

export interface SelectionBoxCoords {
  min: Vec3;
  max: Vec3;
}

export interface DiffOverlay {
  added_positions?: Vec3[];
  removed_positions?: Vec3[];
  modified_positions?: Vec3[];
}

export interface EntityMarker {
  /** World-space continuous position. */
  position: Vec3;
  kind: string;
  selected?: boolean;
}

export interface PickHit {
  regionId: string;
  /** World block coordinates of the hit block. */
  world: Vec3;
  /** Region-local block coordinates of the hit block. */
  local: Vec3;
  /** Outward normal of the hit face (adjacent cell = world + normal). */
  normal: Vec3;
}

export interface RegionStyle {
  visible: boolean;
  /** Drawn translucent (e.g. when another region is active). */
  dimmed: boolean;
}

/** Outline overlays above this many cells collapse to their bounding box. */
const MAX_OUTLINED_CELLS = 20000;

interface RegionEntry {
  view: RegionViewData;
  colors: Uint8Array;
  group: THREE.Group;
  chunks: Map<string, THREE.Mesh>;
  boundary: THREE.LineSegments;
  preview?: THREE.LineSegments;
  style: RegionStyle;
}

function disposeObject(obj: THREE.Object3D): void {
  obj.traverse((child) => {
    if (child instanceof THREE.Mesh || child instanceof THREE.LineSegments) {
      child.geometry.dispose();
    }
  });
}

export class VoxelRenderer {
  private renderer?: THREE.WebGLRenderer;
  private controls?: OrbitControls;
  private scene = new THREE.Scene();
  private camera: THREE.PerspectiveCamera;
  private regions = new Map<string, RegionEntry>();
  private overlayGroup = new THREE.Group();
  private selectionLines?: THREE.LineSegments;
  private hoverLines?: THREE.LineSegments;
  private diffGroup = new THREE.Group();
  private entityGroup = new THREE.Group();
  private raycaster = new THREE.Raycaster();
  private renderQueued = false;
  private hasFramed = false;

  private solidMaterial = new THREE.MeshBasicMaterial({ vertexColors: true });
  private dimmedMaterial = new THREE.MeshBasicMaterial({
    vertexColors: true,
    transparent: true,
    opacity: 0.3,
    depthWrite: false,
  });
  private boundaryMaterial = new THREE.LineBasicMaterial({ color: 0x4a9eff });
  private boundaryDimMaterial = new THREE.LineBasicMaterial({
    color: 0x4a9eff,
    transparent: true,
    opacity: 0.35,
  });
  private previewMaterial = new THREE.LineBasicMaterial({ color: 0x00ff88 });
  private selectionMaterial = new THREE.LineBasicMaterial({ color: 0xffaa00 });
  private hoverMaterial = new THREE.LineBasicMaterial({ color: 0xffffff });

  constructor(canvas?: HTMLCanvasElement) {
    this.scene.background = new THREE.Color('#1a1a24');
    const aspect = canvas && canvas.clientHeight > 0 ? canvas.clientWidth / canvas.clientHeight : 1;
    this.camera = new THREE.PerspectiveCamera(45, aspect, 0.1, 10000);
    this.camera.position.set(20, 20, 30);
    this.camera.lookAt(0, 0, 0);

    const grid = new THREE.GridHelper(32, 32, 0x444455, 0x222233);
    grid.position.y = -0.01;
    this.scene.add(grid, this.overlayGroup, this.diffGroup, this.entityGroup);

    if (canvas) {
      try {
        this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: false });
        this.renderer.setPixelRatio(typeof window !== 'undefined' ? window.devicePixelRatio : 1);
        this.renderer.setSize(canvas.clientWidth, canvas.clientHeight, false);
        this.controls = new OrbitControls(this.camera, canvas);
        this.controls.enableDamping = false;
        this.controls.addEventListener('change', () => this.requestRender());
      } catch {
        // Headless or WebGL-less environment: keep the scene graph only.
      }
    }
  }

  getScene(): THREE.Scene {
    return this.scene;
  }

  getCamera(): THREE.PerspectiveCamera {
    return this.camera;
  }

  /** Region ids currently held by the renderer. */
  regionIds(): string[] {
    return [...this.regions.keys()];
  }

  /** Number of chunk meshes held for a region (for tests/diagnostics). */
  chunkCount(regionId: string): number {
    return this.regions.get(regionId)?.chunks.size ?? 0;
  }

  /**
   * Applies a face buffer from the engine. With `replaceAll` every chunk of the
   * region is rebuilt; otherwise only the chunks contained in the buffer are
   * replaced (empty chunks are removed), leaving the rest untouched.
   */
  updateRegion(view: RegionViewData, faces: ArrayBuffer, replaceAll: boolean): void {
    let entry = this.regions.get(view.region_id);
    if (!entry) {
      const group = new THREE.Group();
      group.name = `region:${view.region_id}`;
      entry = {
        view,
        colors: paletteColors(view.palette),
        group,
        chunks: new Map(),
        boundary: new THREE.LineSegments(),
        style: { visible: true, dimmed: false },
      };
      this.scene.add(group);
      this.regions.set(view.region_id, entry);
    }

    const paletteChanged =
      entry.view.palette.length !== view.palette.length ||
      entry.view.palette.some((p, i) => p !== view.palette[i]);
    const boundsChanged =
      entry.view.origin.some((v, i) => v !== view.origin[i]) ||
      entry.view.size.some((v, i) => v !== view.size[i]);
    if (paletteChanged) entry.colors = paletteColors(view.palette);
    entry.view = view;

    if (replaceAll || boundsChanged) {
      for (const mesh of entry.chunks.values()) {
        entry.group.remove(mesh);
        mesh.geometry.dispose();
      }
      entry.chunks.clear();
    }

    for (const { chunk, faces: chunkFaces } of decodeFaceStream(faces)) {
      const key = chunkKey(chunk);
      const old = entry.chunks.get(key);
      if (old) {
        entry.group.remove(old);
        old.geometry.dispose();
        entry.chunks.delete(key);
      }
      if (chunkFaces.length === 0) continue;
      const mesh = new THREE.Mesh(
        buildChunkGeometry(chunkFaces, entry.colors, { releaseColors: !!this.renderer }),
        entry.style.dimmed ? this.dimmedMaterial : this.solidMaterial,
      );
      mesh.position.set(
        view.origin[0] + chunk[0] * 16,
        view.origin[1] + chunk[1] * 16,
        view.origin[2] + chunk[2] * 16,
      );
      mesh.matrixAutoUpdate = false;
      mesh.updateMatrix();
      mesh.userData = { regionId: view.region_id, chunk: key };
      entry.chunks.set(key, mesh);
      entry.group.add(mesh);
    }

    this.rebuildBoundary(entry);
    this.rebuildPreview(entry);
    if (!this.hasFramed) {
      this.fitToContent();
      this.hasFramed = true;
    }
    this.requestRender();
  }

  /** Drops regions that are no longer in the document. */
  retainRegions(ids: string[]): void {
    const keep = new Set(ids);
    for (const [id, entry] of this.regions) {
      if (!keep.has(id)) {
        this.scene.remove(entry.group);
        disposeObject(entry.group);
        this.regions.delete(id);
      }
    }
    if (this.regions.size === 0) this.hasFramed = false;
    this.requestRender();
  }

  setRegionStyle(regionId: string, style: Partial<RegionStyle>): void {
    const entry = this.regions.get(regionId);
    if (!entry) return;
    entry.style = { ...entry.style, ...style };
    entry.group.visible = entry.style.visible;
    for (const mesh of entry.chunks.values()) {
      mesh.material = entry.style.dimmed ? this.dimmedMaterial : this.solidMaterial;
      mesh.renderOrder = entry.style.dimmed ? 1 : 0;
    }
    entry.boundary.material = entry.style.dimmed ? this.boundaryDimMaterial : this.boundaryMaterial;
    this.requestRender();
  }

  private rebuildBoundary(entry: RegionEntry): void {
    entry.group.remove(entry.boundary);
    entry.boundary.geometry.dispose();
    const { origin, size } = entry.view;
    const geom = new THREE.EdgesGeometry(new THREE.BoxGeometry(size[0], size[1], size[2]));
    entry.boundary = new THREE.LineSegments(
      geom,
      entry.style.dimmed ? this.boundaryDimMaterial : this.boundaryMaterial,
    );
    entry.boundary.position.set(
      origin[0] + size[0] / 2,
      origin[1] + size[1] / 2,
      origin[2] + size[2] / 2,
    );
    entry.boundary.userData = { boundary: true };
    entry.group.add(entry.boundary);
  }

  private rebuildPreview(entry: RegionEntry): void {
    if (entry.preview) {
      entry.group.remove(entry.preview);
      entry.preview.geometry.dispose();
      entry.preview = undefined;
    }
    const cells = entry.view.preview_positions;
    if (cells.length === 0) return;
    const o = entry.view.origin;
    const world = cells.map((c) => [o[0] + c[0], o[1] + c[1], o[2] + c[2]]);
    entry.preview = new THREE.LineSegments(this.outlineGeometry(world), this.previewMaterial);
    entry.group.add(entry.preview);
  }

  /** Outlines cells, collapsing huge sets to their bounding box. */
  private outlineGeometry(cells: number[][]): THREE.BufferGeometry {
    if (cells.length <= MAX_OUTLINED_CELLS) return buildCellOutlines(cells);
    const min = [Infinity, Infinity, Infinity];
    const max = [-Infinity, -Infinity, -Infinity];
    for (const c of cells) {
      for (let i = 0; i < 3; i++) {
        min[i] = Math.min(min[i], c[i]);
        max[i] = Math.max(max[i], c[i] + 1);
      }
    }
    const size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    const geom = new THREE.EdgesGeometry(new THREE.BoxGeometry(size[0], size[1], size[2]));
    geom.translate(min[0] + size[0] / 2, min[1] + size[1] / 2, min[2] + size[2] / 2);
    return geom;
  }

  /** Draws one or more inclusive world-space selection boxes. */
  setSelectionBoxes(boxes: SelectionBoxCoords[] | null): void {
    if (this.selectionLines) {
      this.overlayGroup.remove(this.selectionLines);
      this.selectionLines.geometry.dispose();
      this.selectionLines = undefined;
    }
    if (boxes && boxes.length > 0) {
      const merged: number[] = [];
      for (const box of boxes) {
        const min = [0, 1, 2].map((i) => Math.min(box.min[i], box.max[i]));
        const max = [0, 1, 2].map((i) => Math.max(box.min[i], box.max[i]) + 1);
        const size = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
        const g = new THREE.EdgesGeometry(new THREE.BoxGeometry(size[0], size[1], size[2]));
        g.translate(min[0] + size[0] / 2, min[1] + size[1] / 2, min[2] + size[2] / 2);
        merged.push(...(g.getAttribute('position').array as Float32Array));
        g.dispose();
      }
      const geom = new THREE.BufferGeometry();
      geom.setAttribute('position', new THREE.Float32BufferAttribute(merged, 3));
      this.selectionLines = new THREE.LineSegments(geom, this.selectionMaterial);
      this.overlayGroup.add(this.selectionLines);
    }
    this.requestRender();
  }

  setSelectionBox(box: SelectionBoxCoords | null): void {
    this.setSelectionBoxes(box ? [box] : null);
  }

  /** Highlights the hovered block (world coordinates) or clears it. */
  setHover(world: Vec3 | null): void {
    if (this.hoverLines) {
      this.overlayGroup.remove(this.hoverLines);
      this.hoverLines.geometry.dispose();
      this.hoverLines = undefined;
    }
    if (world) {
      this.hoverLines = new THREE.LineSegments(buildCellOutlines([world], 0.03), this.hoverMaterial);
      this.overlayGroup.add(this.hoverLines);
    }
    this.requestRender();
  }

  setDiffOverlay(overlay: DiffOverlay | null): void {
    while (this.diffGroup.children.length > 0) {
      const child = this.diffGroup.children.pop()!;
      disposeObject(child);
    }
    if (overlay) {
      const add = (cells: Vec3[] | undefined, color: number) => {
        if (!cells || cells.length === 0) return;
        this.diffGroup.add(
          new THREE.LineSegments(
            this.outlineGeometry(cells),
            new THREE.LineBasicMaterial({ color }),
          ),
        );
      };
      add(overlay.added_positions, 0x22c55e);
      add(overlay.modified_positions, 0xf59e0b);
      add(overlay.removed_positions, 0xef4444);
    }
    this.requestRender();
  }

  /** Shows entities as small markers (selected ones larger and highlighted). */
  setEntityMarkers(markers: EntityMarker[]): void {
    while (this.entityGroup.children.length > 0) {
      disposeObject(this.entityGroup.children.pop()!);
    }
    if (markers.length > 0) {
      const geom = new THREE.OctahedronGeometry(0.25);
      for (const m of markers) {
        const mesh = new THREE.Mesh(
          geom,
          new THREE.MeshBasicMaterial({ color: m.selected ? 0xffff00 : 0xff66cc }),
        );
        mesh.position.set(m.position[0], m.position[1] + 0.25, m.position[2]);
        if (m.selected) mesh.scale.setScalar(1.6);
        mesh.userData = { entityKind: m.kind };
        this.entityGroup.add(mesh);
      }
    }
    this.requestRender();
  }

  /** Frames the camera on every visible region. */
  fitToContent(): void {
    const box = new THREE.Box3();
    for (const entry of this.regions.values()) {
      if (!entry.style.visible) continue;
      const { origin, size } = entry.view;
      box.expandByPoint(new THREE.Vector3(...origin));
      box.expandByPoint(
        new THREE.Vector3(origin[0] + size[0], origin[1] + size[1], origin[2] + size[2]),
      );
    }
    if (box.isEmpty()) return;
    const center = box.getCenter(new THREE.Vector3());
    const radius = Math.max(box.getSize(new THREE.Vector3()).length() * 0.8, 4);
    this.camera.position.set(center.x + radius, center.y + radius * 0.9, center.z + radius);
    this.camera.lookAt(center);
    if (this.controls) {
      this.controls.target.copy(center);
      this.controls.update();
    }
    this.requestRender();
  }

  /**
   * Picks the block under normalized device coordinates (-1..1). Returns the
   * nearest visible block face hit, or null.
   */
  pick(ndcX: number, ndcY: number): PickHit | null {
    this.scene.updateMatrixWorld();
    this.raycaster.setFromCamera(new THREE.Vector2(ndcX, ndcY), this.camera);
    const targets: THREE.Object3D[] = [];
    for (const entry of this.regions.values()) {
      if (!entry.style.visible) continue;
      targets.push(...entry.chunks.values());
    }
    const hit = this.raycaster.intersectObjects(targets, false)[0];
    if (!hit || !hit.face) return null;
    return this.hitToBlock(hit.object.userData.regionId as string, hit.point, hit.face.normal);
  }

  /** Picks using client (CSS pixel) coordinates relative to the canvas. */
  pickAtClient(clientX: number, clientY: number): PickHit | null {
    const canvas = this.renderer?.domElement;
    if (!canvas) return null;
    const rect = canvas.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return null;
    const x = ((clientX - rect.left) / rect.width) * 2 - 1;
    const y = -((clientY - rect.top) / rect.height) * 2 + 1;
    return this.pick(x, y);
  }

  /** Converts a surface hit into the block it belongs to. */
  hitToBlock(regionId: string, point: THREE.Vector3, normal: THREE.Vector3): PickHit | null {
    const entry = this.regions.get(regionId);
    if (!entry) return null;
    const n: Vec3 = [Math.round(normal.x), Math.round(normal.y), Math.round(normal.z)];
    const world: Vec3 = [
      Math.floor(point.x - n[0] * 0.5),
      Math.floor(point.y - n[1] * 0.5),
      Math.floor(point.z - n[2] * 0.5),
    ];
    const o = entry.view.origin;
    return {
      regionId,
      world,
      local: [world[0] - o[0], world[1] - o[1], world[2] - o[2]],
      normal: n,
    };
  }

  isRegionVisible(regionId: string): boolean {
    return this.regions.get(regionId)?.style.visible ?? false;
  }

  /** Projects a world point to client (CSS pixel) coordinates of the canvas. */
  projectToClient(world: Vec3): { x: number; y: number } | null {
    const canvas = this.renderer?.domElement;
    if (!canvas) return null;
    this.camera.updateMatrixWorld();
    const v = new THREE.Vector3(...world).project(this.camera);
    const rect = canvas.getBoundingClientRect();
    return {
      x: rect.left + ((v.x + 1) / 2) * rect.width,
      y: rect.top + ((1 - v.y) / 2) * rect.height,
    };
  }

  resize(width: number, height: number): void {
    if (height === 0) return;
    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
    if (this.renderer) {
      this.renderer.setSize(width, height, false);
    }
    this.requestRender();
  }

  /** Coalesces render requests into one frame. */
  requestRender(): void {
    if (!this.renderer || this.renderQueued) return;
    if (typeof requestAnimationFrame === 'undefined') {
      this.render();
      return;
    }
    this.renderQueued = true;
    requestAnimationFrame(() => {
      this.renderQueued = false;
      this.render();
    });
  }

  render(): void {
    this.renderer?.render(this.scene, this.camera);
  }

  dispose(): void {
    this.controls?.dispose();
    for (const entry of this.regions.values()) disposeObject(entry.group);
    this.regions.clear();
    this.renderer?.dispose();
  }
}
