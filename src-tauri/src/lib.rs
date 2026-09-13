mod agent;
mod auth;
mod diffs;
mod grok_bin;
mod paths;
mod permission;
mod recents;
mod rpc;
mod sessions;

use agent::{AgentHost, HostEvent};
use recents::RecentsState;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;

struct Live {
    host: Arc<AgentHost>,
    session_id: Mutex<Option<String>>,
    cwd: Mutex<Option<String>>,
    pending_perm: Mutex<Option<Value>>,
}

struct AppState {
    live: Mutex<Option<Arc<Live>>>,
}

#[derive(Serialize)]
struct AppStatus {
    grok_bin: Option<String>,
    signed_in: bool,
    grok_home: String,
}

fn recents_file() -> PathBuf {
    paths::recents_path()
}

fn load_state() -> RecentsState {
    recents::load_recents(&recents_file())
}

fn store_state(state: &RecentsState) -> Result<(), String> {
    recents::save_recents(&recents_file(), state)
}

fn window_unfocused(app: &AppHandle) -> bool {
    !app.webview_windows()
        .values()
        .any(|w| w.is_focused().unwrap_or(true))
}

fn notify(app: &AppHandle, body: &str) {
    let _ = app
        .notification()
        .builder()
        .title("Grok Build")
        .body(body)
        .show();
}

#[tauri::command]
fn app_status() -> AppStatus {
    let home = paths::grok_home();
    AppStatus {
        grok_bin: grok_bin::resolve_grok_bin().map(|p| p.display().to_string()),
        signed_in: auth::is_signed_in_at(&paths::auth_json_path(&home)),
        grok_home: home.display().to_string(),
    }
}

#[tauri::command]
fn sign_in() -> Result<AppStatus, String> {
    let bin = grok_bin::resolve_grok_bin().ok_or("Grok binary not found")?;
    let _ = Command::new(&bin)
        .arg("login")
        .status()
        .map_err(|e| e.to_string())?;
    let status = app_status();
    if status.signed_in {
        Ok(status)
    } else {
        Err("Sign in did not produce auth.json".into())
    }
}

#[tauri::command]
fn list_recents() -> RecentsState {
    load_state()
}

#[tauri::command]
fn open_folder(path: String) -> Result<Vec<sessions::SessionRow>, String> {
    let p = PathBuf::from(&path);
    if !p.is_dir() {
        return Err("Folder unreadable".into());
    }
    std::fs::read_dir(&p).map_err(|_| "Folder unreadable".to_string())?;
    let rec = recents::push_recent(load_state(), path);
    store_state(&rec)?;
    Ok(sessions::list_sessions(&paths::grok_home(), &p))
}

#[tauri::command]
fn remove_recent(path: String) -> RecentsState {
    let rec = recents::remove_recent(load_state(), &path);
    let _ = store_state(&rec);
    rec
}

#[tauri::command]
fn list_sessions_cmd(cwd: String) -> Vec<sessions::SessionRow> {
    sessions::list_sessions(&paths::grok_home(), Path::new(&cwd))
}

fn handle_event(app: &AppHandle, live: &Live, ev: HostEvent) -> bool {
    match ev {
        HostEvent::Notification { method, params } => {
            if method == "session/update" {
                let _ = app.emit("agent://update", params.clone());
                let kind = params
                    .pointer("/update/sessionUpdate")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if kind == "tool_call_update" {
                    let status = params
                        .pointer("/update/status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if status == "completed" || status == "failed" {
                        let _ = app.emit("agent://refresh-diffs", json!({}));
                    }
                }
            } else if method == "x.ai/session_notification" {
                let _ = app.emit("agent://refresh-diffs", params);
            }
            true
        }
        HostEvent::Request { id, method, params } => {
            if method == "session/request_permission" {
                *live.pending_perm.lock().unwrap() = Some(id.clone());
                let options = params.get("options").cloned().unwrap_or(json!([]));
                let parsed: Vec<permission::PermissionOption> =
                    serde_json::from_value(options).unwrap_or_default();
                let _ = app.emit(
                    "agent://permission",
                    json!({
                        "id": id,
                        "toolCall": params.get("toolCall"),
                        "options": permission::ui_options(&parsed)
                    }),
                );
                if window_unfocused(app) {
                    notify(app, "Grok needs approval");
                }
            } else if method == "fs/read_text_file" {
                let path = params.get("path").and_then(|p| p.as_str()).unwrap_or("");
                let _ = live.host.respond_fs_read(&id, Path::new(path));
            } else if method == "fs/write_text_file" {
                let _ = live.host.respond_error(&id, "writeTextFile is disabled");
            }
            true
        }
        HostEvent::Exit => {
            let _ = app.emit("agent://exit", json!({}));
            false
        }
    }
}

fn ensure_agent(state: &AppState, app: &AppHandle) -> Result<Arc<Live>, String> {
    if let Some(live) = state.live.lock().unwrap().as_ref() {
        return Ok(live.clone());
    }
    let bin = grok_bin::resolve_grok_bin().ok_or("Grok binary not found")?;
    let (tx, rx) = std::sync::mpsc::channel::<HostEvent>();
    let host = Arc::new(AgentHost::spawn(
        &bin,
        Arc::new(move |e| {
            let _ = tx.send(e);
        }),
    )?);
    let _ = host.initialize()?;
    let live = Arc::new(Live {
        host: host.clone(),
        session_id: Mutex::new(None),
        cwd: Mutex::new(None),
        pending_perm: Mutex::new(None),
    });
    *state.live.lock().unwrap() = Some(live.clone());
    let app2 = app.clone();
    let live2 = live.clone();
    thread::spawn(move || {
        while let Ok(ev) = rx.recv() {
            if !handle_event(&app2, &live2, ev) {
                break;
            }
        }
    });
    Ok(live)
}

fn deny_pending(live: &Live) {
    if let Some(id) = live.pending_perm.lock().unwrap().take() {
        let _ = live.host.respond_permission(&id, None);
    }
}

#[tauri::command]
fn session_new(
    app: AppHandle,
    state: State<AppState>,
    cwd: String,
    worktree: bool,
) -> Result<Value, String> {
    let live = ensure_agent(&state, &app)?;
    deny_pending(&live);
    let mut session_cwd = cwd.clone();
    if worktree {
        match live.host.create_worktree(&cwd) {
            Ok(v) => {
                if let Some(p) = v
                    .get("path")
                    .or_else(|| v.get("cwd"))
                    .and_then(|x| x.as_str())
                {
                    session_cwd = p.to_string();
                }
            }
            Err(e) => return Err(format!("Worktree unavailable: {e}")),
        }
    }
    let created = live.host.session_new(&session_cwd)?;
    let id = created
        .get("sessionId")
        .and_then(|s| s.as_str())
        .ok_or("session/new missing sessionId")?
        .to_string();
    *live.session_id.lock().unwrap() = Some(id.clone());
    *live.cwd.lock().unwrap() = Some(session_cwd.clone());
    Ok(json!({
        "sessionId": id,
        "cwd": session_cwd,
        "configOptions": created.get("configOptions")
    }))
}

#[tauri::command]
fn session_load(
    app: AppHandle,
    state: State<AppState>,
    cwd: String,
    id: String,
) -> Result<Value, String> {
    let live = ensure_agent(&state, &app)?;
    deny_pending(&live);
    if let Some(sid) = live.session_id.lock().unwrap().clone() {
        let _ = live.host.session_cancel(&sid);
    }
    match live.host.session_load(&cwd, &id) {
        Ok(v) => {
            *live.session_id.lock().unwrap() = Some(id.clone());
            *live.cwd.lock().unwrap() = Some(cwd.clone());
            if v.get("updates").is_none() && v.get("history").is_none() {
                let updates =
                    sessions::load_updates_jsonl(&paths::grok_home(), Path::new(&cwd), &id);
                Ok(json!({ "sessionId": id, "updates": updates }))
            } else {
                Ok(v)
            }
        }
        Err(e) => Err(e),
    }
}

#[tauri::command]
fn session_prompt(app: AppHandle, state: State<AppState>, text: String) -> Result<(), String> {
    let live = state
        .live
        .lock()
        .unwrap()
        .clone()
        .ok_or("No live session")?;
    let sid = live
        .session_id
        .lock()
        .unwrap()
        .clone()
        .ok_or("No live session")?;
    thread::spawn(move || {
        match live.host.session_prompt(&sid, &text) {
            Ok(v) => {
                let _ = app.emit("agent://prompt-done", json!({ "ok": true, "result": v }));
                if window_unfocused(&app) {
                    notify(&app, "Grok finished");
                }
            }
            Err(e) => {
                let _ = app.emit("agent://prompt-done", json!({ "ok": false, "error": e }));
            }
        }
    });
    Ok(())
}

#[tauri::command]
fn session_cancel(state: State<AppState>) -> Result<(), String> {
    let live = state
        .live
        .lock()
        .unwrap()
        .clone()
        .ok_or("No live session")?;
    deny_pending(&live);
    if let Some(sid) = live.session_id.lock().unwrap().clone() {
        live.host.session_cancel(&sid)?;
    }
    Ok(())
}

#[tauri::command]
fn permission_respond(state: State<AppState>, option_id: Option<String>) -> Result<(), String> {
    let live = state
        .live
        .lock()
        .unwrap()
        .clone()
        .ok_or("No live session")?;
    if let Some(id) = live.pending_perm.lock().unwrap().take() {
        live.host.respond_permission(&id, option_id.as_deref())?;
    }
    Ok(())
}

#[tauri::command]
fn git_diffs(state: State<AppState>) -> Result<Vec<diffs::DiffFile>, String> {
    let live = state
        .live
        .lock()
        .unwrap()
        .clone()
        .ok_or("No live session")?;
    match live.host.git_diffs() {
        Ok(v) => diffs::files_from_value(&v),
        Err(_) => Err("Could not load diffs".into()),
    }
}

#[tauri::command]
fn set_config_option(
    state: State<AppState>,
    config_id: String,
    value: String,
) -> Result<Value, String> {
    let live = state
        .live
        .lock()
        .unwrap()
        .clone()
        .ok_or("No live session")?;
    let sid = live
        .session_id
        .lock()
        .unwrap()
        .clone()
        .ok_or("No live session")?;
    live.host.set_config_option(&sid, &config_id, &value)
}

#[tauri::command]
fn retry_agent(app: AppHandle, state: State<AppState>) -> Result<Value, String> {
    let (cwd, sid) = {
        let live = state.live.lock().unwrap().clone();
        match live {
            Some(l) => {
                l.host.kill();
                (
                    l.cwd.lock().unwrap().clone(),
                    l.session_id.lock().unwrap().clone(),
                )
            }
            None => (None, None),
        }
    };
    *state.live.lock().unwrap() = None;
    let live = ensure_agent(&state, &app)?;
    if let (Some(cwd), Some(sid)) = (cwd, sid) {
        let v = live.host.session_load(&cwd, &sid)?;
        *live.session_id.lock().unwrap() = Some(sid);
        *live.cwd.lock().unwrap() = Some(cwd);
        return Ok(v);
    }
    Ok(json!({ "ok": true }))
}

fn shutdown(state: &AppState) {
    if let Some(live) = state.live.lock().unwrap().take() {
        deny_pending(&live);
        if let Some(sid) = live.session_id.lock().unwrap().clone() {
            let _ = live.host.session_cancel(&sid);
        }
        live.host.kill();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(AppState {
            live: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            app_status,
            sign_in,
            list_recents,
            open_folder,
            remove_recent,
            list_sessions_cmd,
            session_new,
            session_load,
            session_prompt,
            session_cancel,
            permission_respond,
            git_diffs,
            set_config_option,
            retry_agent
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                shutdown(&window.state::<AppState>());
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Grok Build");
}
