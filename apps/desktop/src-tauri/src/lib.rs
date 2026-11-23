//! M0 read-only catalog + M1 Windows mGBA read-only connection.
//! All M1 commands are explicit user-initiated and contain no write/Hook operations.

use trainer_core::{GameCatalog, GameRegistry, SessionSnapshot};

mod m2;
mod m3;
mod m4;
mod m5;
mod m6;
mod m7;
mod m8;
mod m9;

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
    use trainer_rs::{retail_profiles, RsGame, RsProfile, RsSeedSnapshot};
    use trainer_runtime_win::{
        list_mgba_targets, AttachedEmerald, AttachedRubySapphire, MgbaTarget,
    };

    /// Exactly one active Gen3 emulator session. No game may reuse another
    /// game's memory mapping, profile, or read-only session epoch.
    enum AttachedGen3 {
        Emerald(AttachedEmerald),
        RubySapphire(AttachedRubySapphire),
    }
    pub struct RuntimeState(Mutex<Option<AttachedGen3>>);

    #[derive(Serialize)]
    pub struct ConnectReceipt {
        pub session_id: String,
        pub game_id: String,
        pub game_code: String,
        pub language: String,
        pub revision: u8,
        pub verification: &'static str,
        pub read_only: bool,
    }

    #[tauri::command]
    pub fn list_mgba_processes() -> Result<Vec<MgbaTarget>, String> {
        list_mgba_targets().map_err(|error| error.to_string())
    }

    #[tauri::command]
    pub fn list_rs_profiles() -> Vec<RsProfile> {
        retail_profiles()
    }

    #[tauri::command]
    pub async fn attach_emerald_read_only(
        process_id: u32,
        state: State<'_, RuntimeState>,
    ) -> Result<ConnectReceipt, String> {
        let mut attached = tauri::async_runtime::spawn_blocking(move || {
            AttachedEmerald::attach(process_id).map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())??;
        let snapshot = attached.snapshot().map_err(|error| error.to_string())?;
        let receipt = ConnectReceipt {
            session_id: attached.session_id().to_owned(),
            game_id: "games.pokemon.emerald".into(),
            game_code: snapshot.game_code,
            language: snapshot.language,
            revision: 0,
            verification: "partial_anchors_read_only",
            read_only: true,
        };
        *state.0.lock().map_err(|_| "Runtime state poisoned")? =
            Some(AttachedGen3::Emerald(attached));
        Ok(receipt)
    }

    #[tauri::command]
    pub async fn attach_rs_read_only(
        process_id: u32,
        game_id: String,
        state: State<'_, RuntimeState>,
    ) -> Result<ConnectReceipt, String> {
        let game = match game_id.as_str() {
            "games.pokemon.ruby" => RsGame::Ruby,
            "games.pokemon.sapphire" => RsGame::Sapphire,
            _ => return Err("只支持 Ruby/Sapphire 的只读目标".into()),
        };
        let mut attached = tauri::async_runtime::spawn_blocking(move || {
            AttachedRubySapphire::attach(process_id, game).map_err(|e| e.to_string())
        })
        .await
        .map_err(|error| error.to_string())??;
        let snapshot = attached.snapshot().map_err(|error| error.to_string())?;
        if snapshot.game_id != game_id {
            return Err("目标游戏和请求的版本不符".into());
        }
        let receipt = ConnectReceipt {
            session_id: attached.session_id().to_owned(),
            game_id: snapshot.game_id,
            game_code: snapshot.game_code,
            language: snapshot.language,
            revision: snapshot.revision,
            verification: "header_revision_boot_anchor_only",
            read_only: true,
        };
        *state.0.lock().map_err(|_| "Runtime state poisoned")? =
            Some(AttachedGen3::RubySapphire(attached));
        Ok(receipt)
    }

    #[tauri::command]
    pub fn read_emerald_rng(state: State<'_, RuntimeState>) -> Result<RngSnapshot, String> {
        let mut guard = state.0.lock().map_err(|_| "Runtime state poisoned")?;
        let Some(AttachedGen3::Emerald(attached)) = guard.as_mut() else {
            return Err("没有已连接的 Emerald 只读会话".into());
        };
        match attached.snapshot() {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => {
                *guard = None; // session invalidated: fail closed
                Err(error.to_string())
            }
        }
    }

    #[tauri::command]
    pub fn read_rs_rng(state: State<'_, RuntimeState>) -> Result<RsSeedSnapshot, String> {
        let mut guard = state.0.lock().map_err(|_| "Runtime state poisoned")?;
        let Some(AttachedGen3::RubySapphire(attached)) = guard.as_mut() else {
            return Err("没有已连接的 Ruby/Sapphire 只读会话".into());
        };
        match attached.snapshot() {
            Ok(snapshot) => Ok(snapshot),
            Err(error) => {
                *guard = None;
                Err(error.to_string())
            }
        }
    }

    #[tauri::command]
    pub fn detach_emerald(state: State<'_, RuntimeState>) -> Result<(), String> {
        // Legacy IPC name retained. It disconnects any Gen3 game, not just Emerald.
        *state.0.lock().map_err(|_| "Runtime state poisoned")? = None;
        Ok(())
    }

    #[tauri::command]
    pub fn get_session_snapshot(state: State<'_, RuntimeState>) -> SessionSnapshot {
        let guard = state.0.lock().expect("session mutex poisoned");
        match guard.as_ref() {
            Some(AttachedGen3::Emerald(attached)) => SessionSnapshot {
                state: trainer_core::SessionState::AttachedReadOnly,
                read_only: true,
                session_id: Some(attached.session_id().to_owned()),
                game_id: Some("games.pokemon.emerald".into()),
                runtime_id: Some("runtimes.mgba.external".into()),
                profile_id: Some(format!("{}-Rev0-partial", attached.game_code())),
                message: "Emerald 只读连接：部分 ROM 锚点，禁止任何游戏写入".into(),
            },
            Some(AttachedGen3::RubySapphire(attached)) => SessionSnapshot {
                state: trainer_core::SessionState::AttachedReadOnly,
                read_only: true,
                session_id: Some(attached.session_id().to_owned()),
                game_id: Some(attached.game_id().to_owned()),
                runtime_id: Some("runtimes.mgba.external".into()),
                profile_id: Some(format!(
                    "{}_R{}-anchor-only",
                    attached.game_code(),
                    attached.revision()
                )),
                message: format!(
                    "{} 只读连接：仅 Header/Rev/16-byte Anchor，禁止游戏写入",
                    attached.game_id()
                ),
            },
            None => SessionSnapshot::default(),
        }
    }

    pub fn fresh_state() -> RuntimeState {
        RuntimeState(Mutex::new(None))
    }
}

#[cfg(not(windows))]
#[tauri::command]
fn get_session_snapshot() -> SessionSnapshot {
    SessionSnapshot::default()
}

pub fn run() {
    let builder = tauri::Builder::default().setup(|app| {
        m2::install(app);
        m3::install(app);
        m4::install(app);
        m5::install(app);
        Ok(())
    });
    #[cfg(windows)]
    let builder = builder
        .manage(m1::fresh_state())
        .invoke_handler(tauri::generate_handler![
            get_catalog,
            m1::get_session_snapshot,
            m1::list_mgba_processes,
            m1::list_rs_profiles,
            m1::attach_emerald_read_only,
            m1::attach_rs_read_only,
            m1::read_emerald_rng,
            m1::read_rs_rng,
            m1::detach_emerald,
            m2::m2_fixture_status,
            m2::m2_start_fixture,
            m2::m2_preview_fixture_seed,
            m2::m2_cancel_fixture_preview,
            m2::m2_apply_fixture_preview,
            m2::m2_restore_fixture_patch,
            m2::m2_list_action_history,
            m3::m3_live_status,
            m3::m3_live_connect,
            m3::m3_preview_live_capture,
            m3::m3_preview_live_crit,
            m3::m3_apply_live_preview,
            m3::m3_restore_live_patch,
            m3::m3_cancel_live_preview,
            m3::m3_disconnect_live,
            m3::m3_live_history,
            m4::m4_frlg_profiles,
            m4::m4_frlg_status,
            m4::m4_frlg_connect,
            m4::m4_frlg_preview,
            m4::m4_frlg_apply,
            m4::m4_frlg_arm_wild,
            m4::m4_frlg_restore,
            m4::m4_frlg_cancel,
            m4::m4_frlg_disconnect,
            m4::m4_frlg_pause,
            m4::m4_frlg_resume,
            m4::m4_frlg_mailbox,
            m4::m4_frlg_history,
            m5::m5_hgss_profiles,
            m5::m5_list_desmume,
            m5::m5_attach_hgss,
            m5::m5_hgss_snapshot,
            m5::m5_detach_hgss,
            m6::m6_hgss_probe_arm9_stop,
            m7::m7_hgss_preflight,
            m8::m8_hgss_core_rom_identity,
            m9::m9_dp_profiles,
            m9::m9_dp_read_arm9
        ]);
    #[cfg(not(windows))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        get_catalog,
        get_session_snapshot,
        m2::m2_fixture_status,
        m2::m2_start_fixture,
        m2::m2_preview_fixture_seed,
        m2::m2_cancel_fixture_preview,
        m2::m2_apply_fixture_preview,
        m2::m2_restore_fixture_patch,
        m2::m2_list_action_history,
        m3::m3_live_status,
        m3::m3_live_connect,
        m3::m3_preview_live_capture,
        m3::m3_preview_live_crit,
        m3::m3_apply_live_preview,
        m3::m3_restore_live_patch,
        m3::m3_cancel_live_preview,
        m3::m3_disconnect_live,
        m3::m3_live_history,
        m4::m4_frlg_profiles,
        m4::m4_frlg_status,
        m4::m4_frlg_connect,
        m4::m4_frlg_preview,
        m4::m4_frlg_apply,
        m4::m4_frlg_arm_wild,
        m4::m4_frlg_restore,
        m4::m4_frlg_cancel,
        m4::m4_frlg_disconnect,
        m4::m4_frlg_pause,
        m4::m4_frlg_resume,
        m4::m4_frlg_mailbox,
        m4::m4_frlg_history,
        m5::m5_hgss_profiles,
        m5::m5_list_desmume,
        m5::m5_attach_hgss,
        m5::m5_hgss_snapshot,
        m5::m5_detach_hgss,
        m6::m6_hgss_probe_arm9_stop,
        m7::m7_hgss_preflight,
        m8::m8_hgss_core_rom_identity,
            m9::m9_dp_profiles,
            m9::m9_dp_read_arm9
    ]);
    builder
        .run(tauri::generate_context!())
        .expect("failed to run GameTrainer Studio");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readonly_catalog_command_returns_rust_registry() {
        let catalog = get_catalog().expect("valid Rust Mock registry");
        assert_eq!(catalog.games.len(), 11);
        assert!(catalog.read_only);
        assert!(catalog
            .games
            .iter()
            .any(|game| game.game_id == "games.pokemon.ruby"));
        assert!(catalog
            .games
            .iter()
            .any(|game| game.game_id == "games.pokemon.sapphire"));
        assert_eq!(catalog.source, "mock");
    }
}
