//! Trusted Tauri-only live mGBA RSP session. All write plans come from Rust.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{Manager, State};
use trainer_live::{LiveEditor, LiveIdentity, LivePreview, LiveReceipt};
use trainer_storage::{ActionEvent, AuditStore};

pub struct LiveState {
    inner: Arc<Mutex<Option<LiveEditor>>>,
    audit_file: PathBuf,
}
pub fn install(app: &mut tauri::App) {
    let folder = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("GameTrainer"));
    // If this directory cannot persist audit rows, connect will fail closed.
    app.manage(LiveState {
        inner: Arc::new(Mutex::new(None)),
        audit_file: folder.join("m3-live-action-history.sqlite3"),
    });
}
#[tauri::command]
pub fn m3_live_status(state: State<'_, LiveState>) -> Result<Option<LiveIdentity>, String> {
    let lock = state.inner.lock().map_err(|_| "M3 状态锁损坏")?;
    Ok(lock.as_ref().map(LiveEditor::status))
}
#[tauri::command]
pub async fn m3_live_connect(
    port: u16,
    state: State<'_, LiveState>,
) -> Result<LiveIdentity, String> {
    let session = Arc::clone(&state.inner);
    let file = state.audit_file.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = session.lock().map_err(|_| "M3 状态锁损坏".to_string())?;
        if guard.as_ref().is_some_and(LiveEditor::has_active) {
            return Err("已有真实补丁，必须先恢复，不能切换 GDB 会话".into());
        }
        let audit = AuditStore::open_file(&file).map_err(|e| e.to_string())?;
        let editor = LiveEditor::connect(port, audit).map_err(|e| e.to_string())?;
        let id = editor.status();
        *guard = Some(editor);
        Ok(id)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn m3_preview_live_capture(state: State<'_, LiveState>) -> Result<LivePreview, String> {
    let session = Arc::clone(&state.inner);
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = session.lock().map_err(|_| "M3 锁损坏".to_string())?;
        guard
            .as_mut()
            .ok_or("请先通过 mGBA GDB 连接原版 ROM".to_string())?
            .preview_capture()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn m3_preview_live_crit(
    template: Vec<u8>,
    state: State<'_, LiveState>,
) -> Result<LivePreview, String> {
    let session = Arc::clone(&state.inner);
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = session.lock().map_err(|_| "M3 锁损坏".to_string())?;
        guard
            .as_mut()
            .ok_or("请先连接原版 ROM".to_string())?
            .preview_crit(&template)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn m3_apply_live_preview(
    token: String,
    confirmation: String,
    state: State<'_, LiveState>,
) -> Result<LiveReceipt, String> {
    // IPC direct calls need the same positive confirmation as the visible UI.
    if confirmation != "我已备份存档，仅在离线单机测试" {
        return Err("需要明确确认离线单机与存档备份".into());
    }
    let session = Arc::clone(&state.inner);
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = session.lock().map_err(|_| "M3 锁损坏".to_string())?;
        guard
            .as_mut()
            .ok_or("未连接".to_string())?
            .apply(&token)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn m3_restore_live_patch(state: State<'_, LiveState>) -> Result<LiveReceipt, String> {
    let session = Arc::clone(&state.inner);
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = session.lock().map_err(|_| "M3 锁损坏".to_string())?;
        guard
            .as_mut()
            .ok_or("未连接".to_string())?
            .restore()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn m3_cancel_live_preview(state: State<'_, LiveState>) -> Result<(), String> {
    let mut guard = state.inner.lock().map_err(|_| "M3 锁损坏")?;
    guard
        .as_mut()
        .ok_or("未连接")?
        .cancel_preview()
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn m3_disconnect_live(state: State<'_, LiveState>) -> Result<(), String> {
    let mut guard = state.inner.lock().map_err(|_| "M3 锁损坏")?;
    if let Some(active) = guard.as_mut() {
        if active.has_active() {
            return Err("不能在真正补丁仍活跃/恢复失败时断开".into());
        }
        active.disconnect().map_err(|e| e.to_string())?;
    }
    *guard = None;
    Ok(())
}
#[tauri::command]
pub fn m3_live_history(state: State<'_, LiveState>) -> Result<Vec<ActionEvent>, String> {
    let audit = AuditStore::open_file(&state.audit_file).map_err(|e| e.to_string())?;
    audit.list("live", 100).map_err(|e| e.to_string())
}
