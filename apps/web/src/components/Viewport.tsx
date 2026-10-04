import React, { useEffect, useRef } from 'react';
import { VoxelRenderer } from '@schematic-ide/renderer';
import { useSchematicStore } from '../store/schematicStore';

export const Viewport: React.FC = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rendererRef = useRef<VoxelRenderer | null>(null);
  const { meshData, selection, document, loading } = useSchematicStore();

  useEffect(() => {
    if (!canvasRef.current) return;
    const renderer = new VoxelRenderer(canvasRef.current);
    rendererRef.current = renderer;

    const handleResize = () => {
      if (canvasRef.current && rendererRef.current) {
        rendererRef.current.resize(
          canvasRef.current.clientWidth,
          canvasRef.current.clientHeight,
        );
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

  // Update rendered voxels when meshData changes
  useEffect(() => {
    if (!rendererRef.current || !meshData) return;

    rendererRef.current.renderRegion({
      origin: meshData.origin,
      size: meshData.size,
      palette: meshData.palette,
      blocks: meshData.blocks,
      preview_diff: meshData.preview_diff,
    });
  }, [meshData]);

  // Update selection box when selection changes
  useEffect(() => {
    if (!rendererRef.current) return;
    rendererRef.current.setSelectionBox(selection);
  }, [selection]);

  return (
    <div className="viewport-container" data-testid="viewport-container">
      <canvas ref={canvasRef} data-testid="viewport-canvas" className="viewport-canvas" />

      {!document && !loading && (
        <div className="viewport-overlay" data-testid="empty-state">
          <div className="empty-card">
            <h3>No Schematic Loaded</h3>
            <p>Click &quot;Open .litematic&quot; to load a Minecraft schematic file.</p>
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
