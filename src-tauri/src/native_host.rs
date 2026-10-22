//! Native Messaging bridge for Chromium-family browsers and Firefox.
//! The browser launches this executable; messages use the required 4-byte little-endian framing.
use std::{io::{self, Read, Write}, path::PathBuf};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Deserialize)] struct Request { token: String, action: String, url: Option<String> }
#[derive(Serialize)] struct Response { ok: bool, message: String }

fn main() {
  let mut stdin = io::stdin(); let mut stdout = io::stdout();
  while let Some(payload) = read_message(&mut stdin) {
    let response = match serde_json::from_slice::<Request>(&payload) {
      Ok(request) => handle(request), Err(_) => Response { ok: false, message: "Malformed native-message request".into() }
    };
    if write_message(&mut stdout, &response).is_err() { break; }
  }
}
fn handle(request: Request) -> Response {
  if !constant_time_eq(&request.token, &load_token()) { return Response { ok: false, message: "Bridge authorization failed".into() }; }
  if request.action != "enqueue" { return Response { ok: false, message: "Unsupported bridge action".into() }; }
  let Some(url) = request.url else { return Response { ok: false, message: "No URL provided".into() }; };
  match Url::parse(&url) { Ok(parsed) if matches!(parsed.scheme(), "http" | "https") => {
    if parsed.host_str().is_some_and(|host| host.ends_with("youtube.com") || host == "youtu.be") { return Response { ok: false, message: "YouTube pages are detected, but protected or page URLs cannot be downloaded. Use a permitted direct media URL.".into() }; }
    // A tiny, auditable file queue decouples browser processes from the GUI lifetime.
    // The desktop app consumes it at startup; URLs are never executed as commands.
    let queue = app_dir().join("browser-queue.jsonl");
    if let Some(parent) = queue.parent() { let _ = std::fs::create_dir_all(parent); }
    let line = serde_json::json!({"url": parsed.as_str(), "source":"browser-extension"}).to_string();
    match std::fs::OpenOptions::new().create(true).append(true).open(queue).and_then(|mut f| writeln!(f, "{line}")) { Ok(_) => Response { ok: true, message: "Download sent to REA Idm Mangaer".into() }, Err(_) => Response { ok: false, message: "Could not contact local download queue".into() }
  }, _ => Response { ok: false, message: "Only HTTP(S) URLs are permitted".into() } }
}
fn app_dir() -> PathBuf { dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("com.rea.idm-mangaer") }
fn load_token() -> String { let path=app_dir().join("bridge.token"); if let Ok(token)=std::fs::read_to_string(&path) { return token.trim().into(); } let token=uuid::Uuid::new_v4().to_string(); let _=std::fs::create_dir_all(app_dir()); let _=std::fs::write(path,&token); token }
fn read_message(input:&mut impl Read)->Option<Vec<u8>> { let mut size=[0;4]; input.read_exact(&mut size).ok()?; let len=u32::from_le_bytes(size) as usize; if len > 1024*1024 { return None; } let mut payload=vec![0;len]; input.read_exact(&mut payload).ok()?; Some(payload) }
fn write_message(output:&mut impl Write,response:&Response)->io::Result<()> { let data=serde_json::to_vec(response).expect("serializable"); output.write_all(&(data.len() as u32).to_le_bytes())?; output.write_all(&data)?; output.flush() }
fn constant_time_eq(a:&str,b:&str)->bool { if a.len()!=b.len(){return false;} a.bytes().zip(b.bytes()).fold(0u8,|(x,y)|x|(x^y))==0 }
