//! DP M10 Windows DeSmuME identity bridge and two independent CPU observations.
//! No RSP M/X write, c continue, Guest injection or CPU synchronization.
//! Only an OS PID-bound ROM digest may be called identity evidence; two stop
//! replies and a JIT-at-load flag do NOT attest atomic CPU/JIT consistency.
use trainer_dp::m10::{Identity, RuntimeAudit};

#[cfg(windows)]
use trainer_dp::m10::{runtime_audit, same_loaded_rom_session, CpuObservation};
#[cfg(windows)]
use trainer_dp::{inspect, profile_for, DpProfile, DpReadOnlySnapshot, GuestReader};
#[cfg(windows)]
use trainer_rsp::RspClient;

#[cfg(windows)]
struct Reader<'a>(&'a mut RspClient);
#[cfg(windows)]
impl GuestReader for Reader<'_> {
    fn read_guest(&mut self, at: u32, len: usize) -> Result<Vec<u8>, String> {
        if len == 0 || len > 72 {
            return Err("M10 refuses unbounded Guest read".into());
        }
        self.0.read(at, len).map_err(|e| e.to_string())
    }
}

#[cfg(windows)]
fn observe_cpu(
    cpu: &str,
    port: u16,
    profile: Option<&DpProfile>,
) -> (CpuObservation, Option<DpReadOnlySnapshot>) {
    let mut gdb = match RspClient::connect_local(port) {
        Ok(x) => x,
        Err(e) => {
            return (
                CpuObservation::absent(cpu, port, format!("GDB connect/stop handshake: {e}")),
                None,
            )
        }
    };
    if !gdb.is_paused() {
        return (
            CpuObservation::absent(cpu, port, "CPU not stopped".into()),
            None,
        );
    }
    let first = match gdb.pc() {
        Ok(x) => x,
        Err(e) => {
            return (
                CpuObservation::absent(cpu, port, format!("first PC read: {e}")),
                None,
            )
        }
    };
    let second = match gdb.pc() {
        Ok(x) => x,
        Err(e) => {
            return (
                CpuObservation::absent(cpu, port, format!("second PC read: {e}")),
                None,
            )
        }
    };
    let unchanged = first == second;
    let snapshot = if unchanged {
        profile.and_then(|p| inspect(&mut Reader(&mut gdb), p, second).ok())
    } else {
        None
    };
    (
        CpuObservation {
            cpu: cpu.into(),
            port,
            stop_reply_observed: true,
            first_pc: Some(first),
            second_pc: Some(second),
            error: if unchanged {
                None
            } else {
                Some("PC changed during GDB stop observation".into())
            },
        },
        snapshot,
    )
}

fn checked_profile(code: &str, revision: u8) -> Result<(), String> {
    let p = trainer_dp::profile_for(code, revision)
        .ok_or("M10 Diamond/Pearl Code/Revision is not in v0.8.14 source profiles")?;
    if p.write_allowed || p.guest_exec_runtime_verified {
        return Err("unsafe DP profile flags; M10 is read-only".into());
    }
    Ok(())
}

#[cfg(windows)]
#[tauri::command]
pub async fn m10_dp_core_identity(
    process_id: u32,
    game_code: String,
    revision: u8,
    arm9_port: u16,
    arm7_port: u16,
) -> Result<Identity, String> {
    checked_profile(&game_code, revision)?;
    tauri::async_runtime::spawn_blocking(move || {
        trainer_runtime_win::read_dp_core_identity(
            process_id, &game_code, revision, arm9_port, arm7_port,
        )
        .map(|(_, r)| r)
    })
    .await
    .map_err(|e| format!("M10 Windows identity worker: {e}"))?
}

#[cfg(not(windows))]
#[tauri::command]
pub async fn m10_dp_core_identity(
    _process_id: u32,
    _game_code: String,
    _revision: u8,
    _arm9_port: u16,
    _arm7_port: u16,
) -> Result<Identity, String> {
    Err("M10 core ROM identity requires Windows DeSmuME bridge".into())
}

#[cfg(windows)]
#[tauri::command]
pub async fn m10_dp_dual_cpu_overlay11_readonly(
    process_id: u32,
    game_code: String,
    revision: u8,
    arm9_port: u16,
    arm7_port: u16,
) -> Result<RuntimeAudit, String> {
    checked_profile(&game_code, revision)?;
    tauri::async_runtime::spawn_blocking(move || {
        let (before, identity) = trainer_runtime_win::read_dp_core_identity(
            process_id, &game_code, revision, arm9_port, arm7_port,
        )?;
        // Identity failures are returned explicitly, never silently upgraded
        // using the GDB headers. A source snapshot may still support a
        // *read-only* investigation when the complete ROM reference is absent.
        let profile = profile_for(&game_code, revision)
            .ok_or("DP profile unavailable during native snapshot")?;
        let (arm9, snapshot) = observe_cpu("ARM9", arm9_port, Some(&profile));
        let (arm7, _) = observe_cpu("ARM7", arm7_port, None);
        let (after, after_identity) = trainer_runtime_win::read_dp_core_identity(
            process_id, &game_code, revision, arm9_port, arm7_port,
        )?;
        let epoch_stable = same_loaded_rom_session(&before, &after)
            && identity.pipe_and_process_bound
            && after_identity.pipe_and_process_bound
            && identity.both_gdb_ports_bound
            && after_identity.both_gdb_ports_bound;
        Ok::<_, String>(runtime_audit(identity, epoch_stable, arm9, arm7, snapshot))
    })
    .await
    .map_err(|e| format!("M10 independent dual CPU probe panicked: {e}"))?
}
#[cfg(not(windows))]
#[tauri::command]
pub async fn m10_dp_dual_cpu_overlay11_readonly(
    _process_id: u32,
    _game_code: String,
    _revision: u8,
    _arm9_port: u16,
    _arm7_port: u16,
) -> Result<RuntimeAudit, String> {
    Err("M10 two GDB-port probe only supported by Windows Tauri".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_unknown_or_unreviewed_dp_profiles() {
        assert!(checked_profile("APAE", 13).is_ok());
        assert!(checked_profile("ADAE", 13).is_ok());
        assert!(checked_profile("APAJ", 6).is_err());
        assert!(checked_profile("IPKJ", 0).is_err());
    }
}
