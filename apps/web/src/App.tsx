import React, { useEffect } from 'react';
import { Header } from './components/Header';
import { Sidebar } from './components/Sidebar';
import { Viewport } from './components/Viewport';
import { ToolsPanel } from './components/ToolsPanel';
import { useSchematicStore } from './store/schematicStore';
import './App.css';

export const App: React.FC = () => {
  const { init, status, document, selectedRegionId, meshData } = useSchematicStore();

  useEffect(() => {
    init();
  }, [init]);

  return (
    <div className="app-container" data-testid="app-container">
      <Header />

      <main className="main-workspace">
        <Sidebar />
        <Viewport />
        <ToolsPanel />
      </main>

      <footer className="status-bar" data-testid="status-bar">
        <span>Status: {status.loaded ? 'Ready' : 'No document'}</span>
        {document && <span>Regions: {document.regions.length}</span>}
        {selectedRegionId && <span>Active Region: {selectedRegionId}</span>}
        {meshData && (
          <span>
            Voxels: {meshData.blocks.length / 4}
          </span>
        )}
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
    </div>
  );
};
