mod downloads;

use std::path::PathBuf;
use downloads::{DashboardStats, Download, DownloadManager};
use tauri::{menu::{MenuBuilder, MenuItemBuilder}, tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent}, Manager, State, WindowEvent};

#[tauri::command]
async fn list_downloads(manager: State<'_, DownloadManager>) -> Result<Vec<Download>, String> { manager.list().await }
#[tauri::command]
async fn dashboard_stats(manager: State<'_, DownloadManager>) -> Result<DashboardStats, String> { manager.stats().await }
#[tauri::command]
async fn add_download(url: String, destination: Option<String>, manager: State<'_, DownloadManager>) -> Result<Download, String> { manager.add(&url, destination).await }
#[tauri::command]
async fn pause_download(id: String, manager: State<'_, DownloadManager>) -> Result<(), String> { manager.pause(&id).await }
#[tauri::command]
async fn resume_download(id: String, manager: State<'_, DownloadManager>) -> Result<(), String> { manager.resume(&id).await }
#[tauri::command]
async fn retry_download(id: String, manager: State<'_, DownloadManager>) -> Result<(), String> { manager.retry(&id).await }
#[tauri::command]
async fn remove_download(id: String, manager: State<'_, DownloadManager>) -> Result<(), String> { manager.remove(&id).await }
#[tauri::command]
fn set_speed_limit(bytes_per_second: u64, manager: State<'_, DownloadManager>) { manager.set_speed_limit(bytes_per_second); }
#[tauri::command]
fn set_autostart(enabled: bool, app: tauri::AppHandle) -> Result<(), String> {
  use tauri_plugin_autostart::ManagerExt;
  if enabled { app.autolaunch().enable().map_err(|e| e.to_string()) } else { app.autolaunch().disable().map_err(|e| e.to_string()) }
}

pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_notification::init())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
    .setup(|app| {
      let data_dir: PathBuf = app.path().app_data_dir().map_err(|e| e.to_string())?;
      let manager = DownloadManager::new(data_dir)?;
      let recovery_manager = manager.clone();
      tauri::async_runtime::spawn(async move { recovery_manager.import_browser_queue(&data_dir).await; recovery_manager.recover().await; });
      app.manage(manager);
      let show = MenuItemBuilder::with_id("show", "Show REA Idm Mangaer").build(app)?;
      let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
      let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;
      TrayIconBuilder::with_id("main-tray").tooltip("REA Idm Mangaer").menu(&menu).on_menu_event(|app, event| match event.id.as_ref() { "show" => { if let Some(window) = app.get_webview_window("main") { let _ = window.show(); let _ = window.set_focus(); } }, "quit" => app.exit(0), _ => {} }).on_tray_icon_event(|tray, event| { if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event { if let Some(window) = tray.app_handle().get_webview_window("main") { let _ = window.show(); let _ = window.set_focus(); } } }).build(app)?;
      Ok(())
    })
    .on_window_event(|window, event| { if let WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _ = window.hide(); } })
    .invoke_handler(tauri::generate_handler![list_downloads, dashboard_stats, add_download, pause_download, resume_download, retry_download, remove_download, set_speed_limit, set_autostart])
    .run(tauri::generate_context!())
    .expect("failed to run REA Idm Mangaer");
}
