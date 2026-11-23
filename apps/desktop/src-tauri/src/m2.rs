//! Explicit, isolated M2 fixture lab. Never holds a Windows process handle.
//! SQLite logs simulated actions in a separate local database with a fixture label.

use serde::Serialize;
use std::{path::PathBuf, sync::Mutex, time::Duration};
use tauri::{Manager, State};
use trainer_patch::{
    FixtureSnapshot, FixtureTarget, PatchManager, PatchPreview, PatchReceipt, PatchSpec,
};
use trainer_storage::{ActionEvent, AuditStore};

struct Lab {
    engine: PatchManager,
    target: Option<FixtureTarget>,
}
pub struct LabState {
    lab: Mutex<Option<Lab>>,
    unavailable_reason: Option<String>,
}
#[derive(Serialize)]
pub struct LabStatus {
    pub connected: bool,
    pub fixture: Option<FixtureSnapshot>,
    pub unresolved_fixture_intents: u64,
    pub readonly_live_runtime: bool,
    pub audit_scope: &'static str,
}

/// SQLite file is created in app-local data; cannot silently fallback to memory
/// if the persistent audit cannot open. All M2 mutations are blocked in that case.
pub fn install(app: &mut tauri::App) {
    let opened = (|| -> Result<Lab, String> {
        let directory: PathBuf = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
        let database = AuditStore::open_file(&directory.join("m2-fixture-action-history.sqlite3"))
            .map_err(|e| e.to_string())?;
        Ok(Lab {
            engine: PatchManager::new(database, Duration::from_secs(30)),
            target: None,
        })
    })();
    let state = match opened {
        Ok(lab) => LabState { lab: Mutex::new(Some(lab)), unavailable_reason: None },
        Err(error) => LabState { lab: Mutex::new(None), unavailable_reason: Some(error) },
    };
    app.manage(state);
}

fn with_lab<T>(state: &State<'_, LabState>, action: impl FnOnce(&mut Lab) -> Result<T, String>) -> Result<T, String> {
    let mut guard = state.lab.lock().map_err(|_| "补丁实验室状态已损坏")?;
    let lab = guard.as_mut().ok_or_else(|| {
        state.unavailable_reason.clone().unwrap_or_else(|| "SQLite 审计未就绪".into())
    })?;
    action(lab)
}
#[tauri::command]
pub fn m2_fixture_status(state: State<'_, LabState>) -> Result<LabStatus, String> {
    with_lab(&state, |lab| Ok(LabStatus {
        connected: lab.target.is_some(),
        fixture: lab.target.as_ref().map(|target| target.snapshot(lab.engine.active())),
        unresolved_fixture_intents: lab.engine.audit().unresolved_count("fixture").map_err(|e| e.to_string())?,
        readonly_live_runtime: true,
        audit_scope: "fixture",
    }))
}
#[tauri::command]
pub fn m2_start_fixture(state: State<'_, LabState>) -> Result<LabStatus, String> {
    with_lab(&state, |lab| {
        // Synthetic memory starts clean. Never auto-replay or restore SQLite entries.
        lab.engine.clear_fixture_session();
        lab.target = Some(FixtureTarget::new());
        Ok(LabStatus {
            connected: true,
            fixture: lab.target.as_ref().map(|target| target.snapshot(lab.engine.active())),
            unresolved_fixture_intents: lab.engine.audit().unresolved_count("fixture").map_err(|e| e.to_string())?,
            readonly_live_runtime: true,
            audit_scope: "fixture",
        })
    })
}
#[tauri::command]
pub fn m2_preview_fixture_seed(state: State<'_, LabState>, target_seed: u32) -> Result<PatchPreview, String> {
    with_lab(&state, |lab| {
        let target = lab.target.as_ref().ok_or("请先开始隔离测试会话")?;
        lab.engine.preview(target, PatchSpec::emerald_fixture_seed(target_seed)).map_err(|e| e.to_string())
    })
}
#[tauri::command]
pub fn m2_cancel_fixture_preview(state: State<'_, LabState>, preview_id: String) -> Result<(), String> {
    with_lab(&state, |lab| lab.engine.cancel_preview(&preview_id).map_err(|e| e.to_string()))
}
#[tauri::command]
pub fn m2_apply_fixture_preview(state: State<'_, LabState>, preview_id: String) -> Result<PatchReceipt, String> {
    with_lab(&state, |lab| {
        let target = lab.target.as_mut().ok_or("请先开始隔离测试会话")?;
        lab.engine.apply(target, &preview_id).map_err(|e| e.to_string())
    })
}
#[tauri::command]
pub fn m2_restore_fixture_patch(state: State<'_, LabState>, owner_id: String) -> Result<PatchReceipt, String> {
    with_lab(&state, |lab| {
        let target = lab.target.as_mut().ok_or("请先开始隔离测试会话")?;
        lab.engine.restore(target, &owner_id).map_err(|e| e.to_string())
    })
}
#[tauri::command]
pub fn m2_list_action_history(state: State<'_, LabState>, environment: String) -> Result<Vec<ActionEvent>, String> {
    if !matches!(environment.as_str(), "fixture" | "live") {
        return Err("操作历史仅允许 fixture / live".into());
    }
    with_lab(&state, |lab| lab.engine.audit().list(&environment, 100).map_err(|e| e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lab_fixture_flow_changes_only_in_memory() {
        let mut lab = Lab {
            engine: PatchManager::new(AuditStore::open_memory().unwrap(), Duration::from_secs(30)),
            target: Some(FixtureTarget::new()),
        };
        let target = lab.target.as_mut().unwrap();
        let before = target.snapshot(Vec::new()).seed;
        let preview = lab.engine.preview(target, PatchSpec::emerald_fixture_seed(0xDEAD_BEEF)).unwrap();
        assert_eq!(target.snapshot(Vec::new()).seed, before);
        let receipt = lab.engine.apply(target, &preview.preview_id).unwrap();
        assert_eq!(target.snapshot(Vec::new()).seed, 0xDEAD_BEEF);
        lab.engine.restore(target, &receipt.owner_id).unwrap();
        assert_eq!(target.snapshot(Vec::new()).seed, before);
        assert!(lab.engine.audit().list("live", 10).unwrap().is_empty());
    }
}
