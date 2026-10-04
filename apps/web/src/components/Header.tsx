import React, { useRef } from 'react';
import { useSchematicStore, ExportFormat } from '../store/schematicStore';

export const Header: React.FC = () => {
  const {
    document,
    status,
    undo,
    redo,
    exportFile,
    loadFile,
    loading,
    exportFormat,
    setExportFormat,
  } = useSchematicStore();
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      loadFile(file);
    }
  };

  const getExportLabel = () => {
    switch (exportFormat) {
      case 'sponge':
        return 'Export .schem';
      case 'structure':
        return 'Export .nbt';
      default:
        return 'Export .litematic';
    }
  };

  return (
    <header className="header">
      <div className="header-left">
        <h1 className="app-title">Schematic IDE</h1>
        {document && (
          <div className="doc-meta" data-testid="doc-meta">
            <span className="doc-name" data-testid="doc-name">{document.name || 'Untitled'}</span>
            {document.author && <span className="doc-author">by {document.author}</span>}
            <span className="doc-version">v{document.version} (DV: {document.minecraft_data_version})</span>
          </div>
        )}
      </div>

      <div className="header-actions">
        <input
          type="file"
          accept=".litematic,.schem,.nbt"
          ref={fileInputRef}
          style={{ display: 'none' }}
          data-testid="open-file-input"
          onChange={handleFileChange}
        />
        <button
          className="btn btn-primary"
          data-testid="open-file-btn"
          onClick={() => fileInputRef.current?.click()}
          disabled={loading}
        >
          Open Schematic
        </button>

        <button
          className="btn"
          data-testid="undo-btn"
          onClick={undo}
          disabled={!status.can_undo || loading}
          title="Undo last committed edit"
        >
          Undo
        </button>

        <button
          className="btn"
          data-testid="redo-btn"
          onClick={redo}
          disabled={!status.can_redo || loading}
          title="Redo edit"
        >
          Redo
        </button>

        <select
          className="select-input"
          data-testid="export-format-select"
          value={exportFormat}
          onChange={(e) => setExportFormat(e.target.value as ExportFormat)}
          disabled={!document || loading}
          style={{
            padding: '6px 8px',
            borderRadius: '4px',
            background: 'var(--bg-tertiary, #2a2a2a)',
            color: 'var(--text-main, #fff)',
            border: '1px solid var(--border-color, #444)',
          }}
        >
          <option value="litematic">Litematic (.litematic)</option>
          <option value="sponge">Sponge (.schem)</option>
          <option value="structure">Structure (.nbt)</option>
        </select>

        <button
          className="btn btn-success"
          data-testid="export-btn"
          onClick={() => exportFile()}
          disabled={!document || loading}
        >
          {getExportLabel()}
        </button>
      </div>
    </header>
  );
};
