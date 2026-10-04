export interface DocumentSummary {
  name: string;
  author: string;
  description: string;
  minecraft_data_version: number;
  version: number;
  regions: RegionSummary[];
}

export interface RegionSummary {
  name: string;
  origin: [number, number, number];
  size: [number, number, number];
  non_air_blocks: number;
  palette: string[];
}

export interface RegionMeshData {
  region_id: string;
  origin: [number, number, number];
  size: [number, number, number];
  palette: string[];
  blocks: number[]; // flat [x, y, z, pal_idx, ...]
  preview_diff?: {
    modified_positions: [number, number, number][];
  };
}

export interface SelectionBounds {
  min: [number, number, number];
  max: [number, number, number];
}

export interface FillRequest {
  region_id: string;
  selection: SelectionBounds;
  block: string;
}

export interface CopyRequest {
  region_id: string;
  selection: SelectionBounds;
}

export interface PasteRequest {
  region_id: string;
  target: [number, number, number];
}

export interface MoveRequest {
  region_id: string;
  selection: SelectionBounds;
  delta: [number, number, number];
}

export interface RotateRequest {
  region_id: string;
  selection: SelectionBounds;
  angle_deg: number;
}

export interface MirrorRequest {
  region_id: string;
  selection: SelectionBounds;
  axis: 'x' | 'z' | string;
}

export interface ReplaceRequest {
  region_id: string;
  selection?: SelectionBounds;
  from_block: string;
  to_block: string;
}

export interface CleanupRequest {
  region_id: string;
  max_size: number;
  replacement_block?: string;
  protected_kinds: string[];
}

export interface PreviewSummary {
  changed_count: number;
  can_commit: boolean;
  message: string;
}

export interface HistorySummary {
  can_undo: boolean;
  can_redo: boolean;
  has_preview: boolean;
  is_dirty: boolean;
}

export interface SessionStatus {
  loaded: boolean;
  has_preview: boolean;
  can_undo: boolean;
  can_redo: boolean;
  is_dirty: boolean;
}

export type DiagnosticSeverity = 'Error' | 'Warning' | 'Info';
export type Fixability = 'None' | 'Manual' | 'Automatic';

export interface Diagnostic {
  severity: DiagnosticSeverity;
  code: string;
  message: string;
  region?: string;
  position?: [number, number, number];
  fixability: Fixability;
}

export interface BlockInspectionRequest {
  region_id: string;
  x: number;
  y: number;
  z: number;
}

export interface BlockInspection {
  region_id: string;
  local_position: [number, number, number];
  world_position: [number, number, number];
  block_id: string;
  properties: Record<string, string>;
  palette_index: number;
  block_entity?: unknown;
}

export interface DocumentInspection {
  metadata: {
    name?: string;
    author?: string;
    description?: string;
    minecraft_data_version: number;
    version: number;
  };
  regions: {
    name: string;
    origin: [number, number, number];
    size: [number, number, number];
    palette_size: number;
    non_air_blocks: number;
    block_entity_count: number;
    entity_count: number;
  }[];
  total_entities: number;
  total_block_entities: number;
  raw_nbt_keys: string[];
}

export type DiffKind = 'Added' | 'Removed' | 'Modified' | 'Unchanged';

export interface DiffBlockState {
  name: string;
  properties: Record<string, string>;
}

export interface BlockDiff {
  region_id: string;
  position: [number, number, number];
  local_position: [number, number, number];
  kind: DiffKind;
  before?: DiffBlockState | null;
  after?: DiffBlockState | null;
}

export interface EntityDiff {
  position: [number, number, number];
  kind: DiffKind;
  before_type?: string | null;
  after_type?: string | null;
}

export interface RegionDiffSummary {
  region_id: string;
  added_count: number;
  removed_count: number;
  modified_count: number;
  unchanged_count: number;
}

export interface DocumentDiff {
  summaries: RegionDiffSummary[];
  total_added: number;
  total_removed: number;
  total_modified: number;
  total_unchanged: number;
  block_diffs: BlockDiff[];
  entity_diffs: EntityDiff[];
}

interface WasmExports {
  memory: WebAssembly.Memory;
  wasm_alloc(size: number): number;
  wasm_dealloc(ptr: number, size: number): void;
  wasm_get_response_ptr(): number;
  wasm_get_response_len(): number;
  schematic_init(): number;
  schematic_load_litematic(ptr: number, len: number): number;
  schematic_get_region_mesh(ptr: number, len: number): number;
  schematic_preview_replace(ptr: number, len: number): number;
  schematic_preview_fill(ptr: number, len: number): number;
  schematic_copy_selection(ptr: number, len: number): number;
  schematic_preview_paste(ptr: number, len: number): number;
  schematic_preview_move(ptr: number, len: number): number;
  schematic_preview_rotate(ptr: number, len: number): number;
  schematic_preview_mirror(ptr: number, len: number): number;
  schematic_preview_cleanup(ptr: number, len: number): number;
  schematic_commit_preview(): number;
  schematic_cancel_preview(): number;
  schematic_undo(): number;
  schematic_redo(): number;
  schematic_export_litematic(): number;
  schematic_export_sponge(): number;
  schematic_export_structure(): number;
  schematic_get_status(): number;
  schematic_inspect_block(ptr: number, len: number): number;
  schematic_inspect_document(): number;
  schematic_validate_document(): number;
  schematic_diff_with_source(): number;
  schematic_diff_with_litematic(ptr: number, len: number): number;
}

export class SchematicEngine {
  private exports!: WasmExports;
  private textEncoder = new TextEncoder();
  private textDecoder = new TextDecoder();

  async init(wasmSource?: BufferSource | Response | string): Promise<void> {
    let instance: WebAssembly.Instance;

    if (!wasmSource) {
      // Default to fetch /schematic_wasm.wasm
      const response = await fetch('/schematic_wasm.wasm');
      const buffer = await response.arrayBuffer();
      const result = await WebAssembly.instantiate(buffer, {});
      instance = result.instance;
    } else if (typeof wasmSource === 'string') {
      const response = await fetch(wasmSource);
      const buffer = await response.arrayBuffer();
      const result = await WebAssembly.instantiate(buffer, {});
      instance = result.instance;
    } else if (wasmSource instanceof Response) {
      const buffer = await wasmSource.arrayBuffer();
      const result = await WebAssembly.instantiate(buffer, {});
      instance = result.instance;
    } else {
      const result = await WebAssembly.instantiate(wasmSource, {});
      instance = result.instance;
    }

    this.exports = instance.exports as unknown as WasmExports;
    this.exports.schematic_init();
  }

  loadLitematic(data: Uint8Array): DocumentSummary {
    const code = this.callWithBytes(data, (ptr, len) =>
      this.exports.schematic_load_litematic(ptr, len),
    );
    return this.handleJsonResponse<DocumentSummary>(code);
  }

  getRegionMesh(regionName: string): RegionMeshData {
    const bytes = this.textEncoder.encode(regionName);
    const code = this.callWithBytes(bytes, (ptr, len) =>
      this.exports.schematic_get_region_mesh(ptr, len),
    );
    return this.handleJsonResponse<RegionMeshData>(code);
  }

  previewReplace(req: ReplaceRequest): PreviewSummary {
    const jsonBytes = this.textEncoder.encode(JSON.stringify(req));
    const code = this.callWithBytes(jsonBytes, (ptr, len) =>
      this.exports.schematic_preview_replace(ptr, len),
    );
    return this.handleJsonResponse<PreviewSummary>(code);
  }

  previewFill(req: FillRequest): PreviewSummary {
    const jsonBytes = this.textEncoder.encode(JSON.stringify(req));
    const code = this.callWithBytes(jsonBytes, (ptr, len) =>
      this.exports.schematic_preview_fill(ptr, len),
    );
    return this.handleJsonResponse<PreviewSummary>(code);
  }

  copySelection(req: CopyRequest): number {
    const jsonBytes = this.textEncoder.encode(JSON.stringify(req));
    const code = this.callWithBytes(jsonBytes, (ptr, len) =>
      this.exports.schematic_copy_selection(ptr, len),
    );
    return this.handleJsonResponse<number>(code);
  }

  previewPaste(req: PasteRequest): PreviewSummary {
    const jsonBytes = this.textEncoder.encode(JSON.stringify(req));
    const code = this.callWithBytes(jsonBytes, (ptr, len) =>
      this.exports.schematic_preview_paste(ptr, len),
    );
    return this.handleJsonResponse<PreviewSummary>(code);
  }

  previewMove(req: MoveRequest): PreviewSummary {
    const jsonBytes = this.textEncoder.encode(JSON.stringify(req));
    const code = this.callWithBytes(jsonBytes, (ptr, len) =>
      this.exports.schematic_preview_move(ptr, len),
    );
    return this.handleJsonResponse<PreviewSummary>(code);
  }

  previewRotate(req: RotateRequest): PreviewSummary {
    const jsonBytes = this.textEncoder.encode(JSON.stringify(req));
    const code = this.callWithBytes(jsonBytes, (ptr, len) =>
      this.exports.schematic_preview_rotate(ptr, len),
    );
    return this.handleJsonResponse<PreviewSummary>(code);
  }

  previewMirror(req: MirrorRequest): PreviewSummary {
    const jsonBytes = this.textEncoder.encode(JSON.stringify(req));
    const code = this.callWithBytes(jsonBytes, (ptr, len) =>
      this.exports.schematic_preview_mirror(ptr, len),
    );
    return this.handleJsonResponse<PreviewSummary>(code);
  }

  previewCleanup(req: CleanupRequest): PreviewSummary {
    const jsonBytes = this.textEncoder.encode(JSON.stringify(req));
    const code = this.callWithBytes(jsonBytes, (ptr, len) =>
      this.exports.schematic_preview_cleanup(ptr, len),
    );
    return this.handleJsonResponse<PreviewSummary>(code);
  }

  commitPreview(): HistorySummary {
    const code = this.exports.schematic_commit_preview();
    return this.handleJsonResponse<HistorySummary>(code);
  }

  cancelPreview(): HistorySummary {
    const code = this.exports.schematic_cancel_preview();
    return this.handleJsonResponse<HistorySummary>(code);
  }

  undo(): HistorySummary {
    const code = this.exports.schematic_undo();
    return this.handleJsonResponse<HistorySummary>(code);
  }

  redo(): HistorySummary {
    const code = this.exports.schematic_redo();
    return this.handleJsonResponse<HistorySummary>(code);
  }

  exportLitematic(): Uint8Array {
    const code = this.exports.schematic_export_litematic();
    if (code !== 0) {
      this.handleJsonResponse(code); // throws error
    }
    const ptr = this.exports.wasm_get_response_ptr();
    const len = this.exports.wasm_get_response_len();
    return new Uint8Array(this.exports.memory.buffer, ptr, len).slice();
  }

  exportSponge(): Uint8Array {
    const code = this.exports.schematic_export_sponge();
    if (code !== 0) {
      this.handleJsonResponse(code); // throws error
    }
    const ptr = this.exports.wasm_get_response_ptr();
    const len = this.exports.wasm_get_response_len();
    return new Uint8Array(this.exports.memory.buffer, ptr, len).slice();
  }

  exportStructure(): Uint8Array {
    const code = this.exports.schematic_export_structure();
    if (code !== 0) {
      this.handleJsonResponse(code); // throws error
    }
    const ptr = this.exports.wasm_get_response_ptr();
    const len = this.exports.wasm_get_response_len();
    return new Uint8Array(this.exports.memory.buffer, ptr, len).slice();
  }

  getStatus(): SessionStatus {
    const code = this.exports.schematic_get_status();
    return this.handleJsonResponse<SessionStatus>(code);
  }

  inspectBlock(req: BlockInspectionRequest): BlockInspection | null {
    const bytes = this.textEncoder.encode(JSON.stringify(req));
    const code = this.callWithBytes(bytes, (ptr, len) =>
      this.exports.schematic_inspect_block(ptr, len),
    );
    return this.handleJsonResponse<BlockInspection | null>(code);
  }

  inspectDocument(): DocumentInspection {
    const code = this.exports.schematic_inspect_document();
    return this.handleJsonResponse<DocumentInspection>(code);
  }

  validateDocument(): Diagnostic[] {
    const code = this.exports.schematic_validate_document();
    return this.handleJsonResponse<Diagnostic[]>(code);
  }

  diffWithSource(): DocumentDiff {
    const code = this.exports.schematic_diff_with_source();
    return this.handleJsonResponse<DocumentDiff>(code);
  }

  diffWithLitematic(bytes: Uint8Array): DocumentDiff {
    const code = this.callWithBytes(bytes, (ptr, len) =>
      this.exports.schematic_diff_with_litematic(ptr, len),
    );
    return this.handleJsonResponse<DocumentDiff>(code);
  }

  private callWithBytes(
    bytes: Uint8Array,
    fn: (ptr: number, len: number) => number,
  ): number {
    const ptr = this.exports.wasm_alloc(bytes.length);
    new Uint8Array(this.exports.memory.buffer, ptr, bytes.length).set(bytes);
    try {
      return fn(ptr, bytes.length);
    } finally {
      this.exports.wasm_dealloc(ptr, bytes.length);
    }
  }

  private handleJsonResponse<T>(code: number): T {
    const ptr = this.exports.wasm_get_response_ptr();
    const len = this.exports.wasm_get_response_len();
    const bytes = new Uint8Array(this.exports.memory.buffer, ptr, len);
    const jsonStr = this.textDecoder.decode(bytes);

    if (code !== 0) {
      try {
        const errObj = JSON.parse(jsonStr);
        throw new Error(errObj.error || jsonStr);
      } catch (e: unknown) {
        if (e instanceof Error && e.message !== jsonStr) {
          throw e;
        }
        throw new Error(jsonStr);
      }
    }

    return JSON.parse(jsonStr) as T;
  }
}
