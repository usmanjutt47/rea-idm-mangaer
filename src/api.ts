import { invoke } from '@tauri-apps/api/core';
import type { DashboardStats, Download } from './types';

export const api = {
  listDownloads: () => invoke<Download[]>('list_downloads'),
  stats: () => invoke<DashboardStats>('dashboard_stats'),
  addDownload: (url: string, destination?: string) => invoke<Download>('add_download', { url, destination }),
  pause: (id: string) => invoke<void>('pause_download', { id }),
  resume: (id: string) => invoke<void>('resume_download', { id }),
  retry: (id: string) => invoke<void>('retry_download', { id }),
  remove: (id: string) => invoke<void>('remove_download', { id }),
  setSpeedLimit: (bytesPerSecond: number) => invoke<void>('set_speed_limit', { bytesPerSecond })
};
