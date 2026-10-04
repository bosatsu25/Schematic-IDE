import { create } from 'zustand';
import {
  DocumentSummary,
  RegionMeshData,
  PreviewSummary,
  SessionStatus,
  SelectionBounds,
} from '../engine/schematicEngine';
import { SchematicWorkerClient } from '../worker/workerClient';

export interface EditorState {
  client: SchematicWorkerClient;
  document: DocumentSummary | null;
  selectedRegionId: string | null;
  meshData: RegionMeshData | null;
  selection: SelectionBounds | null;
  status: SessionStatus;
  previewSummary: PreviewSummary | null;
  error: string | null;
  loading: boolean;
  exportedBlob: Blob | null;

  init: () => Promise<void>;
  loadFile: (file: File) => Promise<void>;
  loadBuffer: (buffer: ArrayBuffer, name?: string) => Promise<void>;
  selectRegion: (regionId: string) => Promise<void>;
  setSelection: (bounds: SelectionBounds | null) => void;
  selectAllRegion: () => void;
  previewReplace: (fromBlock: string, toBlock: string) => Promise<void>;
  previewCleanup: (
    maxSize: number,
    replacement: string,
    protectedKinds: string[],
  ) => Promise<void>;
  commitPreview: () => Promise<void>;
  cancelPreview: () => Promise<void>;
  undo: () => Promise<void>;
  redo: () => Promise<void>;
  exportFile: (defaultFilename?: string) => Promise<Uint8Array | null>;
}

export const useSchematicStore = create<EditorState>((set, get) => {
  const client = new SchematicWorkerClient();

  return {
    client,
    document: null,
    selectedRegionId: null,
    meshData: null,
    selection: null,
    status: {
      loaded: false,
      has_preview: false,
      can_undo: false,
      can_redo: false,
      is_dirty: false,
    },
    previewSummary: null,
    error: null,
    loading: false,
    exportedBlob: null,

    init: async () => {
      try {
        await client.init();
      } catch (err: unknown) {
        console.error('Failed to initialize worker:', err);
      }
    },

    loadFile: async (file: File) => {
      set({ loading: true, error: null });
      try {
        const buffer = await file.arrayBuffer();
        await get().loadBuffer(buffer, file.name);
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    loadBuffer: async (buffer: ArrayBuffer, name?: string) => {
      set({ loading: true, error: null });
      try {
        const summary = await client.loadLitematic(buffer);
        if (name && !summary.name) {
          summary.name = name;
        }

        const initialRegion = summary.regions.length > 0 ? summary.regions[0].name : null;
        set({
          document: summary,
          selectedRegionId: initialRegion,
          previewSummary: null,
          selection: null,
        });

        if (initialRegion) {
          await get().selectRegion(initialRegion);
        }

        const status = await client.getStatus();
        set({ status, loading: false });
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    selectRegion: async (regionId: string) => {
      set({ selectedRegionId: regionId, loading: true, error: null });
      try {
        const meshData = await client.getRegionMesh(regionId);
        set({ meshData, loading: false });
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    setSelection: (bounds: SelectionBounds | null) => {
      set({ selection: bounds });
    },

    selectAllRegion: () => {
      const { meshData } = get();
      if (!meshData) return;
      const [ox, oy, oz] = meshData.origin;
      const [sx, sy, sz] = meshData.size;
      set({
        selection: {
          min: [ox, oy, oz],
          max: [ox + (sx > 0 ? sx - 1 : 0), oy + (sy > 0 ? sy - 1 : 0), oz + (sz > 0 ? sz - 1 : 0)],
        },
      });
    },

    previewReplace: async (fromBlock: string, toBlock: string) => {
      const { selectedRegionId, selection } = get();
      if (!selectedRegionId) return;

      set({ loading: true, error: null });
      try {
        const prev = await client.previewReplace({
          region_id: selectedRegionId,
          selection: selection || undefined,
          from_block: fromBlock,
          to_block: toBlock,
        });

        const status = await client.getStatus();
        const meshData = await client.getRegionMesh(selectedRegionId);
        set({
          previewSummary: prev,
          status,
          meshData,
          loading: false,
        });
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    previewCleanup: async (
      maxSize: number,
      replacement: string,
      protectedKinds: string[],
    ) => {
      const { selectedRegionId } = get();
      if (!selectedRegionId) return;

      set({ loading: true, error: null });
      try {
        const prev = await client.previewCleanup({
          region_id: selectedRegionId,
          max_size: maxSize,
          replacement_block: replacement,
          protected_kinds: protectedKinds,
        });

        const status = await client.getStatus();
        const meshData = await client.getRegionMesh(selectedRegionId);
        set({
          previewSummary: prev,
          status,
          meshData,
          loading: false,
        });
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    commitPreview: async () => {
      const { selectedRegionId } = get();
      set({ loading: true, error: null });
      try {
        const hist = await client.commitPreview();
        const meshData = selectedRegionId
          ? await client.getRegionMesh(selectedRegionId)
          : null;
        set({
          status: {
            ...get().status,
            ...hist,
          },
          previewSummary: null,
          meshData,
          loading: false,
        });
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    cancelPreview: async () => {
      const { selectedRegionId } = get();
      set({ loading: true, error: null });
      try {
        const hist = await client.cancelPreview();
        const meshData = selectedRegionId
          ? await client.getRegionMesh(selectedRegionId)
          : null;
        set({
          status: {
            ...get().status,
            ...hist,
          },
          previewSummary: null,
          meshData,
          loading: false,
        });
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    undo: async () => {
      const { selectedRegionId } = get();
      set({ loading: true, error: null });
      try {
        const hist = await client.undo();
        const meshData = selectedRegionId
          ? await client.getRegionMesh(selectedRegionId)
          : null;
        set({
          status: {
            ...get().status,
            ...hist,
          },
          meshData,
          loading: false,
        });
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    redo: async () => {
      const { selectedRegionId } = get();
      set({ loading: true, error: null });
      try {
        const hist = await client.redo();
        const meshData = selectedRegionId
          ? await client.getRegionMesh(selectedRegionId)
          : null;
        set({
          status: {
            ...get().status,
            ...hist,
          },
          meshData,
          loading: false,
        });
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    exportFile: async (defaultFilename?: string) => {
      set({ loading: true, error: null });
      try {
        const bytes = await client.exportLitematic();
        const blob = new Blob([bytes.buffer as ArrayBuffer], {
          type: 'application/octet-stream',
        });
        set({ exportedBlob: blob, loading: false });

        if (typeof window !== 'undefined' && window.document) {
          const docName = get().document?.name || 'schematic';
          const filename = defaultFilename || `${docName}-edited.litematic`;
          const url = URL.createObjectURL(blob);
          const a = window.document.createElement('a');
          a.href = url;
          a.download = filename;
          window.document.body.appendChild(a);
          a.click();
          window.document.body.removeChild(a);
          URL.revokeObjectURL(url);
        }

        return bytes;
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
        return null;
      }
    },
  };
});
