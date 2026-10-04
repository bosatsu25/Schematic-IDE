import React, { useState } from 'react';
import { useSchematicStore, SidebarTab } from '../store/schematicStore';

export const Sidebar: React.FC = () => {
  const {
    document,
    selectedRegionId,
    selectRegion,
    meshData,
    selection,
    setSelection,
    selectAllRegion,
    activeTab,
    setActiveTab,
    inspection,
    docInspection,
    diagnostics,
    inspectBlock,
    validateDocument,
    selectDiagnosticPosition,
    currentDiff,
    diffMode,
    runDiffWithSource,
    runDiffWithFile,
    toggleDiffMode,
    selectDiff,
    analysisReport,
    runAnalysis,
  } = useSchematicStore();

  const [inspectCoords, setInspectCoords] = useState<{ x: number; y: number; z: number }>({
    x: 0,
    y: 0,
    z: 0,
  });
  const [materialSearch, setMaterialSearch] = useState('');
  const [copiedCsv, setCopiedCsv] = useState(false);

  const handleCopyCsv = () => {
    if (!analysisReport) return;
    const header = 'Block,Count,Stacks (64),Remainder,Percentage (%)\n';
    const rows = analysisReport.materials
      .map(
        (m) =>
          `"${m.id.replace(/"/g, '""')}",${m.count},${m.stacks_64},${m.remainder},${m.percentage.toFixed(2)}`,
      )
      .join('\n');
    try {
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(header + rows);
      }
    } catch {
      // ignore clipboard error in unpermitted environments
    }
    setCopiedCsv(true);
    setTimeout(() => setCopiedCsv(false), 2000);
  };

  const filteredMaterials = analysisReport
    ? analysisReport.materials.filter((m) =>
        m.id.toLowerCase().includes(materialSearch.toLowerCase()) ||
        m.block_state.toLowerCase().includes(materialSearch.toLowerCase()),
      )
    : [];

  if (!document) {
    return (
      <aside className="sidebar" data-testid="sidebar">
        <div className="panel">
          <h3>Document</h3>
          <p className="text-muted">No document open</p>
        </div>
      </aside>
    );
  }

  const currentRegion = document.regions.find((r) => r.name === selectedRegionId);

  const handleCoordChange = (
    corner: 'min' | 'max',
    axisIndex: 0 | 1 | 2,
    val: number,
  ) => {
    const cur = selection || {
      min: meshData ? [...meshData.origin] : [0, 0, 0],
      max: meshData
        ? [
            meshData.origin[0] + meshData.size[0] - 1,
            meshData.origin[1] + meshData.size[1] - 1,
            meshData.origin[2] + meshData.size[2] - 1,
          ]
        : [0, 0, 0],
    };

    const updated = {
      min: [...cur.min] as [number, number, number],
      max: [...cur.max] as [number, number, number],
    };
    updated[corner][axisIndex] = isNaN(val) ? 0 : val;
    setSelection(updated);
  };

  const handleInspectSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    inspectBlock(inspectCoords.x, inspectCoords.y, inspectCoords.z);
  };

  const errorCount = diagnostics.filter((d) => d.severity === 'Error').length;
  const warningCount = diagnostics.filter((d) => d.severity === 'Warning').length;

  return (
    <aside className="sidebar" data-testid="sidebar">
      {/* Tab Navigation */}
      <div className="sidebar-tabs" data-testid="sidebar-tabs">
        <button
          className={`sidebar-tab-btn ${activeTab === 'regions' ? 'active' : ''}`}
          data-testid="tab-regions"
          onClick={() => setActiveTab('regions')}
        >
          Regions
        </button>
        <button
          className={`sidebar-tab-btn ${activeTab === 'inspector' ? 'active' : ''}`}
          data-testid="tab-inspector"
          onClick={() => setActiveTab('inspector')}
        >
          Inspector
        </button>
        <button
          className={`sidebar-tab-btn ${activeTab === 'problems' ? 'active' : ''}`}
          data-testid="tab-problems"
          onClick={() => setActiveTab('problems')}
        >
          Problems
          {diagnostics.length > 0 && (
            <span
              className={`tab-badge ${errorCount > 0 ? 'badge-error' : 'badge-warning'}`}
              data-testid="problems-badge"
            >
              {diagnostics.length}
            </span>
          )}
        </button>
        <button
          className={`sidebar-tab-btn ${activeTab === 'diff' ? 'active' : ''}`}
          data-testid="tab-diff"
          onClick={() => setActiveTab('diff')}
        >
          Diff
          {currentDiff && (currentDiff.total_added + currentDiff.total_removed + currentDiff.total_modified > 0) && (
            <span className="tab-badge badge-diff" data-testid="diff-badge">
              {currentDiff.total_added + currentDiff.total_removed + currentDiff.total_modified}
            </span>
          )}
        </button>
        <button
          className={`sidebar-tab-btn ${activeTab === 'analysis' ? 'active' : ''}`}
          data-testid="tab-analysis"
          onClick={() => setActiveTab('analysis')}
        >
          Analysis
        </button>
      </div>

      {/* TAB 1: REGIONS & SELECTION */}
      {activeTab === 'regions' && (
        <>
          <div className="panel">
            <h3>Regions</h3>
            <select
              className="select-input"
              data-testid="region-selector"
              value={selectedRegionId || ''}
              onChange={(e) => selectRegion(e.target.value)}
            >
              {document.regions.map((reg) => (
                <option key={reg.name} value={reg.name}>
                  {reg.name} ({reg.size[0]}×{reg.size[1]}×{reg.size[2]})
                </option>
              ))}
            </select>

            {currentRegion && (
              <div className="region-info" data-testid="region-info">
                <div className="info-row">
                  <span>Origin:</span>
                  <span>
                    ({currentRegion.origin[0]}, {currentRegion.origin[1]},{' '}
                    {currentRegion.origin[2]})
                  </span>
                </div>
                <div className="info-row">
                  <span>Dimensions:</span>
                  <span>
                    {currentRegion.size[0]} × {currentRegion.size[1]} ×{' '}
                    {currentRegion.size[2]}
                  </span>
                </div>
                <div className="info-row">
                  <span>Non-Air Blocks:</span>
                  <span data-testid="region-block-count">{currentRegion.non_air_blocks}</span>
                </div>
                <div className="info-row">
                  <span>Palette Count:</span>
                  <span>{currentRegion.palette.length}</span>
                </div>
              </div>
            )}
          </div>

          <div className="panel">
            <h3>Selection Box</h3>
            <div className="selection-actions">
              <button
                className="btn btn-sm"
                data-testid="select-all-btn"
                onClick={selectAllRegion}
              >
                Select All
              </button>
              <button
                className="btn btn-sm"
                data-testid="clear-selection-btn"
                onClick={() => setSelection(null)}
                disabled={!selection}
              >
                Clear
              </button>
            </div>

            <div className="coord-grid">
              <span className="coord-header">Corner</span>
              <span className="coord-header">X</span>
              <span className="coord-header">Y</span>
              <span className="coord-header">Z</span>

              <span className="coord-label">Min:</span>
              <input
                type="number"
                className="coord-input"
                data-testid="sel-min-x"
                value={selection?.min[0] ?? ''}
                placeholder={meshData ? String(meshData.origin[0]) : '0'}
                onChange={(e) => handleCoordChange('min', 0, parseInt(e.target.value, 10))}
              />
              <input
                type="number"
                className="coord-input"
                data-testid="sel-min-y"
                value={selection?.min[1] ?? ''}
                placeholder={meshData ? String(meshData.origin[1]) : '0'}
                onChange={(e) => handleCoordChange('min', 1, parseInt(e.target.value, 10))}
              />
              <input
                type="number"
                className="coord-input"
                data-testid="sel-min-z"
                value={selection?.min[2] ?? ''}
                placeholder={meshData ? String(meshData.origin[2]) : '0'}
                onChange={(e) => handleCoordChange('min', 2, parseInt(e.target.value, 10))}
              />

              <span className="coord-label">Max:</span>
              <input
                type="number"
                className="coord-input"
                data-testid="sel-max-x"
                value={selection?.max[0] ?? ''}
                placeholder={
                  meshData
                    ? String(meshData.origin[0] + meshData.size[0] - 1)
                    : '15'
                }
                onChange={(e) => handleCoordChange('max', 0, parseInt(e.target.value, 10))}
              />
              <input
                type="number"
                className="coord-input"
                data-testid="sel-max-y"
                value={selection?.max[1] ?? ''}
                placeholder={
                  meshData
                    ? String(meshData.origin[1] + meshData.size[1] - 1)
                    : '15'
                }
                onChange={(e) => handleCoordChange('max', 1, parseInt(e.target.value, 10))}
              />
              <input
                type="number"
                className="coord-input"
                data-testid="sel-max-z"
                value={selection?.max[2] ?? ''}
                placeholder={
                  meshData
                    ? String(meshData.origin[2] + meshData.size[2] - 1)
                    : '15'
                }
                onChange={(e) => handleCoordChange('max', 2, parseInt(e.target.value, 10))}
              />
            </div>
          </div>
        </>
      )}

      {/* TAB 2: INSPECTOR */}
      {activeTab === 'inspector' && (
        <>
          <div className="panel" data-testid="inspector-panel">
            <h3>Block Inspector</h3>
            <form onSubmit={handleInspectSubmit} className="inspect-form">
              <div className="inspect-inputs">
                <label>
                  X:
                  <input
                    type="number"
                    className="coord-input-sm"
                    data-testid="inspect-input-x"
                    value={inspectCoords.x}
                    onChange={(e) =>
                      setInspectCoords({ ...inspectCoords, x: parseInt(e.target.value, 10) || 0 })
                    }
                  />
                </label>
                <label>
                  Y:
                  <input
                    type="number"
                    className="coord-input-sm"
                    data-testid="inspect-input-y"
                    value={inspectCoords.y}
                    onChange={(e) =>
                      setInspectCoords({ ...inspectCoords, y: parseInt(e.target.value, 10) || 0 })
                    }
                  />
                </label>
                <label>
                  Z:
                  <input
                    type="number"
                    className="coord-input-sm"
                    data-testid="inspect-input-z"
                    value={inspectCoords.z}
                    onChange={(e) =>
                      setInspectCoords({ ...inspectCoords, z: parseInt(e.target.value, 10) || 0 })
                    }
                  />
                </label>
              </div>
              <button type="submit" className="btn btn-sm btn-primary" data-testid="inspect-btn">
                Inspect Coordinate
              </button>
            </form>

            {inspection ? (
              <div className="inspection-result" data-testid="block-inspection-details">
                <div className="info-row">
                  <span className="info-label">Block ID:</span>
                  <span className="info-val font-mono" data-testid="inspect-block-id">
                    {inspection.block_id}
                  </span>
                </div>
                <div className="info-row">
                  <span className="info-label">Palette Index:</span>
                  <span className="info-val">#{inspection.palette_index}</span>
                </div>
                <div className="info-row">
                  <span className="info-label">Local Pos:</span>
                  <span className="info-val font-mono">
                    ({inspection.local_position[0]}, {inspection.local_position[1]},{' '}
                    {inspection.local_position[2]})
                  </span>
                </div>
                <div className="info-row">
                  <span className="info-label">World Pos:</span>
                  <span className="info-val font-mono">
                    ({inspection.world_position[0]}, {inspection.world_position[1]},{' '}
                    {inspection.world_position[2]})
                  </span>
                </div>

                {/* BlockState Properties */}
                <div className="property-section">
                  <h4>Properties</h4>
                  {Object.keys(inspection.properties).length === 0 ? (
                    <p className="text-muted text-sm">No state properties</p>
                  ) : (
                    <table className="props-table" data-testid="inspect-properties-table">
                      <thead>
                        <tr>
                          <th>Property</th>
                          <th>Value</th>
                        </tr>
                      </thead>
                      <tbody>
                        {Object.entries(inspection.properties).map(([k, v]) => (
                          <tr key={k}>
                            <td className="prop-name font-mono">{k}</td>
                            <td className="prop-val font-mono">{v}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  )}
                </div>

                {/* Block Entity NBT */}
                {inspection.block_entity ? (
                  <div className="nbt-section" data-testid="inspect-block-entity">
                    <h4>Block Entity NBT</h4>
                    <pre className="nbt-json">
                      {JSON.stringify(inspection.block_entity, null, 2)}
                    </pre>
                  </div>
                ) : null}
              </div>
            ) : (
              <p className="text-muted text-sm mt-2">
                Enter coordinates above to inspect a block state in the active region.
              </p>
            )}
          </div>

          {/* Document & Preserved NBT Inspection */}
          <div className="panel" data-testid="doc-inspection-panel">
            <h3>Document & Metadata</h3>
            {docInspection ? (
              <div className="doc-inspection-details">
                <div className="info-row">
                  <span className="info-label">Name:</span>
                  <span className="info-val">{docInspection.metadata.name || 'Untitled'}</span>
                </div>
                <div className="info-row">
                  <span className="info-label">Author:</span>
                  <span className="info-val">{docInspection.metadata.author || 'Anonymous'}</span>
                </div>
                <div className="info-row">
                  <span className="info-label">DataVersion:</span>
                  <span className="info-val">{docInspection.metadata.minecraft_data_version}</span>
                </div>
                <div className="info-row">
                  <span className="info-label">Block Entities:</span>
                  <span className="info-val">{docInspection.total_block_entities}</span>
                </div>
                <div className="info-row">
                  <span className="info-label">Entities:</span>
                  <span className="info-val">{docInspection.total_entities}</span>
                </div>

                {docInspection.raw_nbt_keys.length > 0 && (
                  <div className="raw-nbt-keys-section">
                    <h4>Preserved NBT Keys</h4>
                    <div className="keys-badges">
                      {docInspection.raw_nbt_keys.map((key) => (
                        <span key={key} className="key-badge">
                          {key}
                        </span>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            ) : (
              <p className="text-muted text-sm">No inspection metadata available.</p>
            )}
          </div>
        </>
      )}

      {/* TAB 3: PROBLEMS PANEL */}
      {activeTab === 'problems' && (
        <div className="panel" data-testid="problems-panel">
          <div className="problems-header">
            <h3>Diagnostics ({diagnostics.length})</h3>
            <button
              className="btn btn-xs"
              data-testid="revalidate-btn"
              onClick={() => validateDocument()}
            >
              Revalidate
            </button>
          </div>

          <div className="problems-summary">
            <span className="summary-pill badge-error">{errorCount} Errors</span>
            <span className="summary-pill badge-warning">{warningCount} Warnings</span>
            <span className="summary-pill badge-info">
              {diagnostics.length - errorCount - warningCount} Info
            </span>
          </div>

          {diagnostics.length === 0 ? (
            <div className="empty-problems" data-testid="no-problems">
              <p className="text-success">✓ No validation issues detected.</p>
            </div>
          ) : (
            <div className="problems-list" data-testid="diagnostics-list">
              {diagnostics.map((d, i) => (
                <div
                  key={`${d.code}-${i}`}
                  className={`problem-item severity-${d.severity.toLowerCase()}`}
                  data-testid={`problem-item-${i}`}
                  onClick={() => selectDiagnosticPosition(d)}
                >
                  <div className="problem-title">
                    <span className={`severity-badge badge-${d.severity.toLowerCase()}`}>
                      {d.severity}
                    </span>
                    <span className="problem-code font-mono">{d.code}</span>
                    <span className="fixability-badge">{d.fixability}</span>
                  </div>
                  <p className="problem-msg">{d.message}</p>
                  {d.position && (
                    <div className="problem-pos font-mono">
                      Pos: ({d.position[0]}, {d.position[1]}, {d.position[2]})
                      {d.region ? ` in ${d.region}` : ''}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* TAB 4: CANONICAL DIFF */}
      {activeTab === 'diff' && (
        <div className="panel" data-testid="diff-panel">
          <div className="panel-header-with-actions">
            <h3>Structural Diff</h3>
            <div className="diff-action-buttons">
              <button
                className="btn btn-sm btn-primary"
                data-testid="diff-source-btn"
                onClick={() => runDiffWithSource()}
              >
                Diff vs Source
              </button>
              <label className="btn btn-sm btn-secondary file-upload-label" style={{ cursor: 'pointer' }}>
                Compare File...
                <input
                  type="file"
                  accept=".litematic"
                  style={{ display: 'none' }}
                  data-testid="diff-file-input"
                  onChange={(e) => {
                    const f = e.target.files?.[0];
                    if (f) runDiffWithFile(f);
                  }}
                />
              </label>
            </div>
          </div>

          {currentDiff ? (
            <div className="diff-content" data-testid="diff-content" style={{ marginTop: '12px' }}>
              <div className="diff-controls-bar">
                <button
                  className={`btn btn-sm ${diffMode ? 'btn-success' : 'btn-outline'}`}
                  data-testid="toggle-diff-overlay"
                  onClick={toggleDiffMode}
                >
                  {diffMode ? '✓ 3D Overlay ON' : '3D Overlay OFF'}
                </button>
              </div>

              <div className="diff-stat-grid" data-testid="diff-stats">
                <div className="stat-card stat-added">
                  <span className="stat-value font-mono">+{currentDiff.total_added}</span>
                  <span className="stat-label">Added</span>
                </div>
                <div className="stat-card stat-removed">
                  <span className="stat-value font-mono">-{currentDiff.total_removed}</span>
                  <span className="stat-label">Removed</span>
                </div>
                <div className="stat-card stat-modified">
                  <span className="stat-value font-mono">~{currentDiff.total_modified}</span>
                  <span className="stat-label">Modified</span>
                </div>
                <div className="stat-card stat-unchanged">
                  <span className="stat-value font-mono">{currentDiff.total_unchanged}</span>
                  <span className="stat-label">Unchanged</span>
                </div>
              </div>

              {currentDiff.block_diffs.length === 0 ? (
                <div className="empty-diff text-muted" data-testid="empty-diff">
                  No structural differences detected.
                </div>
              ) : (
                <div className="diff-list" data-testid="diff-list">
                  <h4 style={{ margin: '8px 0' }}>Changed Blocks ({currentDiff.block_diffs.length})</h4>
                  <div className="diff-items-container">
                    {currentDiff.block_diffs.map((b, idx) => (
                      <div
                        key={`${b.region_id}-${b.position.join(',')}-${idx}`}
                        className={`diff-item diff-kind-${b.kind.toLowerCase()}`}
                        data-testid={`diff-item-${idx}`}
                        onClick={() => selectDiff(b)}
                      >
                        <div className="diff-item-header">
                          <span className={`diff-kind-badge badge-${b.kind.toLowerCase()}`}>
                            {b.kind}
                          </span>
                          <span className="font-mono text-sm">
                            ({b.position[0]}, {b.position[1]}, {b.position[2]})
                          </span>
                        </div>
                        <div className="diff-item-details text-xs">
                          {b.kind === 'Added' && (
                            <span className="diff-after text-success">
                              + {b.after?.name}
                            </span>
                          )}
                          {b.kind === 'Removed' && (
                            <span className="diff-before text-error">
                              - {b.before?.name}
                            </span>
                          )}
                          {b.kind === 'Modified' && (
                            <span className="diff-mod">
                              <span className="text-error">{b.before?.name}</span>
                              {' → '}
                              <span className="text-warning">{b.after?.name}</span>
                            </span>
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          ) : (
            <div className="empty-diff text-muted" data-testid="no-diff-run" style={{ marginTop: '12px' }}>
              <p>Compare against the initial source document or load another .litematic to inspect differences.</p>
            </div>
          )}
        </div>
      )}

      {/* TAB 5: ANALYSIS */}
      {activeTab === 'analysis' && (
        <div className="panel" data-testid="analysis-panel">
          <div
            className="analysis-header"
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              marginBottom: '12px',
            }}
          >
            <h3 style={{ margin: 0 }}>Structure Analysis</h3>
            <button
              className="btn btn-sm btn-outline"
              data-testid="refresh-analysis-btn"
              onClick={() => runAnalysis()}
            >
              Refresh
            </button>
          </div>

          {analysisReport ? (
            <div className="analysis-content" data-testid="analysis-content">
              {/* Structural Statistics Cards */}
              <div className="analysis-stats-section">
                <h4
                  style={{
                    margin: '8px 0',
                    fontSize: '0.8rem',
                    color: 'var(--text-muted)',
                    textTransform: 'uppercase',
                  }}
                >
                  Metrics & Geometry
                </h4>
                <div className="diff-stat-grid" data-testid="analysis-metrics-grid">
                  <div className="stat-card" data-testid="metric-dimensions">
                    <span
                      className="stat-value font-mono"
                      style={{ fontSize: '0.75rem' }}
                    >
                      {analysisReport.statistics.dimensions.join('×')}
                    </span>
                    <span className="stat-label">Size (X×Y×Z)</span>
                  </div>
                  <div className="stat-card" data-testid="metric-volume">
                    <span className="stat-value font-mono">
                      {analysisReport.statistics.total_volume.toLocaleString()}
                    </span>
                    <span className="stat-label">Volume</span>
                  </div>
                  <div className="stat-card" data-testid="metric-non-air">
                    <span className="stat-value font-mono text-success">
                      {analysisReport.statistics.non_air_blocks.toLocaleString()}
                    </span>
                    <span className="stat-label">Non-Air</span>
                  </div>
                  <div className="stat-card" data-testid="metric-density">
                    <span className="stat-value font-mono">
                      {analysisReport.statistics.fill_density.toFixed(1)}%
                    </span>
                    <span className="stat-label">Fill Density</span>
                  </div>
                </div>

                <div className="diff-stat-grid" style={{ marginTop: '6px' }}>
                  <div className="stat-card" data-testid="metric-surface">
                    <span className="stat-value font-mono">
                      {analysisReport.statistics.surface_cell_count.toLocaleString()}
                    </span>
                    <span className="stat-label">Surface Voxels</span>
                  </div>
                  <div className="stat-card" data-testid="metric-islands">
                    <span className="stat-value font-mono">
                      {analysisReport.statistics.island_count}
                    </span>
                    <span className="stat-label">Islands</span>
                  </div>
                  <div className="stat-card" data-testid="metric-block-entities">
                    <span className="stat-value font-mono">
                      {analysisReport.statistics.total_block_entities}
                    </span>
                    <span className="stat-label">Block Entities</span>
                  </div>
                  <div className="stat-card" data-testid="metric-entities">
                    <span className="stat-value font-mono">
                      {analysisReport.statistics.total_entities}
                    </span>
                    <span className="stat-label">Entities</span>
                  </div>
                </div>
              </div>

              {/* Surface Features */}
              <div
                className="analysis-features-section"
                style={{ marginTop: '14px' }}
              >
                <h4
                  style={{
                    margin: '8px 0',
                    fontSize: '0.8rem',
                    color: 'var(--text-muted)',
                    textTransform: 'uppercase',
                  }}
                >
                  Surface Topology & Features
                </h4>
                <div className="keys-badges" data-testid="feature-badges">
                  {Object.entries(analysisReport.statistics.feature_counts).map(
                    ([kind, count]) => (
                      <span
                        key={kind}
                        className="key-badge"
                        data-testid={`feature-badge-${kind.toLowerCase()}`}
                      >
                        {kind}: {count}
                      </span>
                    ),
                  )}
                  {Object.keys(analysisReport.statistics.feature_counts).length ===
                    0 && (
                    <span className="text-muted text-xs">
                      No surface features detected
                    </span>
                  )}
                </div>
              </div>

              {/* Materials Breakdown */}
              <div
                className="analysis-materials-section"
                style={{ marginTop: '16px' }}
              >
                <div
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    marginBottom: '8px',
                  }}
                >
                  <h4
                    style={{
                      margin: 0,
                      fontSize: '0.8rem',
                      color: 'var(--text-muted)',
                      textTransform: 'uppercase',
                    }}
                  >
                    Materials ({analysisReport.materials.length} types)
                  </h4>
                  <button
                    className="btn btn-xs btn-secondary"
                    data-testid="copy-materials-csv"
                    onClick={handleCopyCsv}
                  >
                    {copiedCsv ? 'Copied CSV!' : 'Copy CSV'}
                  </button>
                </div>

                <input
                  type="text"
                  placeholder="Filter materials..."
                  className="coord-input"
                  data-testid="material-search"
                  value={materialSearch}
                  onChange={(e) => setMaterialSearch(e.target.value)}
                  style={{
                    width: '100%',
                    marginBottom: '8px',
                    boxSizing: 'border-box',
                  }}
                />

                <div
                  className="materials-table-wrapper"
                  style={{ maxHeight: '280px', overflowY: 'auto' }}
                >
                  <table className="props-table" data-testid="materials-table">
                    <thead>
                      <tr>
                        <th>Block</th>
                        <th style={{ textAlign: 'right' }}>Count</th>
                        <th style={{ textAlign: 'right' }}>Stacks (64)</th>
                        <th style={{ textAlign: 'right' }}>%</th>
                      </tr>
                    </thead>
                    <tbody>
                      {filteredMaterials.map((mat, idx) => (
                        <tr
                          key={`${mat.id}-${idx}`}
                          data-testid={`material-row-${idx}`}
                        >
                          <td
                            className="font-mono text-xs"
                            style={{ wordBreak: 'break-all' }}
                            title={mat.block_state}
                          >
                            {mat.id.replace(/^minecraft:/, '')}
                          </td>
                          <td
                            style={{ textAlign: 'right' }}
                            className="font-mono"
                          >
                            {mat.count.toLocaleString()}
                          </td>
                          <td
                            style={{ textAlign: 'right' }}
                            className="font-mono text-muted text-xs"
                          >
                            {mat.stacks_64 > 0
                              ? `${mat.stacks_64}st + ${mat.remainder}`
                              : `${mat.remainder}`}
                          </td>
                          <td
                            style={{ textAlign: 'right' }}
                            className="font-mono text-xs"
                          >
                            {mat.percentage.toFixed(1)}%
                          </td>
                        </tr>
                      ))}
                      {filteredMaterials.length === 0 && (
                        <tr>
                          <td
                            colSpan={4}
                            className="text-muted text-xs"
                            style={{ textAlign: 'center', padding: '8px' }}
                          >
                            No matching materials
                          </td>
                        </tr>
                      )}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          ) : (
            <div className="text-muted" data-testid="loading-analysis">
              Loading structure analysis...
            </div>
          )}
        </div>
      )}
    </aside>
  );
};
