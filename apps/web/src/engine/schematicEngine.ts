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
  schematic_preview_cleanup(ptr: number, len: number): number;
  schematic_commit_preview(): number;
  schematic_cancel_preview(): number;
  schematic_undo(): number;
  schematic_redo(): number;
  schematic_export_litematic(): number;
  schematic_get_status(): number;
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

  getStatus(): SessionStatus {
    const code = this.exports.schematic_get_status();
    return this.handleJsonResponse<SessionStatus>(code);
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
