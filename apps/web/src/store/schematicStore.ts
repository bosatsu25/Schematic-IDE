import { create } from 'zustand';
import {
  DocumentSummary,
  RegionMeshData,
  PreviewSummary,
  SessionStatus,
  SelectionBounds,
  BlockInspection,
  DocumentInspection,
  Diagnostic,
  DocumentDiff,
  BlockDiff,
  AnalysisReport,
} from '../engine/schematicEngine';
import { SchematicWorkerClient } from '../worker/workerClient';

export type SidebarTab = 'regions' | 'inspector' | 'problems' | 'diff' | 'analysis';
export type ExportFormat = 'litematic' | 'sponge' | 'structure';

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
  exportFormat: ExportFormat;
  activeTab: SidebarTab;
  inspection: BlockInspection | null;
  docInspection: DocumentInspection | null;
  diagnostics: Diagnostic[];
  clipboardCount: number | null;
  currentDiff: DocumentDiff | null;
  diffMode: boolean;
  selectedDiff: BlockDiff | null;
  analysisReport: AnalysisReport | null;

  init: () => Promise<void>;
  loadFile: (file: File) => Promise<void>;
  loadBuffer: (buffer: ArrayBuffer, name?: string) => Promise<void>;
  selectRegion: (regionId: string) => Promise<void>;
  setSelection: (bounds: SelectionBounds | null) => void;
  selectAllRegion: () => void;
  setActiveTab: (tab: SidebarTab) => void;
  inspectBlock: (x: number, y: number, z: number, regionId?: string) => Promise<void>;
  inspectDocument: () => Promise<void>;
  validateDocument: () => Promise<void>;
  selectDiagnosticPosition: (diag: Diagnostic) => Promise<void>;
  runDiffWithSource: () => Promise<void>;
  runDiffWithFile: (file: File) => Promise<void>;
  toggleDiffMode: () => void;
  selectDiff: (diff: BlockDiff) => void;
  previewReplace: (fromBlock: string, toBlock: string) => Promise<void>;
  previewFill: (block: string) => Promise<void>;
  copySelection: () => Promise<number | null>;
  previewPaste: (target: [number, number, number]) => Promise<void>;
  previewMove: (delta: [number, number, number]) => Promise<void>;
  previewRotate: (angleDeg: number) => Promise<void>;
  previewMirror: (axis: 'x' | 'z' | string) => Promise<void>;
  previewCleanup: (
    maxSize: number,
    replacement: string,
    protectedKinds: string[],
  ) => Promise<void>;
  commitPreview: () => Promise<void>;
  cancelPreview: () => Promise<void>;
  undo: () => Promise<void>;
  redo: () => Promise<void>;
  setExportFormat: (format: ExportFormat) => void;
  exportFile: (defaultFilename?: string, format?: ExportFormat) => Promise<Uint8Array | null>;
  runAnalysis: (regionId?: string) => Promise<void>;
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
    exportFormat: 'litematic',
    activeTab: 'regions',
    inspection: null,
    docInspection: null,
    diagnostics: [],
    clipboardCount: null,
    currentDiff: null,
    diffMode: false,
    selectedDiff: null,
    analysisReport: null,

    init: async () => {
      try {
        await client.init();
      } catch (err: unknown) {
        console.error('Failed to initialize worker:', err);
      }
    },

    setExportFormat: (format: ExportFormat) => {
      set({ exportFormat: format });
    },

    setActiveTab: (tab: SidebarTab) => {
      set({ activeTab: tab });
      if (tab === 'analysis' && !get().analysisReport) {
        get().runAnalysis();
      }
    },

    runAnalysis: async (regionId?: string) => {
      const regId = regionId !== undefined ? regionId : get().selectedRegionId || undefined;
      try {
        const report = await client.analyzeDocument(regId);
        set({ analysisReport: report });
      } catch (err: unknown) {
        console.error('Analysis failed:', err);
      }
    },

    inspectBlock: async (x: number, y: number, z: number, regionId?: string) => {
      const regId = regionId || get().selectedRegionId;
      if (!regId) return;
      try {
        const inspection = await client.inspectBlock({
          region_id: regId,
          x,
          y,
          z,
        });
        set({ inspection });
      } catch (err: unknown) {
        console.error('Failed to inspect block:', err);
      }
    },

    inspectDocument: async () => {
      try {
        const docInspection = await client.inspectDocument();
        set({ docInspection });
      } catch (err: unknown) {
        console.error('Failed to inspect document:', err);
      }
    },

    validateDocument: async () => {
      try {
        const diagnostics = await client.validateDocument();
        set({ diagnostics });
      } catch (err: unknown) {
        console.error('Failed to validate document:', err);
      }
    },

    selectDiagnosticPosition: async (diag: Diagnostic) => {
      if (diag.region && diag.region !== get().selectedRegionId) {
        await get().selectRegion(diag.region);
      }
      if (diag.position) {
        const [x, y, z] = diag.position;
        get().setSelection({
          min: [x, y, z],
          max: [x, y, z],
        });
        const reg = get().document?.regions.find((r) => r.name === get().selectedRegionId);
        const localX = reg ? x - reg.origin[0] : x;
        const localY = reg ? y - reg.origin[1] : y;
        const localZ = reg ? z - reg.origin[2] : z;
        await get().inspectBlock(localX, localY, localZ);
      }
      set({ activeTab: 'inspector' });
    },

    runDiffWithSource: async () => {
      set({ loading: true, error: null });
      try {
        const diff = await client.diffWithSource();
        set({ currentDiff: diff, diffMode: true, loading: false, activeTab: 'diff' });
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    runDiffWithFile: async (file: File) => {
      set({ loading: true, error: null });
      try {
        const buffer = await file.arrayBuffer();
        const diff = await client.diffWithLitematic(buffer);
        set({ currentDiff: diff, diffMode: true, loading: false, activeTab: 'diff' });
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    toggleDiffMode: () => {
      set((state) => ({ diffMode: !state.diffMode }));
    },

    selectDiff: (diff: BlockDiff) => {
      set({
        selectedDiff: diff,
        selection: {
          min: [diff.position[0], diff.position[1], diff.position[2]],
          max: [diff.position[0], diff.position[1], diff.position[2]],
        },
      });
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
          inspection: null,
        });

        if (initialRegion) {
          await get().selectRegion(initialRegion);
        }

        await get().inspectDocument();
        await get().validateDocument();
        await get().runAnalysis();

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
        if (get().analysisReport) {
          await get().runAnalysis(regionId);
        }
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

    previewFill: async (block: string) => {
      const { selectedRegionId, selection, meshData } = get();
      if (!selectedRegionId || !meshData) return;
      const sel = selection || {
        min: meshData.origin,
        max: [
          meshData.origin[0] + (meshData.size[0] > 0 ? meshData.size[0] - 1 : 0),
          meshData.origin[1] + (meshData.size[1] > 0 ? meshData.size[1] - 1 : 0),
          meshData.origin[2] + (meshData.size[2] > 0 ? meshData.size[2] - 1 : 0),
        ],
      };

      set({ loading: true, error: null });
      try {
        const prev = await client.previewFill({
          region_id: selectedRegionId,
          selection: sel,
          block,
        });
        const status = await client.getStatus();
        const newMeshData = await client.getRegionMesh(selectedRegionId);
        set({
          previewSummary: prev,
          status,
          meshData: newMeshData,
          loading: false,
        });
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    copySelection: async () => {
      const { selectedRegionId, selection, meshData } = get();
      if (!selectedRegionId || !meshData) return null;
      const sel = selection || {
        min: meshData.origin,
        max: [
          meshData.origin[0] + (meshData.size[0] > 0 ? meshData.size[0] - 1 : 0),
          meshData.origin[1] + (meshData.size[1] > 0 ? meshData.size[1] - 1 : 0),
          meshData.origin[2] + (meshData.size[2] > 0 ? meshData.size[2] - 1 : 0),
        ],
      };

      set({ loading: true, error: null });
      try {
        const count = await client.copySelection({
          region_id: selectedRegionId,
          selection: sel,
        });
        set({ clipboardCount: count, loading: false });
        return count;
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
        return null;
      }
    },

    previewPaste: async (target: [number, number, number]) => {
      const { selectedRegionId } = get();
      if (!selectedRegionId) return;

      set({ loading: true, error: null });
      try {
        const prev = await client.previewPaste({
          region_id: selectedRegionId,
          target,
        });
        const status = await client.getStatus();
        const newMeshData = await client.getRegionMesh(selectedRegionId);
        set({
          previewSummary: prev,
          status,
          meshData: newMeshData,
          loading: false,
        });
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    previewMove: async (delta: [number, number, number]) => {
      const { selectedRegionId, selection, meshData } = get();
      if (!selectedRegionId || !meshData) return;
      const sel = selection || {
        min: meshData.origin,
        max: [
          meshData.origin[0] + (meshData.size[0] > 0 ? meshData.size[0] - 1 : 0),
          meshData.origin[1] + (meshData.size[1] > 0 ? meshData.size[1] - 1 : 0),
          meshData.origin[2] + (meshData.size[2] > 0 ? meshData.size[2] - 1 : 0),
        ],
      };

      set({ loading: true, error: null });
      try {
        const prev = await client.previewMove({
          region_id: selectedRegionId,
          selection: sel,
          delta,
        });
        const status = await client.getStatus();
        const newMeshData = await client.getRegionMesh(selectedRegionId);
        set({
          previewSummary: prev,
          status,
          meshData: newMeshData,
          loading: false,
        });
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    previewRotate: async (angleDeg: number) => {
      const { selectedRegionId, selection, meshData } = get();
      if (!selectedRegionId || !meshData) return;
      const sel = selection || {
        min: meshData.origin,
        max: [
          meshData.origin[0] + (meshData.size[0] > 0 ? meshData.size[0] - 1 : 0),
          meshData.origin[1] + (meshData.size[1] > 0 ? meshData.size[1] - 1 : 0),
          meshData.origin[2] + (meshData.size[2] > 0 ? meshData.size[2] - 1 : 0),
        ],
      };

      set({ loading: true, error: null });
      try {
        const prev = await client.previewRotate({
          region_id: selectedRegionId,
          selection: sel,
          angle_deg: angleDeg,
        });
        const status = await client.getStatus();
        const newMeshData = await client.getRegionMesh(selectedRegionId);
        set({
          previewSummary: prev,
          status,
          meshData: newMeshData,
          loading: false,
        });
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    previewMirror: async (axis: 'x' | 'z' | string) => {
      const { selectedRegionId, selection, meshData } = get();
      if (!selectedRegionId || !meshData) return;
      const sel = selection || {
        min: meshData.origin,
        max: [
          meshData.origin[0] + (meshData.size[0] > 0 ? meshData.size[0] - 1 : 0),
          meshData.origin[1] + (meshData.size[1] > 0 ? meshData.size[1] - 1 : 0),
          meshData.origin[2] + (meshData.size[2] > 0 ? meshData.size[2] - 1 : 0),
        ],
      };

      set({ loading: true, error: null });
      try {
        const prev = await client.previewMirror({
          region_id: selectedRegionId,
          selection: sel,
          axis,
        });
        const status = await client.getStatus();
        const newMeshData = await client.getRegionMesh(selectedRegionId);
        set({
          previewSummary: prev,
          status,
          meshData: newMeshData,
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
        await get().inspectDocument();
        await get().validateDocument();
        if (get().analysisReport) {
          await get().runAnalysis();
        }
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
        await get().inspectDocument();
        await get().validateDocument();
        if (get().analysisReport) {
          await get().runAnalysis();
        }
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
        await get().inspectDocument();
        await get().validateDocument();
        if (get().analysisReport) {
          await get().runAnalysis();
        }
      } catch (err: unknown) {
        set({
          error: err instanceof Error ? err.message : String(err),
          loading: false,
        });
      }
    },

    exportFile: async (defaultFilename?: string, format?: ExportFormat) => {
      const activeFormat = format || get().exportFormat || 'litematic';
      set({ loading: true, error: null });
      try {
        let bytes: Uint8Array;
        let ext = '.litematic';
        if (activeFormat === 'sponge') {
          bytes = await client.exportSponge();
          ext = '.schem';
        } else if (activeFormat === 'structure') {
          bytes = await client.exportStructure();
          ext = '.nbt';
        } else {
          bytes = await client.exportLitematic();
          ext = '.litematic';
        }

        const blob = new Blob([bytes.buffer as ArrayBuffer], {
          type: 'application/octet-stream',
        });
        set({ exportedBlob: blob, loading: false });

        if (typeof window !== 'undefined' && window.document) {
          const docName = get().document?.name || 'schematic';
          const filename = defaultFilename || `${docName}-edited${ext}`;
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
