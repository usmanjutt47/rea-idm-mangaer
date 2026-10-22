export type DownloadState = 'active' | 'queued' | 'completed' | 'paused' | 'failed' | 'scheduled';

export interface Download {
  id: string;
  url: string;
  fileName: string;
  savePath: string;
  category: string;
  state: DownloadState;
  totalBytes: number;
  downloadedBytes: number;
  speedBps: number;
  etaSeconds?: number;
  error?: string;
  createdAt: string;
  updatedAt: string;
  resumable: boolean;
  segments: number;
}

export interface DashboardStats {
  active: number;
  queued: number;
  complete: number;
  speedBps: number;
}

export const bytes = (value: number) => {
  if (!value) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const index = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
  return `${(value / 1024 ** index).toFixed(index ? 1 : 0)} ${units[index]}`;
};

export const percent = (d: Download) => d.totalBytes ? Math.min(100, Math.round((d.downloadedBytes / d.totalBytes) * 100)) : 0;
