//! M9: Diamond/Pearl v0.8.14 source-backed ARM9 GDB read-only diagnostics.
//! This command never sends RSP M/X/c, does not pause or resume the emulator,
//! and cannot install/clear an Overlay11 Thumb trampoline.
use trainer_dp::{inspect, profile_for, profiles, DpProfile, DpReadOnlySnapshot, GuestReader};
use trainer_rsp::RspClient;

struct ReadOnlyGuest<'a>(&'a mut RspClient);
impl GuestReader for ReadOnlyGuest<'_> {
    fn read_guest(&mut self, address: u32, len: usize) -> Result<Vec<u8>, String> {
        if len == 0 || len > 72 {
            return Err("DP GDB read size exceeds source-backed limits".into());
        }
        self.0.read(address, len).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn m9_dp_profiles() -> Vec<DpProfile> {
    profiles()
}

#[tauri::command]
pub async fn m9_dp_read_arm9(
    arm9_port: u16,
    game_code: String,
    revision: u8,
) -> Result<DpReadOnlySnapshot, String> {
    if arm9_port < 1024 {
        return Err("ARM9 local debugger port must be >= 1024".into());
    }
    let profile = profile_for(&game_code, revision)
        .ok_or("Diamond/Pearl v0.8.14 unsupported ROM Code/Revision")?;
    if profile.write_allowed || profile.guest_exec_runtime_verified {
        return Err("M9 is intentionally read-only; profile cannot enable patches".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let mut rsp = RspClient::connect_local(arm9_port)
            .map_err(|e| format!("DeSmuME ARM9 GDB connection failed: {e}"))?;
        if !rsp.is_paused() {
            return Err("ARM9 GDB Stub did not report CPU stopped".into());
        }
        let pc_first = rsp.pc().map_err(|e| e.to_string())?;
        let pc_second = rsp.pc().map_err(|e| e.to_string())?;
        if pc_first != pc_second {
            return Err("ARM9 PC changed during supposedly stopped snapshot".into());
        }
        inspect(&mut ReadOnlyGuest(&mut rsp), &profile, pc_second)
            .map_err(|e| format!("DP read-only guest audit failed: {e}"))
        // Dropping the RSP connection does not send continue/resume.
    })
    .await
    .map_err(|e| format!("DP GDB snapshot worker failed: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diamond_and_pearl_are_separate_read_only_profiles() {
        assert_eq!(profiles().len(), 10);
        assert_eq!(profiles().iter().filter(|p| p.game == "Diamond").count(), 5);
        assert_eq!(profiles().iter().filter(|p| p.game == "Pearl").count(), 5);
        assert!(profiles().iter().all(|p| !p.write_allowed));
    }
}
