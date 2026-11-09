import React, { useEffect, useRef, useState } from 'react';
import { VoxelRenderer, PickHit } from '@schematic-ide/renderer';
import { useSchematicStore, PickMode } from '../store/schematicStore';

/** Pointer travel (px) below which a press counts as a click, not an orbit drag. */
const CLICK_TOLERANCE = 4;

export const Viewport: React.FC = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rendererRef = useRef<VoxelRenderer | null>(null);
  const pressRef = useRef<{ x: number; y: number } | null>(null);
  const [hover, setHover] = useState<PickHit | null>(null);
  const {
    renderSeq,
    drainRenderQueue,
    selection,
    document,
    loading,
    diffMode,
    currentDiff,
    selectedRegionId,
    hiddenRegions,
    dimInactiveRegions,
    pickBlock,
    entities,
    selectedEntity,
  } = useSchematicStore();

  useEffect(() => {
    if (!canvasRef.current) return;
    const renderer = new VoxelRenderer(canvasRef.current);
    rendererRef.current = renderer;
    if (import.meta.env.DEV) {
      // Test hook for browser E2E (projecting world points to click targets).
      (window as unknown as { __schematicViewport?: VoxelRenderer }).__schematicViewport = renderer;
    }
    const handleResize = () => {
      if (canvasRef.current) {
        renderer.resize(canvasRef.current.clientWidth, canvasRef.current.clientHeight);
      }
    };
    window.addEventListener('resize', handleResize);
    handleResize();
    return () => {
      window.removeEventListener('resize', handleResize);
      renderer.dispose();
      rendererRef.current = null;
    };
  }, []);

  // Apply queued mesh updates (full or per-chunk) from the engine.
  useEffect(() => {
    const renderer = rendererRef.current;
    if (!renderer) return;
    const updates = drainRenderQueue();
    for (const update of updates) {
      renderer.updateRegion(update.view, update.faces, update.replaceAll);
    }
    renderer.retainRegions(document ? document.regions.map((r) => r.name) : []);
    // A full rebuild means a newly opened document: frame all of its regions.
    if (updates.some((u) => u.replaceAll)) renderer.fitToContent();
  }, [renderSeq, document, drainRenderQueue]);

  // Region visibility and active-region emphasis.
  useEffect(() => {
    const renderer = rendererRef.current;
    if (!renderer || !document) return;
    const multi = document.regions.length > 1;
    for (const r of document.regions) {
      renderer.setRegionStyle(r.name, {
        visible: !hiddenRegions[r.name],
        dimmed: multi && dimInactiveRegions && r.name !== selectedRegionId,
      });
    }
  }, [document, hiddenRegions, dimInactiveRegions, selectedRegionId, renderSeq]);

  useEffect(() => {
    const renderer = rendererRef.current;
    if (!renderer) return;
    if (!selection) {
      renderer.setSelectionBoxes(null);
      return;
    }
    renderer.setSelectionBoxes([
      { min: selection.min, max: selection.max },
      ...(selection.extra_boxes ?? []).map(([min, max]) => ({ min, max })),
    ]);
  }, [selection]);

  useEffect(() => {
    rendererRef.current?.setHover(hover ? hover.world : null);
  }, [hover]);

  useEffect(() => {
    const renderer = rendererRef.current;
    if (!renderer) return;
    const origins = Object.fromEntries((document?.regions ?? []).map((r) => [r.name, r.origin]));
    renderer.setEntityMarkers(
      entities
        .filter((e) => !hiddenRegions[e.region])
        .map((e, i) => {
          const o = origins[e.region] ?? [0, 0, 0];
          return {
            position: [o[0] + e.position[0], o[1] + e.position[1], o[2] + e.position[2]],
            kind: e.kind,
            selected: selectedEntity === i,
          };
        }),
    );
  }, [entities, selectedEntity, document, hiddenRegions]);

  useEffect(() => {
    const renderer = rendererRef.current;
    if (!renderer) return;
    if (!diffMode || !currentDiff) {
      renderer.setDiffOverlay(null);
      return;
    }
    const added: [number, number, number][] = [];
    const removed: [number, number, number][] = [];
    const modified: [number, number, number][] = [];
    for (const b of currentDiff.block_diffs) {
      if (selectedRegionId && b.region_id !== selectedRegionId) continue;
      if (b.kind === 'Added') added.push(b.position);
      else if (b.kind === 'Removed') removed.push(b.position);
      else if (b.kind === 'Modified') modified.push(b.position);
    }
    renderer.setDiffOverlay({
      added_positions: added,
      removed_positions: removed,
      modified_positions: modified,
    });
  }, [diffMode, currentDiff, selectedRegionId]);

  const onPointerDown = (e: React.PointerEvent) => {
    pressRef.current = { x: e.clientX, y: e.clientY };
  };

  const onPointerMove = (e: React.PointerEvent) => {
    if (e.buttons !== 0) return; // orbiting
    const hit = rendererRef.current?.pickAtClient(e.clientX, e.clientY) ?? null;
    setHover((prev) =>
      prev?.regionId === hit?.regionId && prev?.world.join() === hit?.world.join() ? prev : hit,
    );
  };

  const onPointerUp = (e: React.PointerEvent) => {
    const press = pressRef.current;
    pressRef.current = null;
    if (!press || e.button !== 0) return;
    if (Math.hypot(e.clientX - press.x, e.clientY - press.y) > CLICK_TOLERANCE) return;
    const hit = rendererRef.current?.pickAtClient(e.clientX, e.clientY);
    if (!hit) return;
    const mode: PickMode = e.shiftKey ? 'extend' : e.ctrlKey || e.metaKey ? 'add' : 'replace';
    pickBlock({ regionId: hit.regionId, world: hit.world, local: hit.local }, mode);
  };

  return (
    <div className="viewport-container" data-testid="viewport-container">
      <canvas
        ref={canvasRef}
        data-testid="viewport-canvas"
        className="viewport-canvas"
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerLeave={() => setHover(null)}
      />

      {hover && (
        <div className="viewport-hover" data-testid="viewport-hover">
          {hover.regionId} · ({hover.world.join(', ')})
        </div>
      )}

      {document && (
        <div className="viewport-hint">
          Click: select · Shift+click: extend box · Ctrl/⌘+click: add box · Drag: orbit · Right-drag: pan · Wheel: zoom
        </div>
      )}

      {!document && !loading && (
        <div className="viewport-overlay" data-testid="empty-state">
          <div className="empty-card">
            <h3>No Schematic Loaded</h3>
            <p>Open a .litematic, .schem or .nbt file, or drop one anywhere on the window.</p>
          </div>
        </div>
      )}

      {loading && (
        <div className="viewport-overlay" data-testid="loading-state">
          <div className="spinner">Processing...</div>
        </div>
      )}
    </div>
  );
};
