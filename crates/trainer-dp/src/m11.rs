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
    CaveStage,
}
impl LiveFeature {
    pub fn id(&self) -> &'static str {
        match self {
            Self::WallCollision => "wall_collision",
            Self::CaptureEnglish => "capture_english",
            Self::CaveStage => "cave_stage",
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
        LiveFeature::CaveStage => {
            // Only a stable, known-original, unreferenced 72B ARM9 Thumb scratch
            // region. Stage a deterministic RET-only Thumb test image but NEVER
            // install any caller/branch to it. This is not GuestExec 5A/A5.
            let mut demo = Vec::with_capacity(72);
            for _ in 0..35 {
                demo.extend_from_slice(&[0xC0, 0x46]);
            }
            demo.extend_from_slice(&[0x70, 0x47]); // bx lr
            (p.cave_addr,vec![0u8;72],demo,
             "v0.8.14 T.NATIVE_CAVES 72B all-zero region: laboratory-only code staging, NO executable ingress")
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
    let length = p.original.len();
    let valid_length = if p.feature == LiveFeature::CaveStage {
        length == 72 && p.replacement.len() == 72
    } else {
        length == 2 && p.replacement.len() == 2
    };
    if !valid_length || !p.address.is_multiple_of(2) {
        return Err("M11 source patch span/Thumb alignment is not allowlisted".into());
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
    if p.feature == LiveFeature::CaveStage {
        verify_no_executable_ingress(r, p)?;
    }
    let observed = r.read_guest(p.address, p.original.len())?;
    if observed != p.original {
        return Err("M11 original opcode absent; existing foreign patch or changed Overlay".into());
    }
    Ok(())
}

fn verify_no_executable_ingress<R: GuestReader>(r: &mut R, p: &PatchPlan) -> Result<(), String> {
    if p.feature != LiveFeature::CaveStage {
        return Ok(());
    }
    let profile = profile_for(&p.game_code, p.revision)
        .ok_or("missing original DP profile for Cave stage")?;
    // The v0.8.14 scratch is shared by native warp, battle crit/hit/OHKO,
    // and Guest Exec. Refuse whenever any known program entry is hooked.
    let (crit, orig_crit, hit, orig_hit, ohko, orig_ohko) = match p.game_code.as_str() {
        "APAE" => (
            0x0223_38E8,
            [0x18, 0xF0, 0x28, 0xFB],
            0x0224_084C,
            [0xAB, 0xF6, 0x18, 0xEA],
            0x0223_3110,
            [0x18, 0xF0, 0xFE, 0xFE],
        ),
        "APAI" => (
            0x0223_3A5C,
            [0x18, 0xF0, 0x46, 0xFB],
            0x0224_09CC,
            [0xAB, 0xF6, 0x76, 0xE9],
            0x0223_3270,
            [0x18, 0xF0, 0x26, 0xFF],
        ),
        _ => return Err("M11 no full-ROM Cave owner profile".into()),
    };
    let originals = [
        (profile.field_entry_addr + 4, [0x04, 0x1C, 0x21, 0x68]),
        (crit, orig_crit),
        (hit, orig_hit),
        (ohko, orig_ohko),
    ];
    for (addr, expected) in originals {
        let got = r.read_guest(addr, 4)?;
        if got != expected {
            return Err(format!("M11 cannot stage/clear 72B Cave: existing or unknown executable ingress @ 0x{addr:08X}"));
        }
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
    verify_no_executable_ingress(bus, p)?;
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
                if p.feature == LiveFeature::CaveStage {
                    let profile = profile_for(&p.game_code, p.revision).unwrap();
                    let sites = if p.game_code == "APAE" {
                        vec![
                            (0x0223_38E8, vec![0x18, 0xF0, 0x28, 0xFB]),
                            (0x0224_084C, vec![0xAB, 0xF6, 0x18, 0xEA]),
                            (0x0223_3110, vec![0x18, 0xF0, 0xFE, 0xFE]),
                        ]
                    } else {
                        vec![
                            (0x0223_3A5C, vec![0x18, 0xF0, 0x46, 0xFB]),
                            (0x0224_09CC, vec![0xAB, 0xF6, 0x76, 0xE9]),
                            (0x0223_3270, vec![0x18, 0xF0, 0x26, 0xFF]),
                        ]
                    };
                    for (i, b) in [0x04, 0x1C, 0x21, 0x68].iter().enumerate() {
                        r.bytes.insert(profile.field_entry_addr + 4 + i as u32, *b);
                    }
                    for (addr, bytes) in sites {
                        for (i, b) in bytes.iter().enumerate() {
                            r.bytes.insert(addr + i as u32, *b);
                        }
                    }
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
    fn stage_and_clear_only_known_original_72b_cave_without_any_guest_call() {
        for (code, rev) in [("APAE", 13), ("APAI", 5)] {
            let p = plan(code, rev, LiveFeature::CaveStage).unwrap();
            assert_eq!(p.original.len(), 72);
            assert_eq!(p.replacement.len(), 72);
            assert_eq!(&p.replacement[70..], &[0x70, 0x47]);
            let mut r = Ram::new(&p);
            apply_guarded(&mut r, &p).unwrap();
            assert!(apply_guarded(&mut r, &p).is_err());
            restore_guarded(&mut r, &p).unwrap();
            assert_eq!(r.read_guest(p.address, 72).unwrap(), vec![0u8; 72]);
            // Staging never patched any program entry.
            assert_eq!(
                r.read_guest(profile_for(code, rev).unwrap().field_entry_addr + 4, 4)
                    .unwrap(),
                vec![0x04, 0x1C, 0x21, 0x68]
            );
        }
    }
    #[test]
    fn cave_restore_refuses_any_foreign_crit_callsite() {
        let p = plan("APAE", 13, LiveFeature::CaveStage).unwrap();
        let mut r = Ram::new(&p);
        apply_guarded(&mut r, &p).unwrap();
        r.bytes.insert(0x0223_38E8, 0xFF);
        assert!(restore_guarded(&mut r, &p).is_err());
        assert_eq!(r.read_guest(p.address, 72).unwrap(), p.replacement);
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
