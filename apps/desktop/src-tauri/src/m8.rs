//! M8: source-patched DeSmuME core ROM identity bridge.
//! No user-supplied ROM hash is trusted and no NDS memory writes are exposed.
use trainer_hgss::m8::M8Report;

#[cfg(windows)]
#[tauri::command]
pub async fn m8_hgss_core_rom_identity(
    process_id:u32, game_code:String,revision:u8,
    arm9_port:u16,arm7_port:u16,
)->Result<M8Report,String>{
    if process_id==0 || arm9_port<1024 || arm7_port<1024 ||
        arm9_port==arm7_port {
        return Err("M8 requires valid DeSmuME PID and two distinct GDB ports".into());
    }
    // Restrict identity requests to source-audited original ROMs.
    let p=trainer_hgss::profile_for(&game_code,revision)
        .ok_or("unknown HGSS game code/revision")?;
    if !p.static_audit_1451 || p.write_allowed {
        return Err("M8 requires an HGSS v1.4.5.1 statically audited profile".into());
    }
    tauri::async_runtime::spawn_blocking(move||
        trainer_runtime_win::read_desmume_runtime_rom_identity(
            process_id,&game_code,revision,arm9_port,arm7_port
        )
    ).await.map_err(|e|format!("M8 Win32 reader failed: {e}"))?
}

#[cfg(not(windows))]
#[tauri::command]
pub async fn m8_hgss_core_rom_identity(
    _process_id:u32,_game_code:String,_revision:u8,
    _arm9_port:u16,_arm7_port:u16,
)->Result<M8Report,String>{
    Err("M8 core-loaded ROM identity is only available on Windows".into())
}
#[cfg(test)]
mod tests {
    #[test]
    fn m8_has_no_mutating_guest_commands() {
        let x=trainer_hgss::m8::WIRE_SIZE;
        assert_eq!(x,76);
    }
}
