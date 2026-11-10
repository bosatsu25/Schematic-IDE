import React, { useSyncExternalStore } from 'react';
import { applyUpdate, isUpdateAvailable, subscribeToUpdate } from './registerServiceWorker';

export const UpdateBanner: React.FC = () => {
  const available = useSyncExternalStore(subscribeToUpdate, isUpdateAvailable, () => false);
  if (!available) return null;

  return (
    <div className="sw-update-banner" role="status" data-testid="sw-update-banner">
      <span>New version available</span>
      <button type="button" data-testid="sw-update-reload" onClick={applyUpdate}>
        Reload
      </button>
    </div>
  );
};
