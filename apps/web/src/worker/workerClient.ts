import {
  DocumentSummary,
  RegionMeshData,
  ReplaceRequest,
  FillRequest,
  CopyRequest,
  PasteRequest,
  MoveRequest,
  RotateRequest,
  MirrorRequest,
  CleanupRequest,
  PreviewSummary,
  HistorySummary,
  SessionStatus,
  BlockInspectionRequest,
  BlockInspection,
  DocumentInspection,
  Diagnostic,
  DocumentDiff,
  AnalysisReport,
} from '../engine/schematicEngine';
import { WorkerMessage, WorkerResponse } from './schematic.worker';

type DistributiveOmit<T, K extends keyof any> = T extends any
  ? Omit<T, K>
  : never;

export type WorkerPayload = DistributiveOmit<WorkerMessage, 'id'>;

export class SchematicWorkerClient {
  private worker: Worker;
  private pending = new Map<
    string,
    { resolve: (data: any) => void; reject: (err: Error) => void }
  >();
  private messageCounter = 0;

  constructor(worker?: Worker) {
    if (worker) {
      this.worker = worker;
    } else if (typeof Worker !== 'undefined') {
      this.worker = new Worker(
        new URL('./schematic.worker.ts', import.meta.url),
        { type: 'module' },
      );
    } else {
      this.worker = {
        postMessage: () => {},
        terminate: () => {},
        onmessage: null,
        onerror: null,
      } as unknown as Worker;
    }

    this.worker.onmessage = (e: MessageEvent<WorkerResponse>) => {
      const { id, ok, data, error } = e.data;
      const handlers = this.pending.get(id);
      if (handlers) {
        this.pending.delete(id);
        if (ok) {
          handlers.resolve(data);
        } else {
          handlers.reject(new Error(error || 'Worker operation failed'));
        }
      }
    };

    this.worker.onerror = (e) => {
      console.error('Worker error:', e);
    };
  }

  async init(wasmUrl?: string): Promise<{ initialized: boolean }> {
    return this.send({ type: 'INIT', wasmUrl });
  }

  async loadLitematic(buffer: ArrayBuffer): Promise<DocumentSummary> {
    return this.send({ type: 'LOAD_LITEMATIC', buffer }, [buffer]);
  }

  async getRegionMesh(regionId: string): Promise<RegionMeshData> {
    return this.send({ type: 'GET_REGION_MESH', regionId });
  }

  async previewReplace(req: ReplaceRequest): Promise<PreviewSummary> {
    return this.send({ type: 'PREVIEW_REPLACE', req });
  }

  async previewFill(req: FillRequest): Promise<PreviewSummary> {
    return this.send({ type: 'PREVIEW_FILL', req });
  }

  async copySelection(req: CopyRequest): Promise<number> {
    return this.send({ type: 'COPY_SELECTION', req });
  }

  async previewPaste(req: PasteRequest): Promise<PreviewSummary> {
    return this.send({ type: 'PREVIEW_PASTE', req });
  }

  async previewMove(req: MoveRequest): Promise<PreviewSummary> {
    return this.send({ type: 'PREVIEW_MOVE', req });
  }

  async previewRotate(req: RotateRequest): Promise<PreviewSummary> {
    return this.send({ type: 'PREVIEW_ROTATE', req });
  }

  async previewMirror(req: MirrorRequest): Promise<PreviewSummary> {
    return this.send({ type: 'PREVIEW_MIRROR', req });
  }

  async previewCleanup(req: CleanupRequest): Promise<PreviewSummary> {
    return this.send({ type: 'PREVIEW_CLEANUP', req });
  }

  async commitPreview(): Promise<HistorySummary> {
    return this.send({ type: 'COMMIT_PREVIEW' });
  }

  async cancelPreview(): Promise<HistorySummary> {
    return this.send({ type: 'CANCEL_PREVIEW' });
  }

  async undo(): Promise<HistorySummary> {
    return this.send({ type: 'UNDO' });
  }

  async redo(): Promise<HistorySummary> {
    return this.send({ type: 'REDO' });
  }

  async exportLitematic(): Promise<Uint8Array> {
    const buffer = await this.send<ArrayBuffer>({ type: 'EXPORT_LITEMATIC' });
    return new Uint8Array(buffer);
  }

  async exportSponge(): Promise<Uint8Array> {
    const buffer = await this.send<ArrayBuffer>({ type: 'EXPORT_SPONGE' });
    return new Uint8Array(buffer);
  }

  async exportStructure(): Promise<Uint8Array> {
    const buffer = await this.send<ArrayBuffer>({ type: 'EXPORT_STRUCTURE' });
    return new Uint8Array(buffer);
  }

  async getStatus(): Promise<SessionStatus> {
    return this.send({ type: 'GET_STATUS' });
  }

  async inspectBlock(req: BlockInspectionRequest): Promise<BlockInspection | null> {
    return this.send({ type: 'INSPECT_BLOCK', req });
  }

  async inspectDocument(): Promise<DocumentInspection> {
    return this.send({ type: 'INSPECT_DOCUMENT' });
  }

  async validateDocument(): Promise<Diagnostic[]> {
    return this.send({ type: 'VALIDATE_DOCUMENT' });
  }

  async diffWithSource(): Promise<DocumentDiff> {
    return this.send({ type: 'DIFF_WITH_SOURCE' });
  }

  async diffWithLitematic(buffer: ArrayBuffer): Promise<DocumentDiff> {
    return this.send({ type: 'DIFF_WITH_LITEMATIC', buffer });
  }

  async analyzeDocument(regionId?: string): Promise<AnalysisReport> {
    return this.send({ type: 'ANALYZE_DOCUMENT', regionId });
  }

  terminate(): void {
    this.worker.terminate();
  }

  private send<T>(
    msg: WorkerPayload,
    transfer?: Transferable[],
  ): Promise<T> {
    return new Promise((resolve, reject) => {
      const id = String(++this.messageCounter);
      this.pending.set(id, { resolve, reject });
      const fullMsg = { ...msg, id } as WorkerMessage;
      if (transfer) {
        this.worker.postMessage(fullMsg, transfer);
      } else {
        this.worker.postMessage(fullMsg);
      }
    });
  }
}
