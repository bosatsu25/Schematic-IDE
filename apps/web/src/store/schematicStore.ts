import { create } from 'zustand';
import {
  DocumentSummary,
  PreviewSummary,
  SessionStatus,
  SelectionBounds,
  BlockInspection,
  DocumentInspection,
  Diagnostic,
  DocumentDiff,
  BlockDiff,
  AnalysisReport,
  ExportReport,
  RegionView,
  EntityInfo,
  BlockEntityInfo,
  EntityListing,
} from '../engine/schematicEngine';
import { SchematicWorkerClient } from '../worker/workerClient';

/** A mesh update for the renderer, produced by `refreshView`. */
export interface RenderUpdate {
  view: RegionView;
  faces: ArrayBuffer;
  /** Rebuild every chunk of the region (otherwise only the chunks included). */
  replaceAll: boolean;
}

/** A block hit in the 3D viewport. */
export interface BlockPick {
  regionId: string;
  world: [number, number, number];
  local: [number, number, number];
}

/** replace: new single-block selection; extend: box from anchor; add: new box. */
export type PickMode = 'replace' | 'extend' | 'add';

export type SidebarTab = 'regions' | 'inspector' | 'problems' | 'diff' | 'analysis' | 'entities';
export type ExportFormat = 'litematic' | 'sponge' | 'structure';

export interface EditorState {
  client: SchematicWorkerClient;
  document: DocumentSummary | null;
  selectedRegionId: string | null;
  /** View (origin, size, palette) of the selected region. */
  meshData: RegionView | null;
  regionViews: Record<string, RegionView>;
  /** Pending renderer updates; the viewport drains them on `renderSeq` change. */
  renderQueue: RenderUpdate[];
  renderSeq: number;
  hiddenRegions: Record<string, boolean>;
  /** Draw regions other than the selected one translucent. */
  dimInactiveRegions: boolean;
  selectionAnchor: [number, number, number] | null;
  entities: EntityInfo[];
  blockEntities: BlockEntityInfo[];
  /** Index into `entities` of the entity shown in the entity panel. */
  selectedEntity: number | null;
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
  /** Conversion-loss report awaiting the user's decision before export. */
  pendingExport: { format: ExportFormat; report: ExportReport } | null;

  init: () => Promise<void>;
  /** Re-fetches meshes: every chunk (`full`) or only chunks the engine marked dirty. */
  refreshView: (full?: boolean) => Promise<void>;
  drainRenderQueue: () => RenderUpdate[];
  toggleRegionVisibility: (regionId: string) => void;
  setDimInactiveRegions: (dim: boolean) => void;
  pickBlock: (pick: BlockPick, mode: PickMode) => Promise<void>;
  loadEntities: () => Promise<void>;
  selectEntity: (index: number | null) => void;
  previewRemoveEntities: (indices: number[]) => Promise<void>;
  previewMoveEntity: (index: number, delta: [number, number, number]) => Promise<void>;
  /** Previews automatic repairs (all `Automatic` diagnostics, or only `codes`). */
  previewFix: (codes?: string[]) => Promise<void>;
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
  /** Checks what the export would drop; exports directly when lossless. */
  requestExport: (format?: ExportFormat) => Promise<void>;
  confirmExport: () => Promise<void>;
  cancelExport: () => void;
  runAnalysis: (regionId?: string) => Promise<void>;
}

export const useSchematicStore = create<EditorState>((set, get) => {
  const client = new SchematicWorkerClient();

  const emptyStatus: SessionStatus = {
    loaded: false,
    has_preview: false,
    can_undo: false,
    can_redo: false,
    is_dirty: false,
  };

  // If the engine crashes it restarts empty: drop every trace of the document
  // so the UI cannot keep editing something that no longer exists.
  client.onEngineReset = (message: string) => {
    set({
      document: null,
      selectedRegionId: null,
      meshData: null,
      regionViews: {},
      renderQueue: [],
      renderSeq: get().renderSeq + 1,
      hiddenRegions: {},
      selection: null,
      selectionAnchor: null,
      status: emptyStatus,
      previewSummary: null,
      inspection: null,
      docInspection: null,
      diagnostics: [],
      entities: [],
      blockEntities: [],
      selectedEntity: null,
      clipboardCount: null,
      currentDiff: null,
      diffMode: false,
      selectedDiff: null,
      analysisReport: null,
      pendingExport: null,
      loading: false,
      error: message,
    });
  };

  /** Runs a preview-producing engine call and refreshes the view. */
  const runPreview = async (call: () => Promise<PreviewSummary>) => {
    set({ loading: true, error: null });
    try {
      const prev = await call();
      const status = await client.getStatus();
      await get().refreshView();
      set({ previewSummary: prev, status, loading: false });
    } catch (err: unknown) {
      set({ error: err instanceof Error ? err.message : String(err), loading: false });
    }
  };

  return {
    client,
    document: null,
    selectedRegionId: null,
    meshData: null,
    regionViews: {},
    renderQueue: [],
    renderSeq: 0,
    hiddenRegions: {},
    dimInactiveRegions: true,
    selectionAnchor: null,
    entities: [],
    blockEntities: [],
    selectedEntity: null,
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
    pendingExport: null,

    init: async () => {
      try {
        // Resolve against the page so the app works when hosted under a
        // sub-path (the worker cannot see the document base URL).
        const wasmUrl =
          typeof document !== 'undefined'
            ? new URL('schematic_wasm.wasm', document.baseURI).href
            : undefined;
        await client.init(wasmUrl);
      } catch (err: unknown) {
        console.error('Failed to initialize worker:', err);
      }
    },

    refreshView: async (full = false) => {
      const doc = get().document;
      if (!doc) return;
      const views = { ...get().regionViews };
      const updates: RenderUpdate[] = [];
      const dirty = await client.command<Record<string, [number, number, number][]>>(
        'take_dirty_chunks',
      );
      const targets: [string, [number, number, number][] | undefined][] = full
        ? doc.regions.map((r) => [r.name, undefined])
        : Object.entries(dirty);
      for (const [region, chunks] of targets) {
        const view = await client.command<RegionView>('region_view', { region });
        const faces = await client.regionFaces(region, chunks);
        views[region] = view;
        updates.push({ view, faces, replaceAll: chunks === undefined });
      }
      const selected = get().selectedRegionId;
      set({
        regionViews: views,
        meshData: selected ? views[selected] ?? null : null,
        renderQueue: [...get().renderQueue, ...updates],
        renderSeq: get().renderSeq + 1,
      });
    },

    loadEntities: async () => {
      try {
        const listing = await client.command<EntityListing>('list_entities');
        const selected = get().selectedEntity;
        set({
          entities: listing.entities,
          blockEntities: listing.block_entities,
          selectedEntity: selected !== null && selected < listing.entities.length ? selected : null,
        });
      } catch (err: unknown) {
        console.error('Failed to list entities:', err);
      }
    },

    selectEntity: (index: number | null) => {
      set({ selectedEntity: index });
    },

    previewRemoveEntities: async (indices: number[]) => {
      await runPreview(() => client.command<PreviewSummary>('preview_remove_entities', { indices }));
    },

    previewMoveEntity: async (index: number, delta: [number, number, number]) => {
      await runPreview(() => client.command<PreviewSummary>('preview_move_entity', { index, delta }));
    },

    previewFix: async (codes?: string[]) => {
      await runPreview(() =>
        client.command<PreviewSummary>('preview_fix', codes ? { codes } : {}),
      );
    },

    drainRenderQueue: () => {
      const queue = get().renderQueue;
      if (queue.length > 0) set({ renderQueue: [] });
      return queue;
    },

    toggleRegionVisibility: (regionId: string) => {
      const hidden = { ...get().hiddenRegions };
      hidden[regionId] = !hidden[regionId];
      set({ hiddenRegions: hidden });
    },

    setDimInactiveRegions: (dim: boolean) => {
      set({ dimInactiveRegions: dim });
    },

    pickBlock: async (pick: BlockPick, mode: PickMode) => {
      if (pick.regionId !== get().selectedRegionId) {
        await get().selectRegion(pick.regionId);
        mode = 'replace';
      }
      const { selection, selectionAnchor } = get();
      const w = pick.world;
      if (mode === 'extend' && selectionAnchor && selection) {
        const a = selectionAnchor;
        set({
          selection: {
            ...selection,
            min: [Math.min(a[0], w[0]), Math.min(a[1], w[1]), Math.min(a[2], w[2])],
            max: [Math.max(a[0], w[0]), Math.max(a[1], w[1]), Math.max(a[2], w[2])],
          },
        });
      } else if (mode === 'add' && selection) {
        set({
          selection: {
            min: [...w],
            max: [...w],
            extra_boxes: [...(selection.extra_boxes ?? []), [selection.min, selection.max]],
          },
          selectionAnchor: w,
        });
      } else {
        set({ selection: { min: [...w], max: [...w] }, selectionAnchor: w });
      }
      await get().inspectBlock(pick.local[0], pick.local[1], pick.local[2], pick.regionId);
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
        await get().loadEntities();
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
        const diff = await client.diffWithFile(buffer);
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
          selectionAnchor: null,
          inspection: null,
          regionViews: {},
          hiddenRegions: {},
          currentDiff: null,
          diffMode: false,
          analysisReport: null,
        });

        await get().refreshView(true);

        await get().inspectDocument();
        await get().validateDocument();
        // Analysis is expensive on large structures (see docs/benchmarks.md):
        // it runs when the Analysis tab is opened, not on every load.
        if (get().activeTab === 'analysis') {
          await get().runAnalysis();
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
      set({
        selectedRegionId: regionId,
        meshData: get().regionViews[regionId] ?? null,
        error: null,
      });
      if (get().analysisReport) {
        await get().runAnalysis(regionId);
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
        await get().refreshView();
        set({
          previewSummary: prev,
          status,
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
        await get().refreshView();
        set({
          previewSummary: prev,
          status,
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
        await get().refreshView();
        set({
          previewSummary: prev,
          status,
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
        await get().refreshView();
        set({
          previewSummary: prev,
          status,
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
        await get().refreshView();
        set({
          previewSummary: prev,
          status,
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
        await get().refreshView();
        set({
          previewSummary: prev,
          status,
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
        await get().refreshView();
        set({
          previewSummary: prev,
          status,
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
        await get().refreshView();
        set({
          status: {
            ...get().status,
            ...hist,
          },
          previewSummary: null,
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
        await get().refreshView();
        set({
          status: {
            ...get().status,
            ...hist,
          },
          previewSummary: null,
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
        await get().refreshView();
        set({
          status: {
            ...get().status,
            ...hist,
          },
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
        await get().refreshView();
        set({
          status: {
            ...get().status,
            ...hist,
          },
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

    requestExport: async (format?: ExportFormat) => {
      const activeFormat = format || get().exportFormat || 'litematic';
      set({ error: null });
      try {
        const report = await client.command<ExportReport>('export_report', {
          target: activeFormat,
        });
        if (report.losses.length === 0) {
          await get().exportFile(undefined, activeFormat);
        } else {
          set({ pendingExport: { format: activeFormat, report } });
        }
      } catch (err: unknown) {
        set({ error: err instanceof Error ? err.message : String(err) });
      }
    },

    confirmExport: async () => {
      const pending = get().pendingExport;
      if (!pending || pending.report.blocked) return;
      set({ pendingExport: null });
      await get().exportFile(undefined, pending.format);
    },

    cancelExport: () => {
      set({ pendingExport: null });
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
