import { describe, expect, it } from 'vitest';
import { KNOWN_DATA_VERSIONS, dataVersionLabel } from './index';

describe('dataVersionLabel', () => {
  it('labels known DataVersions with their release', () => {
    expect(dataVersionLabel(2975)).toBe('1.18.2');
    expect(dataVersionLabel(3120)).toBe('1.19.2');
    expect(dataVersionLabel(3465)).toBe('1.20.1');
    expect(dataVersionLabel(3700)).toBe('1.20.4');
    expect(dataVersionLabel(3953)).toBe('1.21');
  });

  it('labels unknown versions relative to the closest older release', () => {
    expect(dataVersionLabel(3500)).toBe('1.20.1+ (3500)');
    expect(dataVersionLabel(99999)).toBe('1.21.4+ (99999)');
    expect(dataVersionLabel(100)).toBe('pre-1.12.2 (100)');
  });

  it('returns "unknown" for invalid input', () => {
    expect(dataVersionLabel(0)).toBe('unknown');
    expect(dataVersionLabel(-1)).toBe('unknown');
    expect(dataVersionLabel(3465.5)).toBe('unknown');
    expect(dataVersionLabel(Number.NaN)).toBe('unknown');
  });

  it('keeps the table sorted ascending', () => {
    const versions = [...KNOWN_DATA_VERSIONS.keys()];
    expect(versions).toEqual([...versions].sort((a, b) => a - b));
  });
});
