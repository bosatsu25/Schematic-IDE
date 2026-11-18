import {
  SchematicEngine,
  ReplaceRequest,
  CleanupRequest,
  BlockInspectionRequest,
  FillRequest,
  CopyRequest,
  PasteRequest,
  MoveRequest,
  RotateRequest,
  MirrorRequest,
} from '../engine/schematicEngine';

const engine = new SchematicEngine();
let wasmUrl: string | undefined;
let initPromise: Promise<void> | null = null;

/** Instantiates the engine once; concurrent callers share the same promise. */
function ensureInit(): Promise<void> {
  if (!initPromise) {
    initPromise = engine.init(wasmUrl).catch((err) => {
      initPromise = null;
      throw err;
    });
  }
  return initPromise;
}

/** A trap inside WASM (panic, out of memory) leaves the instance unusable. */
function isEngineCrash(err: unknown): boolean {
  return typeof WebAssembly !== 'undefined' && err instanceof WebAssembly.RuntimeError;
}

export type WorkerMessage =
  | { id: string; type: 'INIT'; wasmUrl?: string }
  | { id: string; type: 'LOAD_LITEMATIC'; buffer: ArrayBuffer }
  | { id: string; type: 'GET_REGION_MESH'; regionId: string }
  | { id: string; type: 'PREVIEW_REPLACE'; req: ReplaceRequest }
  | { id: string; type: 'PREVIEW_FILL'; req: FillRequest }
  | { id: string; type: 'COPY_SELECTION'; req: CopyRequest }
  | { id: string; type: 'PREVIEW_PASTE'; req: PasteRequest }
  | { id: string; type: 'PREVIEW_MOVE'; req: MoveRequest }
  | { id: string; type: 'PREVIEW_ROTATE'; req: RotateRequest }
  | { id: string; type: 'PREVIEW_MIRROR'; req: MirrorRequest }
  | { id: string; type: 'PREVIEW_CLEANUP'; req: CleanupRequest }
  | { id: string; type: 'COMMIT_PREVIEW' }
  | { id: string; type: 'CANCEL_PREVIEW' }
  | { id: string; type: 'UNDO' }
  | { id: string; type: 'REDO' }
  | { id: string; type: 'EXPORT_LITEMATIC' }
  | { id: string; type: 'EXPORT_SPONGE' }
  | { id: string; type: 'EXPORT_STRUCTURE' }
  | { id: string; type: 'GET_STATUS' }
  | { id: string; type: 'INSPECT_BLOCK'; req: BlockInspectionRequest }
  | { id: string; type: 'INSPECT_DOCUMENT' }
  | { id: string; type: 'VALIDATE_DOCUMENT' }
  | { id: string; type: 'DIFF_WITH_SOURCE' }
  | { id: string; type: 'DIFF_WITH_FILE'; buffer: ArrayBuffer }
  | { id: string; type: 'ANALYZE_DOCUMENT'; regionId?: string }
  | { id: string; type: 'COMMAND'; cmd: string; args?: unknown }
  | { id: string; type: 'REGION_FACES'; region: string; chunks?: [number, number, number][] };

export interface WorkerResponse {
  id: string;
  ok: boolean;
  data?: unknown;
  error?: string;
  /** The engine crashed and was restarted: the open document is gone. */
  engineReset?: boolean;
}

self.onmessage = async (e: MessageEvent<WorkerMessage>) => {
  const msg = e.data;
  try {
    switch (msg.type) {
      case 'INIT': {
        if (msg.wasmUrl) wasmUrl = msg.wasmUrl;
        await ensureInit();
        self.postMessage({ id: msg.id, ok: true, data: { initialized: true } });
        break;
      }
      case 'LOAD_LITEMATIC': {
        await ensureInit();
        const summary = engine.loadLitematic(new Uint8Array(msg.buffer));
        self.postMessage({ id: msg.id, ok: true, data: summary });
        break;
      }
      case 'GET_REGION_MESH': {
        const mesh = engine.getRegionMesh(msg.regionId);
        self.postMessage({ id: msg.id, ok: true, data: mesh });
        break;
      }
      case 'PREVIEW_REPLACE': {
        const res = engine.previewReplace(msg.req);
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'PREVIEW_FILL': {
        const res = engine.previewFill(msg.req);
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'COPY_SELECTION': {
        const count = engine.copySelection(msg.req);
        self.postMessage({ id: msg.id, ok: true, data: count });
        break;
      }
      case 'PREVIEW_PASTE': {
        const res = engine.previewPaste(msg.req);
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'PREVIEW_MOVE': {
        const res = engine.previewMove(msg.req);
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'PREVIEW_ROTATE': {
        const res = engine.previewRotate(msg.req);
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'PREVIEW_MIRROR': {
        const res = engine.previewMirror(msg.req);
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'PREVIEW_CLEANUP': {
        const res = engine.previewCleanup(msg.req);
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'COMMIT_PREVIEW': {
        const res = engine.commitPreview();
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'CANCEL_PREVIEW': {
        const res = engine.cancelPreview();
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'UNDO': {
        const res = engine.undo();
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'REDO': {
        const res = engine.redo();
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'EXPORT_LITEMATIC': {
        const bytes = engine.exportLitematic();
        // Transfer ArrayBuffer for zero-copy
        (self as unknown as Worker).postMessage(
          { id: msg.id, ok: true, data: bytes.buffer },
          [bytes.buffer],
        );
        break;
      }
      case 'EXPORT_SPONGE': {
        const bytes = engine.exportSponge();
        // Transfer ArrayBuffer for zero-copy
        (self as unknown as Worker).postMessage(
          { id: msg.id, ok: true, data: bytes.buffer },
          [bytes.buffer],
        );
        break;
      }
      case 'EXPORT_STRUCTURE': {
        const bytes = engine.exportStructure();
        // Transfer ArrayBuffer for zero-copy
        (self as unknown as Worker).postMessage(
          { id: msg.id, ok: true, data: bytes.buffer },
          [bytes.buffer],
        );
        break;
      }
      case 'GET_STATUS': {
        const status = engine.getStatus();
        self.postMessage({ id: msg.id, ok: true, data: status });
        break;
      }
      case 'INSPECT_BLOCK': {
        const res = engine.inspectBlock(msg.req);
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'INSPECT_DOCUMENT': {
        const res = engine.inspectDocument();
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'VALIDATE_DOCUMENT': {
        const res = engine.validateDocument();
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'DIFF_WITH_SOURCE': {
        const res = engine.diffWithSource();
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'DIFF_WITH_FILE': {
        const res = engine.diffWithFile(new Uint8Array(msg.buffer));
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'ANALYZE_DOCUMENT': {
        const res = engine.analyze(msg.regionId);
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      case 'REGION_FACES': {
        const bytes = engine.regionFaces(msg.region, msg.chunks);
        // Transfer the buffer: no structured-clone copy of large meshes.
        (self as unknown as Worker).postMessage(
          { id: msg.id, ok: true, data: bytes.buffer },
          [bytes.buffer],
        );
        break;
      }
      case 'COMMAND': {
        const res = engine.command(msg.cmd, msg.args ?? {});
        self.postMessage({ id: msg.id, ok: true, data: res });
        break;
      }
      default: {
        self.postMessage({
          id: (msg as { id: string }).id,
          ok: false,
          error: `Unknown action: ${(msg as { type: string }).type}`,
        });
      }
    }
  } catch (err: unknown) {
    const errorMsg = err instanceof Error ? err.message : String(err);
    if (isEngineCrash(err)) {
      // Start a fresh instance so the app keeps working; the session is lost.
      initPromise = null;
      try {
        await ensureInit();
      } catch {
        // Reported on the next call.
      }
      self.postMessage({
        id: msg.id,
        ok: false,
        engineReset: true,
        error: `The engine hit an internal error and was restarted (${errorMsg}). The open document was closed; please reopen the file.`,
      });
      return;
    }
    self.postMessage({ id: msg.id, ok: false, error: errorMsg });
  }
};
