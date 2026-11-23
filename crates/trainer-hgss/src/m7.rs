//! M7 HGSS safety evidence, a deliberately non-authorizing preflight.
//! No production memory-writer exists in this crate. In particular, selected
//! ROM filenames, user-entered SHA1 and two independently stopped GDB stubs
//! cannot be promoted into a verified running-ROM/CPU/cache binding.
use serde::Serialize;

use crate::{
    m6::{OverlayReport, OverlayState},
    profile_for,
};

#[derive(Clone, Debug, Serialize)]
pub struct CpuStopObservation {
    pub cpu: String,
    pub port: u16,
    pub rsp_stop_reply: bool,
    pub pc_first: Option<u32>,
    pub pc_second: Option<u32>,
    pub error: Option<String>,
}

impl CpuStopObservation {
    pub fn consistent(&self) -> bool {
        self.rsp_stop_reply
            && self.pc_first.is_some()
            && self.pc_first == self.pc_second
            && self.error.is_none()
    }
    pub fn unobserved(cpu: &str, port: u16, reason: &str) -> Self {
        Self {
            cpu: cpu.to_owned(),
            port,
            rsp_stop_reply: false,
            pc_first: None,
            pc_second: None,
            error: Some(reason.to_owned()),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Blocker {
    UnsupportedOrUnauditedRom,
    MissingArm9Stop,
    MissingArm7Stop,
    DifferentOrUnknownEmulatorProcess,
    RunningRomDigestNotBound,
    CpuPairNotAtomicallyStopped,
    EmulatorJitAndIcacheNotVerified,
    OverlayEpochNotVerified,
    GuestRestoreFaultInjectionNotVerified,
    LiveWriteBackendIntentionallyAbsent,
}

/// A report is always diagnostic. It is deliberately impossible to request an
/// "override" with UI arguments or to unlock a Guest writer with this result.
#[derive(Clone, Debug, Serialize)]
pub struct M7Preflight {
    pub game_code: String,
    pub revision: u8,
    pub arm9: CpuStopObservation,
    pub arm7: CpuStopObservation,
    pub source_overlay12_original_observed: bool,
    pub running_rom_sha1_verified: bool,
    pub shared_emulator_instance_verified: bool,
    pub simultaneous_cpu_stop_verified: bool,
    pub jit_icache_coherence_verified: bool,
    pub live_restore_faults_verified: bool,
    pub write_authorized: bool,
    pub blockers: Vec<Blocker>,
    pub evidence_scope: String,
}

/// Even a matching offline ROM SHA1 is not evidence of the active DeSmuME ROM.
/// A GDB stop reply cannot certify that the two GDB sockets refer to the same
/// emulator, or that the other CPU / JIT code cache is safe for mutation.
/// This function has no input fields capable of bypassing those barriers.
pub fn preflight(
    game_code: &str,
    revision: u8,
    arm9: CpuStopObservation,
    arm7: CpuStopObservation,
    overlay: Option<&OverlayReport>,
) -> M7Preflight {
    let profile_ok =
        profile_for(game_code, revision).is_some_and(|p| p.static_audit_1451 && !p.write_allowed);
    let original_observed = overlay.is_some_and(|s| {
        s.game_code == game_code
            && s.revision == revision
            && s.gdb_stop_verified
            && s.neighbor_signature_valid
            && s.state == OverlayState::OriginalVerified
            && !s.write_authorized
    });
    let mut blockers = Vec::new();
    if !profile_ok {
        blockers.push(Blocker::UnsupportedOrUnauditedRom);
    }
    if !arm9.consistent() || arm9.cpu != "ARM9" {
        blockers.push(Blocker::MissingArm9Stop);
    }
    if !arm7.consistent() || arm7.cpu != "ARM7" {
        blockers.push(Blocker::MissingArm7Stop);
    }
    if !original_observed {
        blockers.push(Blocker::OverlayEpochNotVerified);
    }
    blockers.extend([
        Blocker::DifferentOrUnknownEmulatorProcess,
        Blocker::RunningRomDigestNotBound,
        Blocker::CpuPairNotAtomicallyStopped,
        Blocker::EmulatorJitAndIcacheNotVerified,
        Blocker::GuestRestoreFaultInjectionNotVerified,
        Blocker::LiveWriteBackendIntentionallyAbsent,
    ]);
    M7Preflight {
        game_code: game_code.into(),
        revision,
        arm9,
        arm7,
        source_overlay12_original_observed: original_observed,
        running_rom_sha1_verified: false,
        shared_emulator_instance_verified: false,
        simultaneous_cpu_stop_verified: false,
        jit_icache_coherence_verified: false,
        live_restore_faults_verified: false,
        write_authorized: false,
        blockers,
        evidence_scope:
            "m7_read_only_dual_gdb_independent_stop_evidence_not_safe_write_authorization".into(),
    }
}

/// M6 snapshots are not transferable to another process, restart, SaveState,
/// ROM revision, or new overlay generation. These are pure comparisons only.
pub fn same_overlay_observation(a: &OverlayReport, b: &OverlayReport) -> bool {
    a.game_code == b.game_code
        && a.revision == b.revision
        && a.overlay_child.is_some()
        && a.overlay_child == b.overlay_child
        && a.overlay_id == Some(12)
        && b.overlay_id == Some(12)
        && a.execution_state.is_some()
        && b.execution_state.is_some()
        && a.neighbor_signature_valid
        && b.neighbor_signature_valid
        && a.observed_halfword == b.observed_halfword
        && a.state == OverlayState::OriginalVerified
        && b.state == OverlayState::OriginalVerified
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::m6::overlay_spec;

    fn stop(cpu: &str, port: u16, pc: u32) -> CpuStopObservation {
        CpuStopObservation {
            cpu: cpu.into(),
            port,
            rsp_stop_reply: true,
            pc_first: Some(pc),
            pc_second: Some(pc),
            error: None,
        }
    }
    #[test]
    fn even_two_stopped_gdb_stubs_do_not_grant_writes() {
        let p = preflight(
            "IPKJ",
            0,
            stop("ARM9", 3333, 0x0200_1000),
            stop("ARM7", 3334, 0x0380_1000),
            None,
        );
        assert!(p.arm9.consistent() && p.arm7.consistent());
        assert!(!p.write_authorized);
        assert!(p.blockers.contains(&Blocker::RunningRomDigestNotBound));
        assert!(p.blockers.contains(&Blocker::CpuPairNotAtomicallyStopped));
        assert!(p
            .blockers
            .contains(&Blocker::EmulatorJitAndIcacheNotVerified));
    }
    #[test]
    fn swapped_cpu_labels_are_rejected() {
        let p = preflight(
            "IPKJ",
            0,
            stop("ARM7", 3333, 0x0200_1000),
            stop("ARM9", 3334, 0x0380_1000),
            None,
        );
        assert!(p.blockers.contains(&Blocker::MissingArm9Stop));
        assert!(p.blockers.contains(&Blocker::MissingArm7Stop));
    }
    #[test]
    fn rejects_legacy_us_unknown_korean_and_changed_revision() {
        for (code, rev) in [("IPKE", 0), ("IPKK", 0), ("IPKJ", 1)] {
            let p = preflight(
                code,
                rev,
                stop("ARM9", 3333, 0x02001000),
                stop("ARM7", 3334, 0x03801000),
                None,
            );
            assert!(p.blockers.contains(&Blocker::UnsupportedOrUnauditedRom));
            assert!(!p.write_authorized);
        }
    }
    #[test]
    fn stop_reply_without_registers_is_not_a_stop_proof() {
        let mut arm9 = stop("ARM9", 3333, 0x0200_1000);
        arm9.pc_second = Some(0x0200_1004);
        let p = preflight("IPKJ", 0, arm9, stop("ARM7", 3334, 0x0380_1000), None);
        assert!(p.blockers.contains(&Blocker::MissingArm9Stop));
    }
    #[test]
    fn missing_arm7_is_not_worked_around() {
        let p = preflight(
            "IPKJ",
            0,
            stop("ARM9", 3333, 0x0200_1000),
            CpuStopObservation::unobserved("ARM7", 3334, "stub absent"),
            None,
        );
        assert!(p.blockers.contains(&Blocker::MissingArm7Stop));
        assert!(!p.simultaneous_cpu_stop_verified);
    }
    #[test]
    fn expected_overlay_code_remains_non_authorizing_without_runtime_binding() {
        let spec = overlay_spec("IPKJ", 0).unwrap();
        let overlay = OverlayReport {
            game_code: "IPKJ".into(),
            revision: 0,
            arm9_pc: 0x0223_3000,
            gdb_stop_verified: true,
            field_system: Some(0x0200_4000),
            process_manager: Some(0x0200_5000),
            overlay_child: Some(0x0200_6000),
            overlay_id: Some(12),
            execution_state: Some(1),
            overlay_load_addr: spec.overlay_load_addr,
            capture_addr: spec.capture_addr,
            observed_halfword: Some(spec.original_halfword),
            neighbor_signature_valid: true,
            state: OverlayState::OriginalVerified,
            running_rom_sha1_verified: false,
            write_authorized: false,
            evidence_scope: "m6_stub".into(),
        };
        let p = preflight(
            "IPKJ",
            0,
            stop("ARM9", 3333, 0x02001000),
            stop("ARM7", 3334, 0x03801000),
            Some(&overlay),
        );
        assert!(p.source_overlay12_original_observed);
        assert!(!p.write_authorized);
        let mut replaced = overlay.clone();
        replaced.overlay_child = Some(0x0200_7000);
        assert!(!same_overlay_observation(&overlay, &replaced));
        replaced = overlay.clone();
        replaced.state = OverlayState::ForeignPatched;
        assert!(!same_overlay_observation(&overlay, &replaced));
        assert!(same_overlay_observation(&overlay, &overlay));
    }
}

#[cfg(test)]
mod restore_fault_injection {
    // Pure, synthetic halfword bus. No live emulator writer is linked here.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum State {
        New,
        Applied,
        Uncertain,
        Restored,
    }
    #[derive(Default)]
    struct Bus {
        cell: u16,
        fail_before_write: bool,
        fail_after_write: bool,
        corrupt_readback: bool,
    }
    impl Bus {
        fn read(&self) -> Result<u16, &'static str> {
            if self.corrupt_readback {
                Err("readback unavailable")
            } else {
                Ok(self.cell)
            }
        }
        fn write(&mut self, data: u16) -> Result<(), &'static str> {
            if self.fail_before_write {
                return Err("write rejected");
            }
            self.cell = data;
            if self.fail_after_write {
                return Err("ACK lost");
            }
            Ok(())
        }
    }
    struct Journal {
        session: u64,
        epoch: u64,
        state: State,
        original: u16,
        patched: u16,
    }
    impl Journal {
        fn new(session: u64, epoch: u64) -> Self {
            Self {
                session,
                epoch,
                state: State::New,
                original: 0x2801,
                patched: 0x4280,
            }
        }
        fn install(
            &mut self,
            bus: &mut Bus,
            session: u64,
            epoch: u64,
            stopped: bool,
        ) -> Result<(), &'static str> {
            if !stopped
                || session != self.session
                || epoch != self.epoch
                || self.state != State::New
            {
                return Err("precondition");
            }
            if bus.read()? != self.original {
                return Err("foreign code");
            }
            // Before attempting writes, real code would durably record intent.
            // Once a write is attempted, an error could mean it took effect.
            self.state = State::Uncertain;
            bus.write(self.patched)?;
            if bus.read()? != self.patched {
                return Err("patch verification failed");
            }
            self.state = State::Applied;
            Ok(())
        }
        fn restore(
            &mut self,
            bus: &mut Bus,
            session: u64,
            epoch: u64,
            stopped: bool,
        ) -> Result<(), &'static str> {
            if !stopped
                || session != self.session
                || epoch != self.epoch
                || !matches!(self.state, State::Applied | State::Uncertain)
            {
                return Err("owner/epoch/stop mismatch");
            }
            if bus.read()? != self.patched {
                return Err("foreign mutation or missing patch");
            }
            self.state = State::Uncertain;
            bus.write(self.original)?;
            if bus.read()? != self.original {
                return Err("restore readback missing");
            }
            self.state = State::Restored;
            Ok(())
        }
    }
    #[test]
    fn normal_patch_and_restore_fixture() {
        let mut b = Bus {
            cell: 0x2801,
            ..Default::default()
        };
        let mut j = Journal::new(10, 2);
        j.install(&mut b, 10, 2, true).unwrap();
        assert_eq!(b.cell, 0x4280);
        j.restore(&mut b, 10, 2, true).unwrap();
        assert_eq!(b.cell, 0x2801);
        assert!(j.restore(&mut b, 10, 2, true).is_err());
    }
    #[test]
    fn no_guest_stop_fails_before_writing() {
        let mut b = Bus {
            cell: 0x2801,
            ..Default::default()
        };
        let mut j = Journal::new(10, 2);
        assert!(j.install(&mut b, 10, 2, false).is_err());
        assert_eq!(b.cell, 0x2801);
        assert_eq!(j.state, State::New);
    }
    #[test]
    fn lost_write_ack_retains_uncertain_owner_and_recovers_only_if_exact() {
        let mut b = Bus {
            cell: 0x2801,
            fail_after_write: true,
            ..Default::default()
        };
        let mut j = Journal::new(10, 2);
        assert!(j.install(&mut b, 10, 2, true).is_err());
        assert_eq!(b.cell, 0x4280);
        assert_eq!(j.state, State::Uncertain);
        b.fail_after_write = false;
        j.restore(&mut b, 10, 2, true).unwrap();
        assert_eq!(j.state, State::Restored);
    }
    #[test]
    fn failed_write_and_late_overlay_epoch_do_not_blindly_restore() {
        let mut b = Bus {
            cell: 0x2801,
            fail_before_write: true,
            ..Default::default()
        };
        let mut j = Journal::new(10, 2);
        assert!(j.install(&mut b, 10, 2, true).is_err());
        assert_eq!(j.state, State::Uncertain);
        b.fail_before_write = false;
        assert!(j.restore(&mut b, 10, 3, true).is_err());
        assert!(j.restore(&mut b, 10, 2, true).is_err()); // still original
        assert_eq!(b.cell, 0x2801);
    }
    #[test]
    fn third_party_memory_change_is_never_overwritten() {
        let mut b = Bus {
            cell: 0x2801,
            ..Default::default()
        };
        let mut j = Journal::new(10, 2);
        j.install(&mut b, 10, 2, true).unwrap();
        b.cell = 0x9999;
        assert!(j.restore(&mut b, 10, 2, true).is_err());
        assert_eq!(b.cell, 0x9999);
    }
    #[test]
    fn session_replaced_or_rom_switched_cannot_inherit_owner() {
        let mut b = Bus {
            cell: 0x2801,
            ..Default::default()
        };
        let mut j = Journal::new(10, 2);
        j.install(&mut b, 10, 2, true).unwrap();
        assert!(j.restore(&mut b, 11, 2, true).is_err());
        assert!(j.restore(&mut b, 10, 3, true).is_err());
        assert_eq!(b.cell, 0x4280);
    }
    #[test]
    fn readback_failure_after_write_stays_uncertain() {
        let mut b = Bus {
            cell: 0x2801,
            ..Default::default()
        };
        let mut j = Journal::new(10, 2);
        j.install(&mut b, 10, 2, true).unwrap();
        b.corrupt_readback = true;
        assert!(j.restore(&mut b, 10, 2, true).is_err());
        assert_eq!(j.state, State::Applied);
        b.corrupt_readback = false;
        j.restore(&mut b, 10, 2, true).unwrap();
    }
    #[test]
    fn restore_lost_ack_reconciles_without_assuming_success() {
        let mut b = Bus {
            cell: 0x2801,
            ..Default::default()
        };
        let mut j = Journal::new(10, 2);
        j.install(&mut b, 10, 2, true).unwrap();
        b.fail_after_write = true;
        assert!(j.restore(&mut b, 10, 2, true).is_err());
        assert_eq!(b.cell, 0x2801);
        assert_eq!(j.state, State::Uncertain);
        // Never reattempt restore when target is no longer our patch.
        b.fail_after_write = false;
        assert!(j.restore(&mut b, 10, 2, true).is_err());
        assert_eq!(b.cell, 0x2801);
    }
    #[test]
    fn foreign_existing_patch_refuses_install_and_adoption() {
        let mut b = Bus {
            cell: 0x4280,
            ..Default::default()
        };
        let mut j = Journal::new(10, 2);
        assert!(j.install(&mut b, 10, 2, true).is_err());
        assert_eq!(j.state, State::New);
        assert_eq!(b.cell, 0x4280);
    }
}
