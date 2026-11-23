//! M0 only exposes a read-only mock catalog. No hook, attach or memory commands.
use trainer_core::{GameCatalog, GameRegistry, SessionSnapshot};

#[tauri::command]
fn get_catalog() -> Result<GameCatalog, String> {
    GameRegistry::mock()
        .map(|registry| registry.snapshot())
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_session_snapshot() -> SessionSnapshot {
    SessionSnapshot::default()
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_catalog, get_session_snapshot])
        .run(tauri::generate_context!())
        .expect("failed to run GameTrainer Studio");
}
