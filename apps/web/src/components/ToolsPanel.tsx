import React, { useState } from 'react';
import { useSchematicStore } from '../store/schematicStore';

export const ToolsPanel: React.FC = () => {
  const {
    document,
    status,
    previewSummary,
    clipboardCount,
    previewReplace,
    previewFill,
    copySelection,
    previewPaste,
    previewMove,
    previewRotate,
    previewMirror,
    previewCleanup,
    commitPreview,
    cancelPreview,
    loading,
    error,
  } = useSchematicStore();

  // Replace form state
  const [fromBlock, setFromBlock] = useState('minecraft:stone');
  const [toBlock, setToBlock] = useState('minecraft:granite');

  // Fill form state
  const [fillBlock, setFillBlock] = useState('minecraft:stone');

  // Paste form state
  const [pasteX, setPasteX] = useState(0);
  const [pasteY, setPasteY] = useState(0);
  const [pasteZ, setPasteZ] = useState(0);

  // Move form state
  const [moveDx, setMoveDx] = useState(1);
  const [moveDy, setMoveDy] = useState(0);
  const [moveDz, setMoveDz] = useState(0);

  // Rotate form state
  const [rotateAngle, setRotateAngle] = useState(90);

  // Mirror form state
  const [mirrorAxis, setMirrorAxis] = useState<'x' | 'z'>('x');

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

  const handleFill = (e: React.FormEvent) => {
    e.preventDefault();
    if (!fillBlock.trim()) return;
    previewFill(fillBlock.trim());
  };

  const handleCopy = () => {
    copySelection();
  };

  const handlePaste = (e: React.FormEvent) => {
    e.preventDefault();
    previewPaste([pasteX, pasteY, pasteZ]);
  };

  const handleMove = (e: React.FormEvent) => {
    e.preventDefault();
    previewMove([moveDx, moveDy, moveDz]);
  };

  const handleRotate = (e: React.FormEvent) => {
    e.preventDefault();
    previewRotate(rotateAngle);
  };

  const handleMirror = (e: React.FormEvent) => {
    e.preventDefault();
    previewMirror(mirrorAxis);
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

      {/* Structural Edit: Fill */}
      <div className="panel">
        <h3>Fill Selection</h3>
        <form onSubmit={handleFill}>
          <div className="form-group">
            <label htmlFor="fill-block">Block State:</label>
            <input
              id="fill-block"
              type="text"
              className="text-input"
              data-testid="fill-block-input"
              value={fillBlock}
              onChange={(e) => setFillBlock(e.target.value)}
              placeholder="e.g. minecraft:stone"
              required
            />
          </div>
          <button
            type="submit"
            className="btn btn-block"
            data-testid="preview-fill-btn"
            disabled={loading || status.has_preview}
          >
            Preview Fill
          </button>
        </form>
      </div>

      {/* Structural Edit: Clipboard (Copy / Paste) */}
      <div className="panel">
        <h3>Clipboard & Placement</h3>
        <div className="form-group">
          <button
            type="button"
            className="btn btn-block"
            data-testid="copy-selection-btn"
            onClick={handleCopy}
            disabled={loading}
          >
            {clipboardCount !== null ? `Copy Selection (${clipboardCount} blocks copied)` : 'Copy Selection'}
          </button>
        </div>
        <form onSubmit={handlePaste}>
          <div className="form-group">
            <label>Paste Target (X, Y, Z):</label>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
              <input
                type="number"
                className="text-input"
                data-testid="paste-target-x"
                value={pasteX}
                onChange={(e) => setPasteX(parseInt(e.target.value, 10) || 0)}
              />
              <input
                type="number"
                className="text-input"
                data-testid="paste-target-y"
                value={pasteY}
                onChange={(e) => setPasteY(parseInt(e.target.value, 10) || 0)}
              />
              <input
                type="number"
                className="text-input"
                data-testid="paste-target-z"
                value={pasteZ}
                onChange={(e) => setPasteZ(parseInt(e.target.value, 10) || 0)}
              />
            </div>
          </div>
          <button
            type="submit"
            className="btn btn-block"
            data-testid="preview-paste-btn"
            disabled={loading || status.has_preview || clipboardCount === null}
          >
            Preview Paste
          </button>
        </form>
      </div>

      {/* Structural Edit: Move, Rotate, Mirror */}
      <div className="panel">
        <h3>Transform Selection</h3>
        <form onSubmit={handleMove} style={{ marginBottom: '1rem' }}>
          <div className="form-group">
            <label>Move Delta (dX, dY, dZ):</label>
            <div style={{ display: 'flex', gap: '0.5rem' }}>
              <input
                type="number"
                className="text-input"
                data-testid="move-dx"
                value={moveDx}
                onChange={(e) => setMoveDx(parseInt(e.target.value, 10) || 0)}
              />
              <input
                type="number"
                className="text-input"
                data-testid="move-dy"
                value={moveDy}
                onChange={(e) => setMoveDy(parseInt(e.target.value, 10) || 0)}
              />
              <input
                type="number"
                className="text-input"
                data-testid="move-dz"
                value={moveDz}
                onChange={(e) => setMoveDz(parseInt(e.target.value, 10) || 0)}
              />
            </div>
          </div>
          <button
            type="submit"
            className="btn btn-block"
            data-testid="preview-move-btn"
            disabled={loading || status.has_preview}
          >
            Preview Move
          </button>
        </form>

        <form onSubmit={handleRotate} style={{ marginBottom: '1rem' }}>
          <div className="form-group">
            <label htmlFor="rotate-angle">Rotate Angle:</label>
            <select
              id="rotate-angle"
              className="text-input"
              data-testid="rotate-angle-select"
              value={rotateAngle}
              onChange={(e) => setRotateAngle(parseInt(e.target.value, 10) || 90)}
            >
              <option value={90}>90° Clockwise</option>
              <option value={180}>180°</option>
              <option value={270}>270° (90° Counter-Clockwise)</option>
            </select>
          </div>
          <button
            type="submit"
            className="btn btn-block"
            data-testid="preview-rotate-btn"
            disabled={loading || status.has_preview}
          >
            Preview Rotate
          </button>
        </form>

        <form onSubmit={handleMirror}>
          <div className="form-group">
            <label htmlFor="mirror-axis">Mirror Axis:</label>
            <select
              id="mirror-axis"
              className="text-input"
              data-testid="mirror-axis-select"
              value={mirrorAxis}
              onChange={(e) => setMirrorAxis(e.target.value as 'x' | 'z')}
            >
              <option value="x">X Axis (East-West flip)</option>
              <option value="z">Z Axis (North-South flip)</option>
            </select>
          </div>
          <button
            type="submit"
            className="btn btn-block"
            data-testid="preview-mirror-btn"
            disabled={loading || status.has_preview}
          >
            Preview Mirror
          </button>
        </form>
      </div>

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
