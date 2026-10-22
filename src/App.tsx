import { useCallback, useEffect, useMemo, useState } from 'react';
import { api } from './api';
import { bytes, percent, type DashboardStats, type Download, type DownloadState } from './types';

const views: { key: 'all' | DownloadState; label: string }[] = [
  { key: 'all', label: 'All downloads' }, { key: 'active', label: 'Active' }, { key: 'queued', label: 'Queued' },
  { key: 'completed', label: 'Completed' }, { key: 'paused', label: 'Paused' }, { key: 'failed', label: 'Failed' }, { key: 'scheduled', label: 'Scheduled' }
];

const fallbackStats: DashboardStats = { active: 0, queued: 0, complete: 0, speedBps: 0 };

function App() {
  const [downloads, setDownloads] = useState<Download[]>([]);
  const [stats, setStats] = useState<DashboardStats>(fallbackStats);
  const [view, setView] = useState<'all' | DownloadState>('all');
  const [url, setUrl] = useState('');
  const [theme, setTheme] = useState<'light' | 'dark'>('dark');
  const [message, setMessage] = useState('');

  const refresh = useCallback(async () => {
    try { const [items, dashboard] = await Promise.all([api.listDownloads(), api.stats()]); setDownloads(items); setStats(dashboard); }
    catch { setMessage('Desktop service is unavailable. Start REA Idm Mangaer from Tauri.'); }
  }, []);
  useEffect(() => { void refresh(); const timer = window.setInterval(() => void refresh(), 900); return () => window.clearInterval(timer); }, [refresh]);
  useEffect(() => { document.documentElement.dataset.theme = theme; }, [theme]);
  useEffect(() => {
    const onPaste = (event: ClipboardEvent) => { const text = event.clipboardData?.getData('text/plain')?.trim(); if (text && /^https?:\/\//i.test(text)) setUrl(text); };
    window.addEventListener('paste', onPaste); return () => window.removeEventListener('paste', onPaste);
  }, []);

  const submit = async () => {
    if (!/^https?:\/\//i.test(url)) { setMessage('Enter a valid HTTP or HTTPS download URL.'); return; }
    try { await api.addDownload(url); setUrl(''); setMessage('Added to the download queue.'); await refresh(); } catch (error) { setMessage(String(error)); }
  };
  const shown = useMemo(() => view === 'all' ? downloads : downloads.filter(d => d.state === view), [downloads, view]);
  const action = async (download: Download) => {
    try {
      if (download.state === 'active') await api.pause(download.id);
      else if (download.state === 'paused' || download.state === 'queued') await api.resume(download.id);
      else if (download.state === 'failed') await api.retry(download.id);
      await refresh();
    } catch (error) { setMessage(String(error)); }
  };

  return <main className="app-shell" onDragOver={e => e.preventDefault()} onDrop={e => { e.preventDefault(); const text = e.dataTransfer.getData('text/uri-list') || e.dataTransfer.getData('text/plain'); if (text) setUrl(text.trim()); }}>
    <aside className="sidebar">
      <div className="brand"><div className="brand-mark">R</div><span>REA <b>Idm Mangaer</b></span></div>
      <nav>{views.map(item => <button key={item.key} className={view === item.key ? 'nav-item selected' : 'nav-item'} onClick={() => setView(item.key)}><span>{item.label}</span><em>{item.key === 'all' ? downloads.length : downloads.filter(d => d.state === item.key).length}</em></button>)}</nav>
      <div className="sidebar-bottom"><button className="nav-item" onClick={() => setMessage('Settings are stored locally. Browser bridge and startup options are documented in the README.')}><span>Settings</span><span>⚙</span></button><small>v0.1.0 · local-first</small></div>
    </aside>
    <section className="workspace">
      <header><div><p className="eyebrow">DOWNLOAD CENTER</p><h1>{view === 'all' ? 'Your downloads' : views.find(v => v.key === view)?.label}</h1></div><div className="header-actions"><button className="icon-button" title="Toggle theme" onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}>{theme === 'dark' ? '☀' : '◐'}</button><button className="primary" onClick={() => void submit()}>+ New download</button></div></header>
      <div className="add-bar"><span>↧</span><input value={url} onChange={e => setUrl(e.target.value)} onKeyDown={e => e.key === 'Enter' && void submit()} placeholder="Paste a download link, or drop one anywhere"/><button onClick={() => void submit()}>Download</button></div>
      {message && <div className="notice" role="status">{message}<button onClick={() => setMessage('')}>×</button></div>}
      <section className="stat-grid"><Stat label="Downloading" value={String(stats.active)} detail={`${bytes(stats.speedBps)}/s combined`} accent="violet"/><Stat label="In queue" value={String(stats.queued)} detail="ready when capacity opens" accent="blue"/><Stat label="Completed" value={String(stats.complete)} detail="in your local history" accent="green"/><Stat label="Connection" value="Secure" detail="HTTPS where supported" accent="gold"/></section>
      <section className="downloads-card"><div className="list-title"><h2>Downloads</h2><span>{shown.length} item{shown.length === 1 ? '' : 's'}</span></div>{shown.length === 0 ? <Empty view={view}/> : <div className="download-list">{shown.map(d => <DownloadRow key={d.id} download={d} onAction={() => void action(d)} onRemove={() => void api.remove(d.id).then(refresh)}/>)}</div>}</section>
    </section>
  </main>;
}

function Stat({ label, value, detail, accent }: { label: string; value: string; detail: string; accent: string }) { return <article className={`stat ${accent}`}><span>{label}</span><strong>{value}</strong><small>{detail}</small></article>; }
function Empty({ view }: { view: string }) { return <div className="empty"><div>⇩</div><h3>No {view === 'all' ? '' : `${view} `}downloads</h3><p>Paste a direct HTTP(S) link above to begin a safe, resumable download.</p></div>; }
function DownloadRow({ download: d, onAction, onRemove }: { download: Download; onAction: () => void; onRemove: () => void }) {
  const isDone = d.state === 'completed'; const isFailure = d.state === 'failed';
  return <article className="download-row"><div className={`file-icon ${d.category}`}>{d.category.slice(0, 1).toUpperCase()}</div><div className="download-info"><div className="download-name"><strong>{d.fileName}</strong><span className={`badge ${d.state}`}>{d.state}</span></div><p>{bytes(d.downloadedBytes)} of {d.totalBytes ? bytes(d.totalBytes) : 'unknown size'} · {d.segments} connection{d.segments === 1 ? '' : 's'} {d.resumable ? '· resumable' : ''}</p><div className="progress"><i style={{ width: `${percent(d)}%` }} /></div><small>{isDone ? 'Ready to open' : isFailure ? d.error || 'Download failed' : `${percent(d)}% · ${bytes(d.speedBps)}/s${d.etaSeconds ? ` · ${d.etaSeconds}s remaining` : ''}`}</small></div><div className="download-actions">{!isDone && <button onClick={onAction}>{d.state === 'active' ? 'Pause' : d.state === 'failed' ? 'Retry' : 'Resume'}</button>}<button className="more" aria-label="Remove from history" onClick={onRemove}>···</button></div></article>;
}
export default App;
