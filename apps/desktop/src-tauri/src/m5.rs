//! M5 NDS / DeSmuME read-only bridge. It intentionally has no write commands.
use std::sync::Mutex;
use tauri::State;
use trainer_hgss::{retail_profiles, HgssProfile};

#[cfg(windows)]
use trainer_runtime_win::{
    list_desmume_targets, AttachedHgss, HgssReadOnlySnapshot, MgbaTarget,
};

#[cfg(windows)]
pub struct M5State(Mutex<Option<AttachedHgss>>);
#[cfg(not(windows))]
pub struct M5State(Mutex<()>);

pub fn install(app: &mut tauri::App) {
    #[cfg(windows)]
    app.manage(M5State(Mutex::new(None)));
    #[cfg(not(windows))]
    app.manage(M5State(Mutex::new(())));
}

#[tauri::command]
pub fn m5_hgss_profiles() -> Vec<HgssProfile> {
    retail_profiles()
}

#[cfg(windows)]
#[tauri::command]
pub fn m5_list_desmume() -> Result<Vec<MgbaTarget>, String> {
    list_desmume_targets().map_err(|e| e.to_string())
}
#[cfg(not(windows))]
#[tauri::command]
pub fn m5_list_desmume() -> Result<Vec<serde_json::Value>, String> {
    Err("DeSmuME external provider currently supports Windows only".into())
}

#[cfg(windows)]
#[tauri::command]
pub async fn m5_attach_hgss(process_id: u32, game_id: String, state: State<'_, M5State>)
    -> Result<HgssReadOnlySnapshot, String>
{
    let attached = tauri::async_runtime::spawn_blocking(move ||
        AttachedHgss::attach(process_id, &game_id).map_err(|e| e.to_string()))
        .await.map_err(|e| e.to_string())??;
    let receipt = attached.snapshot().map_err(|e| e.to_string())?;
    *state.0.lock().map_err(|_| "M5 session lock poisoned")? = Some(attached);
    Ok(receipt)
}
#[cfg(not(windows))]
#[tauri::command]
pub async fn m5_attach_hgss(_process_id: u32, _game_id: String)
    -> Result<serde_json::Value, String>
{
    Err("M5 read-only attach requires Windows DeSmuME".into())
}

#[cfg(windows)]
#[tauri::command]
pub fn m5_hgss_snapshot(state: State<'_, M5State>) -> Result<HgssReadOnlySnapshot, String> {
    let mut guard = state.0.lock().map_err(|_| "M5 session lock poisoned")?;
    let result = guard.as_ref().ok_or("no attached HGSS DeSmuME session")?.snapshot();
    if result.is_err() { *guard = None; }
    result.map_err(|e| e.to_string())
}
#[cfg(not(windows))]
#[tauri::command]
pub fn m5_hgss_snapshot() -> Result<serde_json::Value, String> {
    Err("no NDS session on this platform".into())
}

#[cfg(windows)]
#[tauri::command]
pub fn m5_detach_hgss(state: State<'_, M5State>) -> Result<(), String> {
    *state.0.lock().map_err(|_| "M5 session lock poisoned")? = None;
    Ok(())
}
#[cfg(not(windows))]
#[tauri::command]
pub fn m5_detach_hgss() -> Result<(), String> { Ok(()) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_hgss_profiles_are_read_only() {
        assert_eq!(m5_hgss_profiles().len(), 14);
        assert!(m5_hgss_profiles().iter().all(|p| !p.write_allowed));
    }
}
