import React from 'react';
import { useSchematicStore } from '../store/schematicStore';

export const Sidebar: React.FC = () => {
  const {
    document,
    selectedRegionId,
    selectRegion,
    meshData,
    selection,
    setSelection,
    selectAllRegion,
  } = useSchematicStore();

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

  return (
    <aside className="sidebar" data-testid="sidebar">
      {/* Regions panel */}
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

      {/* Selection Box panel */}
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
    </aside>
  );
};
