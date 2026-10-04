import React from 'react';
import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { App } from './App';

describe('Schematic IDE Shell UI', () => {
  it('renders the header, viewport, sidebar, tools panel, and status bar', () => {
    render(<App />);

    expect(screen.getByText('Schematic IDE')).toBeDefined();
    expect(screen.getByTestId('open-file-btn')).toBeDefined();
    expect(screen.getByTestId('viewport-container')).toBeDefined();
    expect(screen.getByTestId('empty-state')).toBeDefined();
    expect(screen.getByTestId('sidebar')).toBeDefined();
    expect(screen.getByTestId('tools-panel')).toBeDefined();
    expect(screen.getByTestId('status-bar')).toBeDefined();
  });
});
