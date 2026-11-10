import React from 'react';
import { useSchematicStore } from '../store/schematicStore';

const SEVERITY_LABEL = { error: 'Blocked', warning: 'Data loss', info: 'Note' } as const;

/**
 * Shown before an export whose target format cannot represent everything in
 * the document. Nothing is written until the user confirms.
 */
export const ExportLossDialog: React.FC = () => {
  const { pendingExport, confirmExport, cancelExport } = useSchematicStore();
  if (!pendingExport) return null;
  const { report } = pendingExport;

  return (
    <div className="modal-backdrop" data-testid="export-loss-dialog" role="dialog" aria-modal="true">
      <div className="modal-card">
        <h3>{report.blocked ? 'Export not possible' : 'This export will lose information'}</h3>
        <ul className="loss-list" data-testid="export-loss-list">
          {report.losses.map((loss) => (
            <li
              key={loss.code}
              className={`loss-item loss-${loss.severity}`}
              data-testid={`export-loss-${loss.code}`}
            >
              <span className="loss-badge">{SEVERITY_LABEL[loss.severity]}</span>
              <span>{loss.message}</span>
            </li>
          ))}
        </ul>
        <div className="modal-actions">
          <button className="btn" data-testid="export-loss-cancel" onClick={cancelExport}>
            Cancel
          </button>
          <button
            className="btn btn-success"
            data-testid="export-loss-confirm"
            onClick={() => confirmExport()}
            disabled={report.blocked}
          >
            Export anyway
          </button>
        </div>
      </div>
    </div>
  );
};
