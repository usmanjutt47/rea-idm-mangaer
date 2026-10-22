import { describe, expect, it } from 'vitest';
import { bytes, percent, type Download } from '../src/types';

const base: Download = { id: 'x', url: 'https://example.test/file', fileName: 'file', savePath: '', category: 'other', state: 'active', totalBytes: 100, downloadedBytes: 25, speedBps: 0, createdAt: '', updatedAt: '', resumable: true, segments: 1 };
describe('presentation helpers', () => {
  it('formats byte sizes', () => expect(bytes(1536)).toBe('1.5 KB'));
  it('never reports over one hundred percent', () => expect(percent({ ...base, downloadedBytes: 140 })).toBe(100));
});
