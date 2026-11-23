//! M0 read-only catalog + M1 Windows mGBA read-only connection.
//! All M1 commands are explicit user-initiated and contain no write/Hook operations.

use trainer_core::{GameCatalog, GameRegistry, SessionSnapshot};

#[tauri::command]
fn get_catalog() -> Result<GameCatalog, String> {
    GameRegistry::mock()
        .map(|registry| registry.snapshot())
        .map_err(|error| error.to_string())
}

#[cfg(windows)]
mod m1 {
    use super::*;
    use serde::Serialize;
    use std::sync::Mutex;
    use tauri::State;
    use trainer_mgba::RngSnapshot;
    use trainer_runtime_win::{list_mgba_targets, AttachedEmerald, MgbaTarget};

    pub struct RuntimeState(pub Mutex<Option<AttachedEmerald>>);

    #[derive(Serialize)]
    pub struct ConnectReceipt {
        pub session_id: String,
        pub game_code: String,
        pub language: String,
        pub verification: &'static str,
        pub read_only: bool,
    }

    #[tauri::command]
    pub fn list_mgba_processes() -> Result<Vec<MgbaTarget>, String> {
        list_mgba_targets().map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub async fn attach_emerald_read_only(
        process_id: u32,
        state: State<'_, RuntimeState>,
    ) -> Result<ConnectReceipt, String> {
        // Scanning process memory must never block the WebView UI thread.
        let mut attached = tauri::async_runtime::spawn_blocking(move || {
            AttachedEmerald::attach(process_id).map_err(|error| error.to_string())
        }).await.map_err(|error| error.to_string())??;
        let snapshot = attached.snapshot().map_err(|error| error.to_string())?;
        let receipt = ConnectReceipt {
            session_id: attached.session_id().to_owned(),
            game_code: snapshot.game_code,
            language: snapshot.language,
            verification: "partial_anchors_read_only",
            read_only: true,
        };
        *state.0.lock().map_err(|_| "Runtime state poisoned")? = Some(attached);
        Ok(receipt)
    }

    #[tauri::command]
    pub fn read_emerald_rng(
        state: State<'_, RuntimeState>,
    ) -> Result<RngSnapshot, String> {
        let mut guard = state.0.lock().map_err(|_| "Runtime state poisoned")?;
        let attached = guard.as_mut().ok_or("没有已连接的只读 Emerald 会话")?;
        match attached.snapshot() {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => {
                *guard = None; // fail closed; a stale handle cannot be reused.
                Err(error.to_string())
            }
        }
    }

    #[tauri::command]
    pub fn detach_emerald(state: State<'_, RuntimeState>) -> Result<(), String> {
        *state.0.lock().map_err(|_| "Runtime state poisoned")? = None;
        Ok(())
    }

    #[tauri::command]
    pub fn get_session_snapshot(state: State<'_, RuntimeState>) -> SessionSnapshot {
        let guard = state.0.lock().expect("session mutex poisoned");
        match guard.as_ref() {
            Some(attached) => SessionSnapshot {
                state: trainer_core::SessionState::AttachedReadOnly,
                read_only: true,
                session_id: Some(attached.session_id().to_owned()),
                game_id: Some("games.pokemon.emerald".into()),
                runtime_id: Some("runtimes.mgba.external".into()),
                profile_id: Some(format!("{}-Rev0-partial", attached.game_code())),
                message: "已连接只读 mGBA 会话；ROM 锚点是部分验证，不允许写入".into(),
            },
            None => SessionSnapshot::default(),
        }
    }
}

#[cfg(not(windows))]
#[tauri::command]
fn get_session_snapshot() -> SessionSnapshot {
    SessionSnapshot::default()
}

pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(windows)]
    let builder = builder
        .manage(m1::RuntimeState(std::sync::Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            get_catalog,
            m1::get_session_snapshot,
            m1::list_mgba_processes,
            m1::attach_emerald_read_only,
            m1::read_emerald_rng,
            m1::detach_emerald
        ]);
    #[cfg(not(windows))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        get_catalog, get_session_snapshot
    ]);
    builder
        .run(tauri::generate_context!())
        .expect("failed to run GameTrainer Studio");
}
