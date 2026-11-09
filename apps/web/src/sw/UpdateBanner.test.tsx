import React from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { UpdateBanner } from './UpdateBanner';
import { notifyUpdateAvailable } from './registerServiceWorker';

describe('UpdateBanner', () => {
  afterEach(() => {
    act(() => notifyUpdateAvailable(null));
    cleanup();
  });

  it('is hidden until an updated service worker is waiting', () => {
    render(<UpdateBanner />);
    expect(screen.queryByTestId('sw-update-banner')).toBeNull();

    act(() => notifyUpdateAvailable({ postMessage: vi.fn() }));
    expect(screen.getByTestId('sw-update-banner').textContent).toContain('New version available');
  });

  it('tells the waiting worker to take over when Reload is clicked', () => {
    const postMessage = vi.fn();
    act(() => notifyUpdateAvailable({ postMessage }));
    render(<UpdateBanner />);

    fireEvent.click(screen.getByTestId('sw-update-reload'));
    expect(postMessage).toHaveBeenCalledWith({ type: 'SKIP_WAITING' });
  });
});
