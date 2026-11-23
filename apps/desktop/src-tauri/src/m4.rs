//! FireRed/LeafGreen M4: trusted only through source-verified Rust profiles.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{Manager, State};
use trainer_frlg::{
    live::{Diff, Editor, Receipt, Status, WildMailbox},
    retail_profiles, FrlgRomProfile,
};
use trainer_storage::{ActionEvent, AuditStore};

pub struct FrlgState {
    inner: Arc<Mutex<Option<Editor>>>,
    audit_file: Option<PathBuf>,
}
pub fn install(app: &mut tauri::App) {
    let audit_file = app
        .path()
        .app_local_data_dir()
        .ok()
        .map(|folder| folder.join("m4-frlg-live-history.sqlite3"));
    app.manage(FrlgState {
        inner: Arc::new(Mutex::new(None)),
        audit_file,
    });
}

#[tauri::command]
pub fn m4_frlg_profiles() -> Result<Vec<FrlgRomProfile>, String> {
    retail_profiles().map_err(|e| e.to_string())
}
#[tauri::command]
pub fn m4_frlg_status(state: State<'_, FrlgState>) -> Result<Option<Status>, String> {
    let guard = state.inner.lock().map_err(|_| "FRLG 状态锁异常")?;
    Ok(guard.as_ref().map(Editor::status))
}
#[tauri::command]
pub async fn m4_frlg_connect(
    port: u16,
    state: State<'_, FrlgState>,
    rs_state: State<'_, crate::m3::LiveState>,
) -> Result<Status, String> {
    if crate::m3::m3_live_status(rs_state)?.is_some() {
        return Err("红蓝宝石 M3 已占用 GDB 会话，请先安全断开".into());
    }
    let inner = Arc::clone(&state.inner);
    let file = state
        .audit_file
        .as_ref()
        .ok_or("无法创建持久化 SQLite 审计目录，拒绝真实写入")?
        .clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = inner.lock().map_err(|_| "FRLG 状态锁异常".to_owned())?;
        if guard.as_ref().is_some_and(Editor::has_owner) {
            return Err("已有 FRLG 活跃补丁，必须验证恢复后才能重新连接".into());
        }
        let audit = AuditStore::open_file(&file).map_err(|e| e.to_string())?;
        let editor = Editor::connect(port, audit).map_err(|e| e.to_string())?;
        let status = editor.status();
        *guard = Some(editor);
        Ok(status)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn m4_frlg_preview(
    feature: String,
    template: Option<Vec<u8>>,
    state: State<'_, FrlgState>,
) -> Result<Diff, String> {
    let inner = Arc::clone(&state.inner);
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = inner.lock().map_err(|_| "FRLG 状态锁异常".to_owned())?;
        guard
            .as_mut()
            .ok_or("请先连接合法原版火红/叶绿 ROM".to_string())?
            .preview(&feature, template.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn m4_frlg_apply(
    token: String,
    confirmation: String,
    state: State<'_, FrlgState>,
) -> Result<Receipt, String> {
    if confirmation != "已备份存档并确认仅供离线单机实验" {
        return Err("必须明确确认备份和仅限离线单机实验".into());
    }
    let inner = Arc::clone(&state.inner);
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = inner.lock().map_err(|_| "FRLG 状态锁异常".to_owned())?;
        guard
            .as_mut()
            .ok_or("未连接 FRLG GDB 会话".to_string())?
            .apply(&token)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn m4_frlg_arm_wild(
    target_seed: u32,
    confirmation: String,
    state: State<'_, FrlgState>,
) -> Result<WildMailbox, String> {
    if confirmation != "已备份存档并确认仅供离线单机实验" {
        return Err("需要独立确认一次性野生 Hook 注入".into());
    }
    let inner = Arc::clone(&state.inner);
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = inner.lock().map_err(|_| "FRLG 状态锁异常".to_owned())?;
        guard
            .as_mut()
            .ok_or("未连接 FRLG GDB 会话".to_string())?
            .arm_wild_once(target_seed)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn m4_frlg_restore(state: State<'_, FrlgState>) -> Result<Receipt, String> {
    let inner = Arc::clone(&state.inner);
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = inner.lock().map_err(|_| "FRLG 状态锁异常".to_owned())?;
        guard
            .as_mut()
            .ok_or("没有 FRLG 会话".to_string())?
            .restore()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn m4_frlg_cancel(state: State<'_, FrlgState>) -> Result<(), String> {
    let mut guard = state.inner.lock().map_err(|_| "FRLG 状态锁异常")?;
    guard
        .as_mut()
        .ok_or("没有 FRLG 会话")?
        .cancel_preview()
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn m4_frlg_disconnect(state: State<'_, FrlgState>) -> Result<(), String> {
    let mut guard = state.inner.lock().map_err(|_| "FRLG 状态锁异常")?;
    if let Some(editor) = guard.as_mut() {
        editor.disconnect().map_err(|e| e.to_string())?;
    }
    *guard = None;
    Ok(())
}
#[tauri::command]
pub async fn m4_frlg_pause(state: State<'_, FrlgState>) -> Result<Status, String> {
    let inner = Arc::clone(&state.inner);
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = inner.lock().map_err(|_| "FRLG 状态锁异常".to_owned())?;
        let editor = guard.as_mut().ok_or("没有 FRLG 会话")?;
        editor.pause().map_err(|e| e.to_string())?;
        Ok(editor.status())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn m4_frlg_resume(state: State<'_, FrlgState>) -> Result<Status, String> {
    let mut guard = state.inner.lock().map_err(|_| "FRLG 状态锁异常")?;
    let editor = guard.as_mut().ok_or("没有 FRLG 会话")?;
    editor.resume().map_err(|e| e.to_string())?;
    Ok(editor.status())
}
#[tauri::command]
pub fn m4_frlg_mailbox(state: State<'_, FrlgState>) -> Result<WildMailbox, String> {
    let mut guard = state.inner.lock().map_err(|_| "FRLG 状态锁异常")?;
    guard
        .as_mut()
        .ok_or("没有 FRLG 会话")?
        .wild_mailbox()
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn m4_frlg_history(state: State<'_, FrlgState>) -> Result<Vec<ActionEvent>, String> {
    let path = state.audit_file.as_ref().ok_or("审计目录不可用")?;
    let audit = AuditStore::open_file(path).map_err(|e| e.to_string())?;
    audit.list("live-frlg", 100).map_err(|e| e.to_string())
}
