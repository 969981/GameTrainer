//! M11: manually opted-in, source-allowlisted Diamond/Pearl *short opcode*
//! transaction primitives. NOT an atomic two-CPU/JIT/Overlay safe-point.
//!
//! The only executable patches represented here are a 16-bit player collision
//! return and the original APAE English battle capture comparison. No raw
//! addresses/bytes from UI. Cave, warp, battle trampolines and cleanup are
//! excluded from this live writer because the uploaded v0.8.14 evidence explicitly
//! lacks a safe Guest CPU and JIT/Overlay unload quiescence barrier.
//!
//! A real DeSmuME operation requires runtime OS-bound full ROM identity from
//! M10, manual ARM9/ARM7 stops, interpreter-only user attestation, an explicit
//! acknowledgement, and a durable before/after journal in Tauri.
use crate::{profile_for, GuestReader};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveFeature {
    WallCollision,
    CaptureEnglish,
}
impl LiveFeature {
    pub fn id(&self) -> &'static str {
        match self {
            Self::WallCollision => "wall_collision",
            Self::CaptureEnglish => "capture_english",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatchPlan {
    pub feature: LiveFeature,
    pub game_code: String,
    pub revision: u8,
    pub address: u32,
    pub original: Vec<u8>,
    pub replacement: Vec<u8>,
    pub expected_rom_sha256: String,
    pub source: String,
    pub risk_warning: String,
}
pub fn plan(code: &str, revision: u8, feature: LiveFeature) -> Result<PatchPlan, String> {
    let p = profile_for(code, revision).ok_or("M11 unknown ROM profile")?;
    // Only the two retail Pearl ROMs have complete original-image SHA256.
    let digest = p
        .rom_sha256
        .clone()
        .ok_or("M11 no complete retail ROM SHA256: ARM9/Overlay hashes are not a substitute")?;
    if !matches!(code, "APAE" | "APAI") || p.write_allowed || p.guest_exec_runtime_verified {
        return Err("M11 unsupported original ROM / suspicious profile flags".into());
    }
    let (address, original, replacement, source) = match feature {
        LiveFeature::WallCollision => (
            p.collision_addr,
            vec![0x20, 0x1C],
            vec![0x00, 0x20],
            "v0.8.14 src/main.lua T.WALL_COLLISION_HOOKS; 14-byte tail signature",
        ),
        LiveFeature::CaptureEnglish => {
            if code != "APAE" || revision != 13 {
                return Err("v0.8.14 EN16 capture source only allows English APAE".into());
            }
            (
                0x0223_C1F4,
                vec![0x01, 0x28],
                vec![0x80, 0x42],
                "v0.8.14 src/main.lua T.catchProfiles EN16 0x2801 -> 0x4280",
            )
        }
    };
    Ok(PatchPlan {
        feature,game_code:code.into(),revision,address,original,replacement,
        expected_rom_sha256:digest,
        source:source.into(),
        risk_warning:"EXPERIMENTAL: independently stopped ARM9/ARM7 and JIT-at-load=off do NOT prove atomic stop, current JIT/ICache or Overlay lifetime. Back up SAV/savestate. Never enable on another ROM."
            .into(),
    })
}
pub fn signature_preflight<R: GuestReader>(r: &mut R, p: &PatchPlan) -> Result<(), String> {
    if p.original.len() != 2 || p.replacement.len() != 2 || !p.address.is_multiple_of(2) {
        return Err("M11 patch must be exactly one aligned Thumb halfword".into());
    }
    if p.feature == LiveFeature::WallCollision {
        // Last seven 16-bit words in the ROM-verified PlayerAvatar collision
        // routine. Refuses a foreign hook or an overlay/mapping mismatch.
        let expected = [
            0x01, 0x28, 0x01, 0xD1, 0x01, 0x20, 0x04, 0x43, 0x20, 0x1C, 0xF8, 0xBD, 0x00, 0x00,
        ];
        let actual = r.read_guest(p.address - 8, expected.len())?;
        if actual != expected {
            return Err("M11 collision surrounding ARM9 Thumb original bytes mismatch".into());
        }
    }
    let observed = r.read_guest(p.address, p.original.len())?;
    if observed != p.original {
        return Err("M11 original opcode absent; existing foreign patch or changed Overlay".into());
    }
    Ok(())
}

pub trait GuardedBus: GuestReader {
    fn write_guest(&mut self, address: u32, bytes: &[u8]) -> Result<(), String>;
}
pub fn apply_guarded<B: GuardedBus>(bus: &mut B, p: &PatchPlan) -> Result<(), String> {
    signature_preflight(bus, p)?;
    bus.write_guest(p.address, &p.replacement)?;
    let got = bus.read_guest(p.address, p.replacement.len())?;
    if got != p.replacement {
        return Err("M11 write ACK/result uncertain: patched opcode not confirmed".into());
    }
    Ok(())
}
/// Only a record that owns the exact original/replacement tuple may restore.
/// A changed overlay or foreign patch is never overwritten.
pub fn restore_guarded<B: GuardedBus>(bus: &mut B, p: &PatchPlan) -> Result<(), String> {
    let got = bus.read_guest(p.address, p.replacement.len())?;
    if got != p.replacement {
        return Err("M11 current bytes no longer match owned patch; refuse blind restore".into());
    }
    bus.write_guest(p.address, &p.original)?;
    let readback = bus.read_guest(p.address, p.original.len())?;
    if readback != p.original {
        return Err("M11 original instruction restoration readback failed".into());
    }
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Prepared,
    Applied,
    Uncertain,
    Restored,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DurableRecord {
    pub owner_id: String,
    pub phase: Phase,
    pub process_id: u32,
    pub process_created_filetime: String,
    pub rom_epoch: String,
    pub arm9_port: u16,
    pub arm7_port: u16,
    pub plan: PatchPlan,
}
pub fn same_owner_session(
    a: &DurableRecord,
    pid: u32,
    created: &str,
    epoch: &str,
    sha256: &str,
    port9: u16,
    port7: u16,
) -> bool {
    a.process_id == pid
        && pid != 0
        && a.process_created_filetime == created
        && a.rom_epoch == epoch
        && a.arm9_port == port9
        && a.arm7_port == port7
        && a.plan.expected_rom_sha256.eq_ignore_ascii_case(sha256)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    struct Ram {
        bytes: HashMap<u32, u8>,
        failure: bool,
    }
    impl Ram {
        fn new(p: &PatchPlan) -> Self {
            let mut r = Self {
                bytes: HashMap::new(),
                failure: false,
            };
            if p.feature == LiveFeature::WallCollision {
                let known = [
                    0x01, 0x28, 0x01, 0xD1, 0x01, 0x20, 0x04, 0x43, 0x20, 0x1C, 0xF8, 0xBD, 0, 0,
                ];
                for (i, v) in known.iter().enumerate() {
                    r.bytes.insert(p.address - 8 + i as u32, *v);
                }
            } else {
                for (i, v) in p.original.iter().enumerate() {
                    r.bytes.insert(p.address + i as u32, *v);
                }
            }
            r
        }
    }
    impl GuestReader for Ram {
        fn read_guest(&mut self, addr: u32, len: usize) -> Result<Vec<u8>, String> {
            (0..len)
                .map(|i| {
                    self.bytes
                        .get(&(addr + i as u32))
                        .copied()
                        .ok_or("unmapped synthetic Guest byte".into())
                })
                .collect()
        }
    }
    impl GuardedBus for Ram {
        fn write_guest(&mut self, addr: u32, data: &[u8]) -> Result<(), String> {
            if self.failure {
                // Unknown state after a write acknowledgement timeout.
                self.bytes.insert(addr, data[0]);
                return Err("injected lost write ACK".into());
            }
            for (i, v) in data.iter().enumerate() {
                self.bytes.insert(addr + i as u32, *v);
            }
            Ok(())
        }
    }
    #[test]
    fn two_verified_pearl_short_patches_with_exact_opcode() {
        let en = plan("APAE", 13, LiveFeature::CaptureEnglish).unwrap();
        assert_eq!(en.address, 0x0223_C1F4);
        assert_eq!(en.original, [1, 0x28]);
        assert_eq!(en.replacement, [0x80, 0x42]);
        let it = plan("APAI", 5, LiveFeature::WallCollision).unwrap();
        assert_eq!(it.original, [0x20, 0x1C]);
        assert_eq!(it.replacement, [0, 0x20]);
        assert!(plan("APAI", 5, LiveFeature::CaptureEnglish).is_err());
        assert!(plan("ADAE", 13, LiveFeature::WallCollision).is_err());
        assert!(plan("APAF", 5, LiveFeature::WallCollision).is_err());
    }
    #[test]
    fn real_operation_model_reads_original_before_writing_and_restores_owned_bytes() {
        for (code, rev, kind) in [
            ("APAE", 13, LiveFeature::CaptureEnglish),
            ("APAE", 13, LiveFeature::WallCollision),
            ("APAI", 5, LiveFeature::WallCollision),
        ] {
            let p = plan(code, rev, kind).unwrap();
            let mut r = Ram::new(&p);
            apply_guarded(&mut r, &p).unwrap();
            assert_eq!(r.read_guest(p.address, 2).unwrap(), p.replacement);
            assert!(apply_guarded(&mut r, &p).is_err());
            restore_guarded(&mut r, &p).unwrap();
            assert_eq!(r.read_guest(p.address, 2).unwrap(), p.original);
        }
    }
    #[test]
    fn existing_third_party_patch_or_dirty_tail_is_never_overwritten() {
        let p = plan("APAI", 5, LiveFeature::WallCollision).unwrap();
        let mut r = Ram::new(&p);
        r.bytes.insert(p.address - 8, 0xFF);
        assert!(apply_guarded(&mut r, &p).is_err());
        r.bytes.insert(p.address - 8, 0x01);
        apply_guarded(&mut r, &p).unwrap();
        r.bytes.insert(p.address, 0xFF);
        assert!(restore_guarded(&mut r, &p).is_err());
        assert_eq!(r.bytes[&p.address], 0xFF);
    }
    #[test]
    fn write_ack_failure_preserves_uncertain_state_for_journal_recovery() {
        let p = plan("APAE", 13, LiveFeature::CaptureEnglish).unwrap();
        let mut r = Ram::new(&p);
        r.failure = true;
        assert!(apply_guarded(&mut r, &p).is_err());
        assert_ne!(r.read_guest(p.address, 2).unwrap(), p.original);
    }
    #[test]
    fn different_pid_created_time_epoch_and_full_sha_invalidates_owner() {
        let p = plan("APAE", 13, LiveFeature::CaptureEnglish).unwrap();
        let record = DurableRecord {
            owner_id: "fixture".into(),
            phase: Phase::Prepared,
            process_id: 101,
            process_created_filetime: "900".into(),
            rom_epoch: "3".into(),
            arm9_port: 3333,
            arm7_port: 3334,
            plan: p.clone(),
        };
        assert!(same_owner_session(
            &record,
            101,
            "900",
            "3",
            &p.expected_rom_sha256,
            3333,
            3334
        ));
        assert!(!same_owner_session(
            &record,
            102,
            "900",
            "3",
            &p.expected_rom_sha256,
            3333,
            3334
        ));
        assert!(!same_owner_session(
            &record,
            101,
            "901",
            "3",
            &p.expected_rom_sha256,
            3333,
            3334
        ));
        assert!(!same_owner_session(
            &record,
            101,
            "900",
            "4",
            &p.expected_rom_sha256,
            3333,
            3334
        ));
        assert!(!same_owner_session(
            &record, 101, "900", "3", "a", 3333, 3334
        ));
    }
}
