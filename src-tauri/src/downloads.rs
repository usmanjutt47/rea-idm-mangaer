use std::{collections::HashMap, path::{Path, PathBuf}, sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}}, time::{Duration, Instant}};

use chrono::Utc;
use futures_util::StreamExt;
use reqwest::{header::{ACCEPT_RANGES, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, RANGE}, Client, StatusCode};
use rusqlite::{params, Connection};
use serde::Serialize;
use tokio::{io::{AsyncSeekExt, AsyncWriteExt}, sync::Mutex, task::JoinSet, time::sleep};
use url::Url;
use uuid::Uuid;

const SEGMENT_THRESHOLD: u64 = 2 * 1024 * 1024;
const MAX_SEGMENTS: u8 = 8;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadState { Active, Queued, Completed, Paused, Failed, Scheduled }
impl DownloadState { fn as_str(&self) -> &'static str { match self { Self::Active => "active", Self::Queued => "queued", Self::Completed => "completed", Self::Paused => "paused", Self::Failed => "failed", Self::Scheduled => "scheduled" } } }
impl From<String> for DownloadState { fn from(value: String) -> Self { match value.as_str() { "active" => Self::Active, "queued" => Self::Queued, "completed" => Self::Completed, "paused" => Self::Paused, "failed" => Self::Failed, "scheduled" => Self::Scheduled, _ => Self::Failed } } }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Download { pub id: String, pub url: String, pub file_name: String, pub save_path: String, pub category: String, pub state: DownloadState, pub total_bytes: u64, pub downloaded_bytes: u64, pub speed_bps: u64, pub eta_seconds: Option<u64>, pub error: Option<String>, pub created_at: String, pub updated_at: String, pub resumable: bool, pub segments: u8 }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats { pub active: usize, pub queued: usize, pub complete: usize, pub speed_bps: u64 }

struct Control { paused: AtomicBool, cancelled: AtomicBool }
impl Control { fn new() -> Self { Self { paused: AtomicBool::new(false), cancelled: AtomicBool::new(false) } } }
struct RemoteMetadata { length: u64, ranges: bool, file_name: Option<String> }

#[derive(Clone)]
pub struct DownloadManager {
  database: Arc<Mutex<Connection>>,
  active: Arc<Mutex<HashMap<String, Arc<Control>>>>,
  client: Client,
  download_dir: PathBuf,
  speed_limit: Arc<AtomicU64>,
}

impl DownloadManager {
  pub fn new(data_dir: PathBuf) -> Result<Self, String> {
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    let download_dir = dirs::download_dir().unwrap_or_else(|| data_dir.join("Downloads")).join("REA Idm Mangaer");
    std::fs::create_dir_all(&download_dir).map_err(|e| e.to_string())?;
    let database = Connection::open(data_dir.join("downloads.sqlite3")).map_err(|e| e.to_string())?;
    database.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS downloads (id TEXT PRIMARY KEY, url TEXT NOT NULL, file_name TEXT NOT NULL, save_path TEXT NOT NULL, category TEXT NOT NULL, state TEXT NOT NULL, total_bytes INTEGER NOT NULL DEFAULT 0, downloaded_bytes INTEGER NOT NULL DEFAULT 0, speed_bps INTEGER NOT NULL DEFAULT 0, eta_seconds INTEGER, error TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, resumable INTEGER NOT NULL DEFAULT 0, segments INTEGER NOT NULL DEFAULT 1);") .map_err(|e| e.to_string())?;
    database.execute("UPDATE downloads SET state = 'queued', speed_bps = 0, updated_at = ?1 WHERE state = 'active'", params![now()]).map_err(|e| e.to_string())?;
    Ok(Self { database: Arc::new(Mutex::new(database)), active: Arc::new(Mutex::new(HashMap::new())), client: Client::builder().user_agent("REA-Idm-Mangaer/0.1 (+https://github.com/usmanjutt47/rea-idm-mangaer)").connect_timeout(Duration::from_secs(15)).build().map_err(|e| e.to_string())?, download_dir, speed_limit: Arc::new(AtomicU64::new(0)) })
  }

  pub async fn list(&self) -> Result<Vec<Download>, String> {
    let db = self.database.lock().await;
    let mut stmt = db.prepare("SELECT id,url,file_name,save_path,category,state,total_bytes,downloaded_bytes,speed_bps,eta_seconds,error,created_at,updated_at,resumable,segments FROM downloads ORDER BY updated_at DESC").map_err(|e| e.to_string())?;
    stmt.query_map([], row_to_download).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
  }
  pub async fn stats(&self) -> Result<DashboardStats, String> {
    let db = self.database.lock().await;
    let (active, queued, complete, speed): (i64, i64, i64, i64) = db.query_row("SELECT SUM(state='active'), SUM(state='queued'), SUM(state='completed'), SUM(CASE WHEN state='active' THEN speed_bps ELSE 0 END) FROM downloads", [], |r| Ok((r.get::<_,Option<i64>>(0)?.unwrap_or(0), r.get::<_,Option<i64>>(1)?.unwrap_or(0), r.get::<_,Option<i64>>(2)?.unwrap_or(0), r.get::<_,Option<i64>>(3)?.unwrap_or(0)))).map_err(|e| e.to_string())?;
    Ok(DashboardStats { active: active as usize, queued: queued as usize, complete: complete as usize, speed_bps: speed as u64 })
  }
  pub async fn add(&self, url: &str, destination: Option<String>) -> Result<Download, String> {
    let parsed = Url::parse(url).map_err(|_| "Only valid HTTP(S) URLs can be downloaded.".to_owned())?;
    if !matches!(parsed.scheme(), "http" | "https") { return Err("Only HTTP(S) URLs can be downloaded.".to_owned()); }
    let file_name = name_from_url(&parsed); let save_path = destination.map(PathBuf::from).unwrap_or_else(|| self.download_dir.join(&file_name));
    let item = Download { id: Uuid::new_v4().to_string(), url: url.to_owned(), file_name, save_path: save_path.to_string_lossy().to_string(), category: category_for(&save_path), state: DownloadState::Queued, total_bytes: 0, downloaded_bytes: 0, speed_bps: 0, eta_seconds: None, error: None, created_at: now(), updated_at: now(), resumable: false, segments: 1 };
    self.insert(&item).await?; self.start(item.id.clone()).await; Ok(item)
  }
  pub async fn pause(&self, id: &str) -> Result<(), String> { if let Some(control) = self.active.lock().await.get(id) { control.paused.store(true, Ordering::Release); } else { self.update_state(id, DownloadState::Paused, None).await?; } Ok(()) }
  pub async fn resume(&self, id: &str) -> Result<(), String> {
    if let Some(control) = self.active.lock().await.get(id) {
      control.paused.store(false, Ordering::Release);
      self.update_state(id, DownloadState::Active, None).await?;
    } else { self.start(id.to_owned()).await; }
    Ok(())
  }
  pub async fn retry(&self, id: &str) -> Result<(), String> { self.start(id.to_owned()).await; Ok(()) }
  pub async fn remove(&self, id: &str) -> Result<(), String> { if let Some(control) = self.active.lock().await.get(id) { control.cancelled.store(true, Ordering::Release); } self.database.lock().await.execute("DELETE FROM downloads WHERE id=?1", params![id]).map_err(|e| e.to_string())?; Ok(()) }
  pub fn set_speed_limit(&self, value: u64) { self.speed_limit.store(value, Ordering::Release); }
  pub async fn recover(&self) { if let Ok(items) = self.list().await { for item in items.into_iter().filter(|d| matches!(d.state, DownloadState::Queued)) { self.start(item.id).await; } } }
  pub async fn import_browser_queue(&self, data_dir: &Path) {
    let queue = data_dir.join("browser-queue.jsonl");
    let processing = data_dir.join("browser-queue.processing");
    if !queue.exists() || std::fs::rename(&queue, &processing).is_err() { return; }
    if let Ok(contents) = std::fs::read_to_string(&processing) {
      for line in contents.lines() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
          if let Some(url) = value.get("url").and_then(|v| v.as_str()) { if !self.has_pending_url(url).await.unwrap_or(true) { let _ = self.add(url, None).await; } }
        }
      }
    }
    let _ = std::fs::remove_file(processing);
  }
  async fn has_pending_url(&self, url: &str) -> Result<bool, String> { let db = self.database.lock().await; let count: i64 = db.query_row("SELECT COUNT(*) FROM downloads WHERE url=?1 AND state IN ('active','queued','paused','scheduled')", params![url], |row| row.get(0)).map_err(|e| e.to_string())?; Ok(count > 0) }
  async fn start(&self, id: String) {
    if self.active.lock().await.contains_key(&id) { return; }
    let control = Arc::new(Control::new()); self.active.lock().await.insert(id.clone(), control.clone());
    let manager = self.clone(); tauri::async_runtime::spawn(async move { if let Err(error) = manager.run(id.clone(), control.clone()).await { let _ = manager.update_state(&id, DownloadState::Failed, Some(error)).await; } manager.active.lock().await.remove(&id); });
  }
  async fn run(&self, id: String, control: Arc<Control>) -> Result<(), String> {
    let mut item = self.one(&id).await?; self.update_state(&id, DownloadState::Active, None).await?;
    let remote = self.probe(&item.url).await?;
    if let Some(name) = remote.file_name { item.file_name = safe_file_name(&name); item.save_path = self.download_dir.join(&item.file_name).to_string_lossy().to_string(); }
    let target = self.unique_path(PathBuf::from(&item.save_path)).await?;
    item.file_name = target.file_name().and_then(|name| name.to_str()).unwrap_or("download.bin").to_owned(); item.save_path = target.to_string_lossy().to_string();
    item.total_bytes = remote.length; item.resumable = remote.ranges; item.segments = if remote.ranges && remote.length >= SEGMENT_THRESHOLD { ((remote.length / SEGMENT_THRESHOLD).clamp(2, MAX_SEGMENTS as u64)) as u8 } else { 1 }; self.update_metadata(&item).await?;
    if let Some(parent) = target.parent() { tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?; }
    let temp = PathBuf::from(format!("{}.part", item.save_path));
    if item.segments > 1 { self.segmented(&item, &temp, control.clone()).await?; } else { self.single(&item, &temp, control.clone()).await?; }
    if control.cancelled.load(Ordering::Acquire) || control.paused.load(Ordering::Acquire) { return Ok(()); }
    if item.total_bytes > 0 && tokio::fs::metadata(&temp).await.map_err(|e| e.to_string())?.len() != item.total_bytes { return Err("Integrity check failed: downloaded length does not match the server length.".into()); }
    tokio::fs::rename(&temp, &target).await.map_err(|e| e.to_string())?; self.complete(&id).await
  }
  async fn probe(&self, url: &str) -> Result<RemoteMetadata, String> {
    let head = self.client.head(url).send().await.map_err(network_error)?;
    if head.status().is_success() { return Ok(metadata_from_headers(head.headers(), false)); }
    let probe = self.client.get(url).header(RANGE, "bytes=0-0").send().await.map_err(network_error)?;
    if !probe.status().is_success() && probe.status() != StatusCode::PARTIAL_CONTENT { return Err(format!("Remote server returned HTTP {} to HEAD and GET fallback", probe.status())); }
    Ok(metadata_from_headers(probe.headers(), probe.status() == StatusCode::PARTIAL_CONTENT))
  }
  async fn unique_path(&self, preferred: PathBuf) -> Result<PathBuf, String> {
    if tokio::fs::metadata(&preferred).await.is_err() { return Ok(preferred); }
    let parent = preferred.parent().ok_or("Download destination has no parent directory.")?; let stem = preferred.file_stem().and_then(|v| v.to_str()).unwrap_or("download"); let extension = preferred.extension().and_then(|v| v.to_str());
    for index in 1..10_000 { let name = match extension { Some(ext) => format!("{stem} ({index}).{ext}"), None => format!("{stem} ({index})") }; let candidate = parent.join(name); if tokio::fs::metadata(&candidate).await.is_err() { return Ok(candidate); } }
    Err("Could not create a unique filename after 9,999 attempts.".into())
  }
  async fn single(&self, item: &Download, temp: &Path, control: Arc<Control>) -> Result<(), String> {
    let existing = if item.resumable { tokio::fs::metadata(temp).await.map(|m| m.len()).unwrap_or(0) } else { 0 };
    let mut request = self.client.get(&item.url); if existing > 0 { request = request.header(RANGE, format!("bytes={existing}-")); }
    let response = request.send().await.map_err(network_error)?; if !response.status().is_success() { return Err(format!("Remote server returned HTTP {}", response.status())); }
    let append = existing > 0 && response.status() == StatusCode::PARTIAL_CONTENT;
    let mut file = if append { tokio::fs::OpenOptions::new().append(true).open(temp).await.map_err(|e| e.to_string())? } else { tokio::fs::File::create(temp).await.map_err(|e| e.to_string())? }; let mut stream = response.bytes_stream(); let mut downloaded = if append { existing } else { 0 }; let started = Instant::now(); let mut last_update = Instant::now();
    while let Some(chunk) = stream.next().await { if control.cancelled.load(Ordering::Acquire) { return Ok(()); } while control.paused.load(Ordering::Acquire) { self.update_state(&item.id, DownloadState::Paused, None).await?; sleep(Duration::from_millis(150)).await; if control.cancelled.load(Ordering::Acquire) { return Ok(()); } } self.update_state(&item.id, DownloadState::Active, None).await?; let chunk = chunk.map_err(network_error)?; file.write_all(&chunk).await.map_err(|e| e.to_string())?; downloaded += chunk.len() as u64; self.limit(chunk.len()).await; if last_update.elapsed() > Duration::from_millis(300) { self.progress(&item.id, downloaded, rate(downloaded, started), item.total_bytes).await?; last_update = Instant::now(); } }
    file.flush().await.map_err(|e| e.to_string())?; self.progress(&item.id, downloaded, rate(downloaded, started), item.total_bytes).await
  }
  async fn segmented(&self, item: &Download, temp: &Path, control: Arc<Control>) -> Result<(), String> {
    let file = tokio::fs::File::create(temp).await.map_err(|e| e.to_string())?; file.set_len(item.total_bytes).await.map_err(|e| e.to_string())?; drop(file);
    let progress = Arc::new(AtomicU64::new(0)); let started = Instant::now(); let mut tasks = JoinSet::new(); let segment_size = item.total_bytes / item.segments as u64;
    for index in 0..item.segments { let start = index as u64 * segment_size; let end = if index + 1 == item.segments { item.total_bytes - 1 } else { start + segment_size - 1 }; let client = self.client.clone(); let url = item.url.clone(); let path = temp.to_path_buf(); let total = progress.clone(); let stop = control.clone(); let limit = self.speed_limit.clone(); tasks.spawn(async move { fetch_range(client, url, path, start, end, total, stop, limit).await }); }
    while !tasks.is_empty() { if control.cancelled.load(Ordering::Acquire) { tasks.abort_all(); return Ok(()); } while control.paused.load(Ordering::Acquire) { self.update_state(&item.id, DownloadState::Paused, None).await?; sleep(Duration::from_millis(150)).await; if control.cancelled.load(Ordering::Acquire) { tasks.abort_all(); return Ok(()); } } self.update_state(&item.id, DownloadState::Active, None).await?; while let Some(joined) = tasks.try_join_next() { joined.map_err(|e| e.to_string())??; } self.progress(&item.id, progress.load(Ordering::Acquire), rate(progress.load(Ordering::Acquire), started), item.total_bytes).await?; sleep(Duration::from_millis(250)).await; }
    self.progress(&item.id, item.total_bytes, rate(item.total_bytes, started), item.total_bytes).await
  }
  async fn limit(&self, bytes: usize) { let limit = self.speed_limit.load(Ordering::Acquire); if limit > 0 { sleep(Duration::from_secs_f64(bytes as f64 / limit as f64)).await; } }
  async fn insert(&self, item: &Download) -> Result<(), String> { self.database.lock().await.execute("INSERT INTO downloads VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)", params![item.id,item.url,item.file_name,item.save_path,item.category,item.state.as_str(),item.total_bytes as i64,item.downloaded_bytes as i64,item.speed_bps as i64,item.eta_seconds.map(|v|v as i64),item.error,item.created_at,item.updated_at,item.resumable as i32,item.segments as i32]).map_err(|e|e.to_string())?; Ok(()) }
  async fn one(&self, id: &str) -> Result<Download, String> { self.database.lock().await.query_row("SELECT id,url,file_name,save_path,category,state,total_bytes,downloaded_bytes,speed_bps,eta_seconds,error,created_at,updated_at,resumable,segments FROM downloads WHERE id=?1", params![id], row_to_download).map_err(|e| e.to_string()) }
  async fn update_state(&self, id: &str, state: DownloadState, error: Option<String>) -> Result<(), String> { self.database.lock().await.execute("UPDATE downloads SET state=?2, error=?3, speed_bps=0, updated_at=?4 WHERE id=?1", params![id,state.as_str(),error,now()]).map_err(|e|e.to_string())?; Ok(()) }
  async fn update_metadata(&self, item: &Download) -> Result<(), String> { self.database.lock().await.execute("UPDATE downloads SET file_name=?2,save_path=?3,category=?4,total_bytes=?5,resumable=?6,segments=?7,updated_at=?8 WHERE id=?1", params![item.id,item.file_name,item.save_path,item.category,item.total_bytes as i64,item.resumable as i32,item.segments as i32,now()]).map_err(|e|e.to_string())?; Ok(()) }
  async fn progress(&self, id: &str, bytes: u64, speed: u64, total: u64) -> Result<(), String> { let eta = if speed > 0 && total > bytes { Some(((total - bytes) / speed) as i64) } else { None }; self.database.lock().await.execute("UPDATE downloads SET downloaded_bytes=?2,speed_bps=?3,eta_seconds=?4,updated_at=?5 WHERE id=?1", params![id,bytes as i64,speed as i64,eta,now()]).map_err(|e|e.to_string())?; Ok(()) }
  async fn complete(&self, id: &str) -> Result<(), String> { self.database.lock().await.execute("UPDATE downloads SET state='completed',downloaded_bytes=total_bytes,speed_bps=0,eta_seconds=NULL,error=NULL,updated_at=?2 WHERE id=?1",params![id,now()]).map_err(|e|e.to_string())?; Ok(()) }
}

async fn fetch_range(client: Client, url: String, path: PathBuf, start: u64, end: u64, total: Arc<AtomicU64>, control: Arc<Control>, speed_limit: Arc<AtomicU64>) -> Result<(), String> { let response = client.get(url).header(RANGE, format!("bytes={start}-{end}")).send().await.map_err(network_error)?; if response.status() != StatusCode::PARTIAL_CONTENT { return Err("Remote server stopped supporting ranged transfers.".to_owned()); } let mut file = tokio::fs::OpenOptions::new().write(true).open(path).await.map_err(|e|e.to_string())?; file.seek(std::io::SeekFrom::Start(start)).await.map_err(|e|e.to_string())?; let mut stream=response.bytes_stream(); while let Some(chunk)=stream.next().await { while control.paused.load(Ordering::Acquire) { if control.cancelled.load(Ordering::Acquire) { return Ok(()); } sleep(Duration::from_millis(150)).await; } if control.cancelled.load(Ordering::Acquire) { return Ok(()); } let chunk=chunk.map_err(network_error)?; file.write_all(&chunk).await.map_err(|e|e.to_string())?; total.fetch_add(chunk.len() as u64,Ordering::AcqRel); let limit=speed_limit.load(Ordering::Acquire); if limit>0 { sleep(Duration::from_secs_f64(chunk.len() as f64/limit as f64)).await; } } Ok(()) }
fn row_to_download(row: &rusqlite::Row<'_>) -> rusqlite::Result<Download> { Ok(Download { id:row.get(0)?,url:row.get(1)?,file_name:row.get(2)?,save_path:row.get(3)?,category:row.get(4)?,state:DownloadState::from(row.get::<_,String>(5)?),total_bytes:row.get::<_,i64>(6)? as u64,downloaded_bytes:row.get::<_,i64>(7)? as u64,speed_bps:row.get::<_,i64>(8)? as u64,eta_seconds:row.get::<_,Option<i64>>(9)?.map(|v|v as u64),error:row.get(10)?,created_at:row.get(11)?,updated_at:row.get(12)?,resumable:row.get::<_,i32>(13)? != 0,segments:row.get::<_,i32>(14)? as u8 }) }
fn now() -> String { Utc::now().to_rfc3339() }
fn rate(bytes:u64, started:Instant)->u64 { let s=started.elapsed().as_secs_f64(); if s>0.0 {(bytes as f64/s) as u64} else {0} }
fn network_error(error:reqwest::Error)->String { if error.is_timeout() { "Connection timed out; retry is available.".into() } else { format!("Network error: {error}") } }
fn metadata_from_headers(headers: &reqwest::header::HeaderMap, proven_ranges: bool) -> RemoteMetadata { let from_length = headers.get(CONTENT_LENGTH).and_then(|h| h.to_str().ok()).and_then(|v| v.parse().ok()).unwrap_or(0); let from_range = headers.get(CONTENT_RANGE).and_then(|h| h.to_str().ok()).and_then(|v| v.rsplit('/').next()).and_then(|v| v.parse().ok()).unwrap_or(0); RemoteMetadata { length: from_range.max(from_length), ranges: proven_ranges || headers.get(ACCEPT_RANGES).and_then(|h| h.to_str().ok()).is_some_and(|v| v.eq_ignore_ascii_case("bytes")), file_name: headers.get(CONTENT_DISPOSITION).and_then(|h| h.to_str().ok()).and_then(filename_from_disposition) } }
fn name_from_url(url:&Url)->String { url.path_segments().and_then(|mut p|p.next_back()).filter(|s|!s.is_empty()).map(safe_file_name).unwrap_or_else(|| format!("download-{}.bin",Uuid::new_v4())) }
fn filename_from_disposition(header:&str)->Option<String> { header.split(';').find_map(|p| p.trim().strip_prefix("filename=")).map(|v| v.trim_matches('"').to_owned()) }
fn safe_file_name(value:&str)->String { let result:String=value.chars().map(|c|if matches!(c,'<'|'>'|':'|'"'|'/'|'\\'|'|'|'?'|'*'){'_'}else{c}).collect(); if result.is_empty(){"download.bin".into()}else{result.chars().take(180).collect()} }
fn category_for(path:&Path)->String { match path.extension().and_then(|s|s.to_str()).unwrap_or("").to_ascii_lowercase().as_str() { "mp4"|"mkv"|"webm"|"avi"|"mov" => "video", "mp3"|"wav"|"flac"|"m4a"|"ogg" => "audio", "zip"|"rar"|"7z"|"tar"|"gz" => "archive", "pdf"|"doc"|"docx"|"txt" => "document", _ => "other" }.into() }
