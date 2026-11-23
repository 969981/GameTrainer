//! M6: DeSmuME ARM9 GDB stub READ-ONLY stop/Overlay12 evidence.
//! A successful sample never unlocks memory writes. This command sends only
//! RSP '?', 'g' and fixed 'm' requests via trainer-rsp. It never sends M/X/c.
use trainer_hgss::{
    m6::{inspect, GuestReader, OverlayReport},
    profile_for,
};
use trainer_rsp::RspClient;

struct ReadOnlyRsp<'a>(&'a mut RspClient);
impl GuestReader for ReadOnlyRsp<'_> {
    fn read_guest(&mut self, address: u32, bytes: usize) -> Result<Vec<u8>, String> {
        if bytes > 34 {
            return Err("M6 limits all guest reads to 34 bytes".into());
        }
        self.0.read(address, bytes).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn m6_hgss_probe_arm9_stop(
    port: u16,
    game_code: String,
    revision: u8,
) -> Result<OverlayReport, String> {
    if port < 1024 {
        return Err("ARM9 debugger port must be >= 1024".into());
    }
    let profile = profile_for(&game_code, revision).ok_or("unknown HGSS game code or revision")?;
    if !profile.static_audit_1451 || profile.write_allowed {
        return Err("only 12 v1.4.5.1 statically audited HGSS variants are eligible".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let mut rsp =
            RspClient::connect_local(port).map_err(|e| format!("ARM9 GDB stop handshake: {e}"))?;
        if !rsp.is_paused() {
            return Err("debugger did not confirm stopped CPU".into());
        }
        // Two identical ARM9 PCs while RSP is stopped are a useful stop probe,
        // not proof that all emulator threads and JIT caches are frozen.
        let pc_before = rsp.pc().map_err(|e| e.to_string())?;
        let pc_after = rsp.pc().map_err(|e| e.to_string())?;
        if pc_before != pc_after {
            return Err("ARM9 PC changed while supposedly stopped".into());
        }
        inspect(&mut ReadOnlyRsp(&mut rsp), &profile, pc_after)
            .map_err(|e| format!("Overlay12 guest evidence failed: {e}"))
        // Drop the connection without sending a GDB continue command.
        // User controls emulator resume in the ARM9 debugger.
    })
    .await
    .map_err(|e| format!("M6 probe thread failed: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_m6_read_only_transport_uses_source_backed_profiles() {
        assert!(profile_for("IPKJ", 0).unwrap().static_audit_1451);
        assert!(!profile_for("IPKE", 0).unwrap().static_audit_1451);
    }
}
