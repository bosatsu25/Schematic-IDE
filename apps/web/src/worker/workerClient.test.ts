import { describe, expect, it, vi } from 'vitest';
import { SchematicWorkerClient } from './workerClient';
import type { WorkerResponse } from './schematic.worker';

/** A stand-in worker that lets the test answer each posted message. */
function fakeWorker() {
  const posted: { id: string; type: string }[] = [];
  const worker = {
    postMessage: (msg: { id: string; type: string }) => posted.push(msg),
    terminate: () => {},
    onmessage: null as ((e: MessageEvent<WorkerResponse>) => void) | null,
    onerror: null,
  };
  const reply = (response: WorkerResponse) =>
    worker.onmessage?.({ data: response } as MessageEvent<WorkerResponse>);
  return { worker: worker as unknown as Worker, posted, reply };
}

describe('SchematicWorkerClient', () => {
  it('resolves and rejects calls by message id', async () => {
    const { worker, posted, reply } = fakeWorker();
    const client = new SchematicWorkerClient(worker);
    const status = client.getStatus();
    const undo = client.undo();
    reply({ id: posted[1].id, ok: false, error: 'nothing to undo' });
    reply({ id: posted[0].id, ok: true, data: { loaded: true } });
    await expect(status).resolves.toEqual({ loaded: true });
    await expect(undo).rejects.toThrow('nothing to undo');
  });

  it('reports an engine restart so the app can drop the lost document', async () => {
    const { worker, posted, reply } = fakeWorker();
    const client = new SchematicWorkerClient(worker);
    const onReset = vi.fn();
    client.onEngineReset = onReset;
    const call = client.command('region_view', { region: 'Main' });
    reply({
      id: posted[0].id,
      ok: false,
      engineReset: true,
      error: 'The engine hit an internal error and was restarted (unreachable).',
    });
    await expect(call).rejects.toThrow('restarted');
    expect(onReset).toHaveBeenCalledWith(expect.stringContaining('restarted'));
  });
});
