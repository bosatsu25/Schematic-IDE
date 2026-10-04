import {
  SchematicEngine,
  ReplaceRequest,
  CleanupRequest,
  BlockInspectionRequest,
} from '../engine/schematicEngine';

const engine = new SchematicEngine();
let isInitialized = false;

export type WorkerMessage =
  | { id: string; type: 'INIT'; wasmUrl?: string }
  | { id: string; type: 'LOAD_LITEMATIC'; buffer: ArrayBuffer }
  | { id: string; type: 'GET_REGION_MESH'; regionId: string }
  | { id: string; type: 'PREVIEW_REPLACE'; req: ReplaceRequest }
  | { id: string; type: 'PREVIEW_CLEANUP'; req: CleanupRequest }
  | { id: string; type: 'COMMIT_PREVIEW' }
  | { id: string; type: 'CANCEL_PREVIEW' }
  | { id: string; type: 'UNDO' }
  | { id: string; type: 'REDO' }
  | { id: string; type: 'EXPORT_LITEMATIC' }
  | { id: string; type: 'GET_STATUS' }
  | { id: string; type: 'INSPECT_BLOCK'; req: BlockInspectionRequest }
  | { id: string; type: 'INSPECT_DOCUMENT' }
  | { id: string; type: 'VALIDATE_DOCUMENT' };

export interface WorkerResponse {
  id: string;
  ok: boolean;
  data?: unknown;
  error?: string;
}

self.onmessage = async (e: MessageEvent<WorkerMessage>) => {
  const msg = e.data;
  try {
    switch (msg.type) {
      case 'INIT': {
        if (!isInitialized) {
          await engine.init(msg.wasmUrl);
          isInitialized = true;
        }
        self.postMessage({ id: msg.id, ok: true, data: { initialized: true } });
        break;
      }
      case 'LOAD_LITEMATIC': {
        if (!isInitialized) {
          await engine.init();
          isInitialized = true;
        }
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
    self.postMessage({ id: msg.id, ok: false, error: errorMsg });
  }
};
