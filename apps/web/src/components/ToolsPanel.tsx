import React, { useState } from 'react';
import { useSchematicStore } from '../store/schematicStore';

export const ToolsPanel: React.FC = () => {
  const {
    document,
    status,
    previewSummary,
    previewReplace,
    previewCleanup,
    commitPreview,
    cancelPreview,
    loading,
    error,
  } = useSchematicStore();

  // Replace form state
  const [fromBlock, setFromBlock] = useState('minecraft:stone');
  const [toBlock, setToBlock] = useState('minecraft:granite');

  // Cleanup form state
  const [maxSize, setMaxSize] = useState(5);
  const [replacementBlock, setReplacementBlock] = useState('minecraft:air');
  const [protectedFeatures, setProtectedFeatures] = useState<Record<string, boolean>>({
    Tip: true,
    ThinFeature: true,
    Edge: false,
    Corner: false,
    Face: false,
  });

  const toggleFeature = (name: string) => {
    setProtectedFeatures((prev) => ({ ...prev, [name]: !prev[name] }));
  };

  const handleReplace = (e: React.FormEvent) => {
    e.preventDefault();
    if (!fromBlock.trim() || !toBlock.trim()) return;
    previewReplace(fromBlock.trim(), toBlock.trim());
  };

  const handleCleanup = (e: React.FormEvent) => {
    e.preventDefault();
    const kinds = Object.entries(protectedFeatures)
      .filter(([, active]) => active)
      .map(([name]) => name);
    previewCleanup(maxSize, replacementBlock.trim(), kinds);
  };

  if (!document) {
    return (
      <aside className="tools-panel" data-testid="tools-panel">
        <div className="panel">
          <h3>Tools</h3>
          <p className="text-muted">Load a schematic to use editing tools.</p>
        </div>
      </aside>
    );
  }

  return (
    <aside className="tools-panel" data-testid="tools-panel">
      {error && (
        <div className="error-banner" data-testid="error-banner">
          {error}
        </div>
      )}

      {/* Active Preview Panel */}
      {status.has_preview && previewSummary && (
        <div className="panel preview-panel" data-testid="preview-panel">
          <div className="preview-header">
            <h3>Active Preview</h3>
            <span className="badge badge-warning">Previewing</span>
          </div>
          <p className="preview-msg" data-testid="preview-message">
            {previewSummary.message}
          </p>
          <div className="preview-stats">
            <span>Blocks Changed:</span>
            <strong data-testid="preview-changed-count">
              {previewSummary.changed_count}
            </strong>
          </div>
          <div className="preview-buttons">
            <button
              className="btn btn-primary"
              data-testid="commit-btn"
              onClick={commitPreview}
              disabled={loading || !previewSummary.can_commit}
            >
              Commit Edit
            </button>
            <button
              className="btn btn-secondary"
              data-testid="cancel-btn"
              onClick={cancelPreview}
              disabled={loading}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Block Replacement Tool */}
      <div className="panel">
        <h3>Block Replacement</h3>
        <form onSubmit={handleReplace}>
          <div className="form-group">
            <label htmlFor="replace-from">From Block:</label>
            <input
              id="replace-from"
              type="text"
              className="text-input"
              data-testid="replace-from-input"
              value={fromBlock}
              onChange={(e) => setFromBlock(e.target.value)}
              placeholder="e.g. minecraft:stone"
              required
            />
          </div>

          <div className="form-group">
            <label htmlFor="replace-to">To Block:</label>
            <input
              id="replace-to"
              type="text"
              className="text-input"
              data-testid="replace-to-input"
              value={toBlock}
              onChange={(e) => setToBlock(e.target.value)}
              placeholder="e.g. minecraft:granite"
              required
            />
          </div>

          <button
            type="submit"
            className="btn btn-block"
            data-testid="preview-replace-btn"
            disabled={loading || status.has_preview}
          >
            Preview Replace
          </button>
        </form>
      </div>

      {/* VoxelWeave Conservative Island Cleanup */}
      <div className="panel">
        <h3>Island Cleanup (VoxelWeave)</h3>
        <form onSubmit={handleCleanup}>
          <div className="form-group">
            <label htmlFor="cleanup-max-size">Max Island Size (blocks):</label>
            <input
              id="cleanup-max-size"
              type="number"
              min="1"
              max="500"
              className="text-input"
              data-testid="cleanup-max-size-input"
              value={maxSize}
              onChange={(e) => setMaxSize(parseInt(e.target.value, 10) || 1)}
              required
            />
          </div>

          <div className="form-group">
            <label htmlFor="cleanup-repl">Replace With:</label>
            <input
              id="cleanup-repl"
              type="text"
              className="text-input"
              data-testid="cleanup-replacement-input"
              value={replacementBlock}
              onChange={(e) => setReplacementBlock(e.target.value)}
              placeholder="minecraft:air"
              required
            />
          </div>

          <div className="form-group">
            <label>Protected Features (Keep):</label>
            <div className="checkbox-group">
              {Object.keys(protectedFeatures).map((feat) => (
                <label key={feat} className="checkbox-label">
                  <input
                    type="checkbox"
                    data-testid={`protect-${feat.toLowerCase()}`}
                    checked={protectedFeatures[feat]}
                    onChange={() => toggleFeature(feat)}
                  />
                  <span>{feat}</span>
                </label>
              ))}
            </div>
          </div>

          <button
            type="submit"
            className="btn btn-block"
            data-testid="preview-cleanup-btn"
            disabled={loading || status.has_preview}
          >
            Preview Cleanup
          </button>
        </form>
      </div>
    </aside>
  );
};
