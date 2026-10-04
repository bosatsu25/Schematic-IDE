import * as THREE from 'three';
import { getBlockColor } from './colorMap';

export { getBlockColor };

export interface RenderRegionOptions {
  origin: [number, number, number];
  size: [number, number, number];
  palette: string[];
  blocks: number[]; // flat [x, y, z, pal_idx, ...]
  preview_diff?: {
    modified_positions: [number, number, number][];
  };
}

export interface SelectionBoxCoords {
  min: [number, number, number];
  max: [number, number, number];
}

export class VoxelRenderer {
  private canvas?: HTMLCanvasElement;
  private renderer?: THREE.WebGLRenderer;
  private scene: THREE.Scene;
  private camera: THREE.PerspectiveCamera;
  private voxelGroup: THREE.Group;
  private wireframeGroup: THREE.Group;
  private previewHighlightGroup: THREE.Group;
  private selectionBoxMesh?: THREE.LineSegments;
  private regionBoxMesh?: THREE.LineSegments;
  private animFrameId?: number;

  constructor(canvas?: HTMLCanvasElement) {
    this.canvas = canvas;
    this.scene = new THREE.Scene();
    this.scene.background = new THREE.Color('#1a1a24');

    const aspect = canvas && canvas.clientHeight > 0
      ? canvas.clientWidth / canvas.clientHeight
      : 1;
    this.camera = new THREE.PerspectiveCamera(45, aspect, 0.1, 1000);
    this.camera.position.set(20, 20, 30);
    this.camera.lookAt(0, 0, 0);

    // Ambient and Directional Lights
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.7);
    this.scene.add(ambientLight);

    const dirLight = new THREE.DirectionalLight(0xffffff, 0.8);
    dirLight.position.set(50, 80, 50);
    this.scene.add(dirLight);

    // Groups
    this.voxelGroup = new THREE.Group();
    this.wireframeGroup = new THREE.Group();
    this.previewHighlightGroup = new THREE.Group();

    this.scene.add(this.voxelGroup);
    this.scene.add(this.wireframeGroup);
    this.scene.add(this.previewHighlightGroup);

    // Grid helper
    const grid = new THREE.GridHelper(32, 32, 0x444455, 0x222233);
    grid.position.y = -0.01;
    this.scene.add(grid);

    if (canvas) {
      try {
        this.renderer = new THREE.WebGLRenderer({
          canvas,
          antialias: true,
          alpha: false,
        });
        this.renderer.setSize(canvas.clientWidth, canvas.clientHeight, false);
      } catch {
        // Fallback for headless/unsupported WebGL
      }
    }
  }

  getScene(): THREE.Scene {
    return this.scene;
  }

  getCamera(): THREE.PerspectiveCamera {
    return this.camera;
  }

  renderRegion(options: RenderRegionOptions): void {
    // Clear previous voxels
    while (this.voxelGroup.children.length > 0) {
      const child = this.voxelGroup.children.pop()!;
      if (child instanceof THREE.Mesh) {
        child.geometry.dispose();
        if (Array.isArray(child.material)) {
          child.material.forEach((m) => m.dispose());
        } else {
          child.material.dispose();
        }
      }
    }

    // Clear preview highlights
    while (this.previewHighlightGroup.children.length > 0) {
      const child = this.previewHighlightGroup.children.pop()!;
      if (child instanceof THREE.LineSegments) {
        child.geometry.dispose();
        (child.material as THREE.Material).dispose();
      }
    }

    const { origin, size, palette, blocks, preview_diff } = options;

    // Region boundary wireframe
    if (this.regionBoxMesh) {
      this.wireframeGroup.remove(this.regionBoxMesh);
      this.regionBoxMesh.geometry.dispose();
      (this.regionBoxMesh.material as THREE.Material).dispose();
      this.regionBoxMesh = undefined;
    }

    const boxMin = new THREE.Vector3(origin[0], origin[1], origin[2]);
    const boxMax = new THREE.Vector3(
      origin[0] + size[0],
      origin[1] + size[1],
      origin[2] + size[2],
    );
    const boxGeom = new THREE.BoxGeometry(size[0], size[1], size[2]);
    const edges = new THREE.EdgesGeometry(boxGeom);
    this.regionBoxMesh = new THREE.LineSegments(
      edges,
      new THREE.LineBasicMaterial({ color: 0x4a9eff, linewidth: 2 }),
    );
    this.regionBoxMesh.position.set(
      origin[0] + size[0] / 2,
      origin[1] + size[1] / 2,
      origin[2] + size[2] / 2,
    );
    this.wireframeGroup.add(this.regionBoxMesh);

    // Group blocks by palette index
    const blocksByPalette = new Map<number, { x: number; y: number; z: number }[]>();
    for (let i = 0; i < blocks.length; i += 4) {
      const x = blocks[i];
      const y = blocks[i + 1];
      const z = blocks[i + 2];
      const palIdx = blocks[i + 3];

      if (!blocksByPalette.has(palIdx)) {
        blocksByPalette.set(palIdx, []);
      }
      blocksByPalette.get(palIdx)!.push({ x, y, z });
    }

    const boxGeometry = new THREE.BoxGeometry(1, 1, 1);
    const matrix = new THREE.Matrix4();

    for (const [palIdx, instances] of blocksByPalette.entries()) {
      const stateName = palette[palIdx] || 'minecraft:stone';
      const colorHex = getBlockColor(stateName);
      const material = new THREE.MeshLambertMaterial({
        color: new THREE.Color(colorHex),
      });

      const instMesh = new THREE.InstancedMesh(
        boxGeometry,
        material,
        instances.length,
      );

      for (let k = 0; k < instances.length; k++) {
        const { x, y, z } = instances[k];
        matrix.setPosition(
          origin[0] + x + 0.5,
          origin[1] + y + 0.5,
          origin[2] + z + 0.5,
        );
        instMesh.setMatrixAt(k, matrix);
      }
      instMesh.instanceMatrix.needsUpdate = true;
      this.voxelGroup.add(instMesh);
    }

    // Highlight preview diff positions if any
    if (preview_diff && preview_diff.modified_positions.length > 0) {
      const highlightGeom = new THREE.BoxGeometry(1.02, 1.02, 1.02);
      const highlightEdges = new THREE.EdgesGeometry(highlightGeom);
      const highlightMat = new THREE.LineBasicMaterial({
        color: 0x00ff88,
        linewidth: 2,
      });

      for (const pos of preview_diff.modified_positions) {
        const seg = new THREE.LineSegments(highlightEdges, highlightMat);
        seg.position.set(
          origin[0] + pos[0] + 0.5,
          origin[1] + pos[1] + 0.5,
          origin[2] + pos[2] + 0.5,
        );
        this.previewHighlightGroup.add(seg);
      }
    }

    // Center camera on the region
    this.centerCamera(boxMin, boxMax);
    this.render();
  }

  setSelectionBox(box: SelectionBoxCoords | null): void {
    if (this.selectionBoxMesh) {
      this.wireframeGroup.remove(this.selectionBoxMesh);
      this.selectionBoxMesh.geometry.dispose();
      (this.selectionBoxMesh.material as THREE.Material).dispose();
      this.selectionBoxMesh = undefined;
    }

    if (!box) {
      this.render();
      return;
    }

    const minX = Math.min(box.min[0], box.max[0]);
    const minY = Math.min(box.min[1], box.max[1]);
    const minZ = Math.min(box.min[2], box.max[2]);
    const maxX = Math.max(box.min[0], box.max[0]) + 1;
    const maxY = Math.max(box.min[1], box.max[1]) + 1;
    const maxZ = Math.max(box.min[2], box.max[2]) + 1;

    const sizeX = maxX - minX;
    const sizeY = maxY - minY;
    const sizeZ = maxZ - minZ;

    const geom = new THREE.BoxGeometry(sizeX, sizeY, sizeZ);
    const edges = new THREE.EdgesGeometry(geom);
    this.selectionBoxMesh = new THREE.LineSegments(
      edges,
      new THREE.LineBasicMaterial({ color: 0xffaa00, linewidth: 2 }),
    );
    this.selectionBoxMesh.position.set(
      minX + sizeX / 2,
      minY + sizeY / 2,
      minZ + sizeZ / 2,
    );
    this.wireframeGroup.add(this.selectionBoxMesh);
    this.render();
  }

  centerCamera(min?: THREE.Vector3, max?: THREE.Vector3): void {
    const center = new THREE.Vector3();
    let radius = 15;

    if (min && max) {
      center.addVectors(min, max).multiplyScalar(0.5);
      radius = min.distanceTo(max) * 0.8;
    }

    this.camera.position.set(
      center.x + radius,
      center.y + radius * 0.9,
      center.z + radius,
    );
    this.camera.lookAt(center);
  }

  resize(width: number, height: number): void {
    if (height === 0) return;
    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
    if (this.renderer) {
      this.renderer.setSize(width, height, false);
      this.render();
    }
  }

  render(): void {
    if (this.renderer) {
      this.renderer.render(this.scene, this.camera);
    }
  }

  dispose(): void {
    if (this.animFrameId) {
      cancelAnimationFrame(this.animFrameId);
    }
    if (this.renderer) {
      this.renderer.dispose();
    }
  }
}
