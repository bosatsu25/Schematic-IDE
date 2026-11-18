import React, { useEffect, useRef, useState } from 'react';
import { isSchematicFile, SCHEMATIC_EXTENSIONS } from './files';
import { Header } from './components/Header';
import { Sidebar } from './components/Sidebar';
import { Viewport } from './components/Viewport';
import { ToolsPanel } from './components/ToolsPanel';
import { UpdateBanner } from './sw/UpdateBanner';
import { useSchematicStore } from './store/schematicStore';
import './App.css';

export const App: React.FC = () => {
  const { init, status, document, selectedRegionId, docInspection, loadFile } = useSchematicStore();
  const [dragging, setDragging] = useState(false);
  const dragDepth = useRef(0);
  const activeRegion = docInspection?.regions.find((r) => r.name === selectedRegionId);

  useEffect(() => {
    init();
  }, [init]);

  const hasFiles = (e: React.DragEvent) => e.dataTransfer.types.includes('Files');

  const onDrop = (e: React.DragEvent) => {
    if (!hasFiles(e)) return;
    e.preventDefault();
    dragDepth.current = 0;
    setDragging(false);
    const file = e.dataTransfer.files?.[0];
    if (!file) return;
    if (!isSchematicFile(file.name)) {
      useSchematicStore.setState({
        error: `Unsupported file "${file.name}". Supported: ${SCHEMATIC_EXTENSIONS.join(', ')}`,
      });
      return;
    }
    if (
      status.is_dirty &&
      !window.confirm('Discard unsaved changes and open the dropped file?')
    ) {
      return;
    }
    loadFile(file);
  };

  return (
    <div
      className="app-container"
      data-testid="app-container"
      onDragEnter={(e) => {
        if (!hasFiles(e)) return;
        dragDepth.current += 1;
        setDragging(true);
      }}
      onDragOver={(e) => {
        if (hasFiles(e)) e.preventDefault();
      }}
      onDragLeave={(e) => {
        if (!hasFiles(e)) return;
        dragDepth.current = Math.max(0, dragDepth.current - 1);
        if (dragDepth.current === 0) setDragging(false);
      }}
      onDrop={onDrop}
    >
      {dragging && (
        <div className="drop-overlay" data-testid="drop-overlay">
          <div className="drop-card">Drop a .litematic, .schem or .nbt file to open it</div>
        </div>
      )}
      <Header />

      <main className="main-workspace">
        <Sidebar />
        <Viewport />
        <ToolsPanel />
      </main>

      <footer className="status-bar" data-testid="status-bar">
        <span data-testid="app-version">v{__APP_VERSION__}</span>
        <span>Status: {status.loaded ? 'Ready' : 'No document'}</span>
        {document && <span>Regions: {document.regions.length}</span>}
        {selectedRegionId && <span>Active Region: {selectedRegionId}</span>}
        {activeRegion && <span>Blocks: {activeRegion.non_air_blocks}</span>}
        {status.is_dirty && (
          <span className="status-dirty" data-testid="dirty-indicator">
            ● Modified (Unsaved changes)
          </span>
        )}
        {status.has_preview && (
          <span className="status-preview" data-testid="preview-indicator">
            ⚡ Preview Active
          </span>
        )}
      </footer>

      <UpdateBanner />
    </div>
  );
};
