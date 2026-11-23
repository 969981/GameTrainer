//! M7: two independent DeSmuME GDB observations. Always READ ONLY.
//! The two local TCP endpoints are user-selected; there is no secure proof
//! that they are from one DeSmuME PID or share a simultaneous stop epoch.
//! Never promote this report to guest-write authorization.
use trainer_hgss::{
    m6::{inspect, GuestReader, OverlayReport},
    m7::{preflight, CpuStopObservation, M7Preflight},
    profile_for, HgssProfile,
};
use trainer_rsp::RspClient;

struct BoundedReader<'a>(&'a mut RspClient);
impl GuestReader for BoundedReader<'_> {
    fn read_guest(&mut self, guest: u32, bytes: usize) -> Result<Vec<u8>, String> {
        if bytes > 34 {
            return Err("M7 refused an unbounded guest read".into());
        }
        self.0.read(guest, bytes).map_err(|e| e.to_string())
    }
}

fn stop_observation(cpu: &str, port: u16) -> (CpuStopObservation, Option<RspClient>) {
    match RspClient::connect_local(port) {
        Ok(mut stub) if stub.is_paused() => {
            let pc_first = stub.pc();
            let pc_second = stub.pc();
            let (first, second, error) = match (pc_first, pc_second) {
                (Ok(a), Ok(b)) if a == b => (Some(a), Some(b), None),
                (Ok(a), Ok(b)) => (
                    Some(a),
                    Some(b),
                    Some("PC changed during stop probe".into()),
                ),
                (Err(e), _) | (_, Err(e)) => (None, None, Some(format!("GDB register read: {e}"))),
            };
            let result = CpuStopObservation {
                cpu: cpu.into(),
                port,
                rsp_stop_reply: true,
                pc_first: first,
                pc_second: second,
                error,
            };
            (result, Some(stub))
        }
        Ok(_) => (
            CpuStopObservation::unobserved(cpu, port, "GDB did not return a stopped CPU"),
            None,
        ),
        Err(e) => (
            CpuStopObservation::unobserved(cpu, port, &format!("GDB stop handshake failed: {e}")),
            None,
        ),
    }
}

fn read_overlay(
    stub: &mut RspClient,
    profile: &HgssProfile,
    observation: &CpuStopObservation,
) -> Option<OverlayReport> {
    if !observation.consistent() {
        return None;
    }
    let pc = observation.pc_second?;
    inspect(&mut BoundedReader(stub), profile, pc).ok()
}

#[tauri::command]
pub async fn m7_hgss_preflight(
    arm9_port: u16,
    arm7_port: u16,
    game_code: String,
    revision: u8,
) -> Result<M7Preflight, String> {
    if arm9_port < 1024 || arm7_port < 1024 || arm9_port == arm7_port {
        return Err("ARM9/ARM7 must use two distinct localhost GDB ports >= 1024".into());
    }
    let profile =
        profile_for(&game_code, revision).ok_or("ROM Code/Revision not in the HGSS manifest")?;
    if !profile.static_audit_1451 || profile.write_allowed {
        return Err("M7 requires one of the 12 audited original HGSS profiles".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        // Sequential stop observations are NOT a synchronized dual-CPU stop.
        let (arm9, mut arm9_stub) = stop_observation("ARM9", arm9_port);
        let observed_overlay = arm9_stub
            .as_mut()
            .and_then(|stub| read_overlay(stub, &profile, &arm9));
        let (arm7, _arm7_stub) = stop_observation("ARM7", arm7_port);
        let result = preflight(&game_code, revision, arm9, arm7, observed_overlay.as_ref());
        // Explicitly drop both sockets. No Guest writes, resume, interrupts or
        // process-level suspend operations have been issued.
        Ok::<_, String>(result)
    })
    .await
    .map_err(|e| format!("M7 dual GDB read-only probe crashed: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn static_m7_audit_excludes_us_legacy_and_korean() {
        assert!(profile_for("IPKJ", 0).unwrap().static_audit_1451);
        assert!(!profile_for("IPKE", 0).unwrap().static_audit_1451);
        assert!(profile_for("IPKK", 0).is_none());
    }
}
