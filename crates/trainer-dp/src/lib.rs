//! Diamond/Pearl v0.8.14 GuestExecEvidence: source-backed READ-ONLY NDS profiles.
//! Never promote ARM9/Overlay06 component hashes to full ROM proof.
//! Overlay11 0x5A/0xA5 has static/offline evidence only; runtime not attested.
//! This crate has no Guest writes, Hook installer, code-cave cleanup, or Resume.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub mod m10;
pub mod m11;

pub const SOURCE_VERSION: &str = "0.8.14";
const PROFILES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/dp-v0814-profiles.json"
));
const BATTLE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/dp-v0814-overlay11-evidence.json"
));
#[derive(Clone, Debug, Deserialize)]
struct Manifest {
    source: String,
    source_main_lua_sha256: String,
    profiles: Vec<DpProfile>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DpProfile {
    pub game_code: String,
    pub revision: u8,
    pub header6c: u8,
    pub game_id: String,
    pub game: String,
    pub language: String,
    pub arm9_sha256: String,
    pub overlay06_sha256: String,
    pub arm9_size: u32,
    pub rom_sha256: Option<String>,
    pub current_seed_addr: u32,
    pub mt_seed_addr: u32,
    pub mt_index_addr: u32,
    pub delay_addr: u32,
    pub trainer_ptr_addr: u32,
    pub pid_ptr_addr: u32,
    pub field_global_addr: u32,
    pub field_entry_addr: u32,
    pub warp_addr: u32,
    pub collision_addr: u32,
    pub cave_addr: u32,
    pub battle_static_audit: bool,
    pub guest_exec_runtime_verified: bool,
    pub write_allowed: bool,
}
#[derive(Clone, Debug, Deserialize)]
struct BattleManifest {
    variants: Vec<BattleAudit>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BattleAudit {
    pub game_code: String,
    pub revision: u8,
    pub rom_sha256: String,
    pub overlay11_sha256: String,
    pub overlay11_base: u32,
    pub crit_callsite: u32,
    pub crit_target: u32,
    pub hit_callsite: u32,
    pub ohko_callsite: u32,
    pub guest_heap_alloc: u32,
    pub guest_heap_free: u32,
    pub dc_flush_all: u32,
    pub ic_invalidate_all: u32,
    pub thumb_cave_addr: u32,
    pub cave_bytes: u32,
}
pub fn profiles() -> Vec<DpProfile> {
    serde_json::from_str::<Manifest>(PROFILES)
        .expect("checked-in DP source manifest")
        .profiles
}
pub fn profile_for(code: &str, revision: u8) -> Option<DpProfile> {
    profiles()
        .into_iter()
        .find(|p| p.game_code == code && p.revision == revision)
}
pub fn battle_for(code: &str, revision: u8) -> Option<BattleAudit> {
    serde_json::from_str::<BattleManifest>(BATTLE)
        .expect("checked-in Overlay11 audit")
        .variants
        .into_iter()
        .find(|b| b.game_code == code && b.revision == revision)
}
pub fn valid_catalog() -> bool {
    let m: Manifest = serde_json::from_str(PROFILES).expect("valid source manifest");
    let mut seen = HashSet::new();
    m.profiles.len() == 10
        && m.source.contains("v0.8.14")
        && m.source_main_lua_sha256.len() == 64
        && m.profiles.iter().filter(|p| p.game == "Diamond").count() == 5
        && m.profiles.iter().filter(|p| p.game == "Pearl").count() == 5
        && m.profiles.iter().all(|p| {
            seen.insert((p.game_code.clone(), p.revision))
                && p.game_code.len() == 4
                && p.game_code.starts_with('A')
                && matches!(p.game_code.as_bytes()[1], b'D' | b'P')
                && p.game_code.as_bytes()[2] == b'A'
                && matches!(p.game_code.as_bytes()[3], b'E' | b'F' | b'D' | b'I' | b'S')
                && p.arm9_sha256.len() == 64
                && p.overlay06_sha256.len() == 64
                && p.arm9_sha256.bytes().all(|b| b.is_ascii_hexdigit())
                && p.overlay06_sha256.bytes().all(|b| b.is_ascii_hexdigit())
                && p.rom_sha256
                    .as_ref()
                    .is_none_or(|x| x.len() == 64 && x.bytes().all(|b| b.is_ascii_hexdigit()))
                && p.mt_seed_addr == p.current_seed_addr + 4
                && p.arm9_size >= 1_000_000
                && !p.write_allowed
                && !p.guest_exec_runtime_verified
        })
        && profiles().iter().filter(|p| p.battle_static_audit).count() == 2
        && profiles().iter().filter(|p| p.rom_sha256.is_some()).count() == 2
}
pub trait GuestReader {
    fn read_guest(&mut self, address: u32, len: usize) -> Result<Vec<u8>, String>;
}
fn read<R: GuestReader>(r: &mut R, address: u32, len: usize) -> Result<Vec<u8>, String> {
    if len == 0 || len > 72 {
        return Err("DP read request outside fixed size cap".into());
    }
    let b = r.read_guest(address, len)?;
    if b.len() != len {
        return Err(format!("DP short Guest memory read @ {address:08X}"));
    }
    Ok(b)
}
fn word<R: GuestReader>(r: &mut R, at: u32) -> Result<u32, String> {
    let b = read(r, at, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn half<R: GuestReader>(r: &mut R, at: u32) -> Result<u16, String> {
    let b = read(r, at, 2)?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}
fn valid_nds_ptr(p: u32) -> bool {
    (0x0200_0000..0x0300_0000).contains(&p) && p.is_multiple_of(4)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BattleObservation {
    pub version_static_evidence: bool,
    pub overlay11_callsite: u32,
    pub observed_thumb_call_bytes: String,
    pub thumb_bl_candidate: bool,
    pub source_cave_addr: u32,
    pub cave_all_zero: bool,
    pub guest_execution_confirmed: bool,
    pub code_cave_restore_safe: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DpReadOnlySnapshot {
    pub source_version: String,
    pub game_id: String,
    pub game_code: String,
    pub revision: u8,
    pub language: String,
    pub arm9_pc: u32,
    pub gdb_stop_observed: bool,
    pub header6c: u8,
    pub arm9_anchors_match: bool,
    pub current_seed: u32,
    pub mt_state0: u32,
    pub mt_index: u32,
    pub delay: u32,
    pub trainer_ptr: u32,
    pub pid_ptr: u32,
    pub battle: Option<BattleObservation>,
    pub complete_running_rom_sha256_verified: bool,
    pub write_authorized: bool,
    pub evidence_scope: String,
}
fn hex(bytes: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(H[(b >> 4) as usize] as char);
        s.push(H[(b & 0xF) as usize] as char);
    }
    s
}
/// Requires independent ARM9 GDB stop confirmation and stable PC from caller.
/// NDS main RAM header is mirrored every 4 MiB; 0x023FFE0C is the canonical
/// 4 MiB window alias of original CE source 0x02FFFE0C. No writes or resume.
/// Code/Rev/header6C and ARM9 opcodes do NOT prove a full loaded ROM digest.
pub fn inspect<R: GuestReader>(
    r: &mut R,
    profile: &DpProfile,
    arm9_pc: u32,
) -> Result<DpReadOnlySnapshot, String> {
    if profile.write_allowed
        || profile.guest_exec_runtime_verified
        || !(0x0200_0000..0x0300_0000).contains(&arm9_pc)
    {
        return Err("DP profile or CPU PC not eligible for read-only audit".into());
    }
    let code_bytes = read(r, 0x023F_FE0C, 4)?;
    if code_bytes != profile.game_code.as_bytes() {
        return Err("running NDS header Code differs from requested DP version".into());
    }
    let revision = read(r, 0x023F_FE1E, 1)?[0];
    let header6c = read(r, 0x023F_FE6C, 1)?[0];
    if revision != profile.revision || header6c != profile.header6c {
        return Err("DP NDS Header Revision/0x6C fingerprint mismatch".into());
    }
    let anchors = [
        half(r, profile.field_entry_addr)?,
        half(r, profile.field_entry_addr + 2)?,
        half(r, profile.warp_addr)?,
        half(r, profile.warp_addr + 2)?,
        half(r, profile.collision_addr - 8)?,
        half(r, profile.collision_addr - 6)?,
    ];
    let expected = [0xB538, 0xB084, 0xB530, 0xB085, 0x2801, 0xD101];
    if anchors != expected {
        return Err(
            "DP ARM9 source code anchor mismatch; stop stale/wrong/modified session".into(),
        );
    }
    let seed = word(r, profile.current_seed_addr)?;
    let mt = word(r, profile.mt_seed_addr)?;
    let index = word(r, profile.mt_index_addr)?;
    let delay = word(r, profile.delay_addr)?;
    let trainer_ptr = word(r, profile.trainer_ptr_addr)?;
    let pid_ptr = word(r, profile.pid_ptr_addr)?;
    if index > 624 || !valid_nds_ptr(trainer_ptr) || !valid_nds_ptr(pid_ptr) {
        return Err("DP RNG MT index or pointers outside source-backed bounds".into());
    }
    let battle = match battle_for(&profile.game_code, profile.revision) {
        Some(source) => {
            let call = read(r, source.crit_callsite, 4)?;
            let first = u16::from_le_bytes([call[0], call[1]]);
            let second = u16::from_le_bytes([call[2], call[3]]);
            let bl = first & 0xF800 == 0xF000 && second & 0xF800 == 0xF800;
            let cave = read(r, source.thumb_cave_addr, 72)?;
            Some(BattleObservation {
                version_static_evidence: true,
                overlay11_callsite: source.crit_callsite,
                observed_thumb_call_bytes: hex(&call),
                thumb_bl_candidate: bl,
                source_cave_addr: source.thumb_cave_addr,
                cave_all_zero: cave.iter().all(|x| *x == 0),
                guest_execution_confirmed: false,
                code_cave_restore_safe: false,
            })
        }
        None => None,
    };
    Ok(DpReadOnlySnapshot {
        source_version: SOURCE_VERSION.into(),
        game_id: profile.game_id.clone(),
        game_code: profile.game_code.clone(),
        revision,
        language: profile.language.clone(),
        arm9_pc,
        gdb_stop_observed: true,
        header6c,
        arm9_anchors_match: true,
        current_seed: seed,
        mt_state0: mt,
        mt_index: index,
        delay,
        trainer_ptr,
        pid_ptr,
        battle,
        complete_running_rom_sha256_verified: false,
        write_authorized: false,
        evidence_scope:
            "dp_arm9_gdb_guest_code_rev_6c_opcode_rng_read_only_not_full_rom_or_safe_patch".into(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    struct Fake(HashMap<u32, u8>);
    impl Fake {
        fn new() -> Self {
            Self(HashMap::new())
        }
        fn bytes(&mut self, a: u32, b: &[u8]) {
            for (i, c) in b.iter().enumerate() {
                self.0.insert(a + i as u32, *c);
            }
        }
        fn u16(&mut self, a: u32, v: u16) {
            self.bytes(a, &v.to_le_bytes());
        }
        fn u32(&mut self, a: u32, v: u32) {
            self.bytes(a, &v.to_le_bytes());
        }
    }
    impl GuestReader for Fake {
        fn read_guest(&mut self, address: u32, len: usize) -> Result<Vec<u8>, String> {
            (0..len)
                .map(|i| {
                    self.0
                        .get(&(address + i as u32))
                        .copied()
                        .ok_or_else(|| format!("unmapped {address:08X}+{i}"))
                })
                .collect()
        }
    }
    fn fixture(code: &str, revision: u8) -> (DpProfile, Fake) {
        let p = profile_for(code, revision).unwrap();
        let mut f = Fake::new();
        f.bytes(0x023F_FE0C, p.game_code.as_bytes());
        f.bytes(0x023F_FE1E, &[p.revision]);
        f.bytes(0x023F_FE6C, &[p.header6c]);
        for (a, v) in [
            (p.field_entry_addr, 0xB538),
            (p.field_entry_addr + 2, 0xB084),
            (p.warp_addr, 0xB530),
            (p.warp_addr + 2, 0xB085),
            (p.collision_addr - 8, 0x2801),
            (p.collision_addr - 6, 0xD101),
        ] {
            f.u16(a, v);
        }
        for (a, v) in [
            (p.current_seed_addr, 0xDEAD_BEEF),
            (p.mt_seed_addr, 0x1234_5678),
            (p.mt_index_addr, 624),
            (p.delay_addr, 200),
            (p.trainer_ptr_addr, 0x0212_3000),
            (p.pid_ptr_addr, 0x0212_4000),
        ] {
            f.u32(a, v);
        }
        if let Some(b) = battle_for(code, revision) {
            f.bytes(b.crit_callsite, &[0x00, 0xF0, 0x00, 0xF8]);
            f.bytes(b.thumb_cave_addr, &[0u8; 72]);
        }
        (p, f)
    }
    #[test]
    fn ten_source_profiles_are_distinct_and_never_writable() {
        assert!(valid_catalog());
        assert!(profile_for("ADAE", 13).is_some());
        assert!(profile_for("APAE", 13).is_some());
        assert!(profile_for("ADAE", 0).is_none());
        assert!(profile_for("APAJ", 6).is_none());
        assert!(profile_for("APAK", 0).is_none());
    }
    #[test]
    fn two_pearl_variants_only_have_full_rom_and_overlay11_static_audits() {
        let pp = profiles();
        assert_eq!(pp.iter().filter(|p| p.battle_static_audit).count(), 2);
        for code in ["APAE", "APAI"] {
            let p = pp.iter().find(|p| p.game_code == code).unwrap();
            let b = battle_for(code, p.revision).unwrap();
            assert_eq!(p.rom_sha256.as_ref().unwrap(), &b.rom_sha256);
            assert_eq!(b.cave_bytes, 72);
        }
        assert!(battle_for("ADAE", 13).is_none());
    }
    #[test]
    fn each_of_ten_profiles_yields_read_only_stopped_snapshot() {
        for p in profiles() {
            let (p, mut f) = fixture(&p.game_code, p.revision);
            let s = inspect(&mut f, &p, 0x0200_1040).unwrap();
            assert_eq!(s.current_seed, 0xDEAD_BEEF);
            assert_eq!(s.mt_index, 624);
            assert!(s.arm9_anchors_match && s.gdb_stop_observed);
            assert!(!s.complete_running_rom_sha256_verified && !s.write_authorized);
            if p.battle_static_audit {
                assert!(s.battle.unwrap().thumb_bl_candidate);
            } else {
                assert!(s.battle.is_none());
            }
        }
    }
    #[test]
    fn corrupted_revision_and_anchors_fail_closed() {
        let (p, mut f) = fixture("ADAE", 13);
        f.bytes(0x023F_FE1E, &[5]);
        assert!(inspect(&mut f, &p, 0x0200_1040).is_err());
        let (p, mut f) = fixture("APAI", 5);
        f.u16(p.collision_addr - 8, 0xFFFF);
        assert!(inspect(&mut f, &p, 0x0200_1040).is_err());
    }
    #[test]
    fn incorrect_gdb_pc_mt_index_and_pointers_rejected() {
        let (p, mut f) = fixture("ADAD", 5);
        assert!(inspect(&mut f, &p, 0x0100_0000).is_err());
        f.u32(p.mt_index_addr, 9999);
        assert!(inspect(&mut f, &p, 0x0200_1040).is_err());
        f.u32(p.mt_index_addr, 2);
        f.u32(p.trainer_ptr_addr, 0);
        assert!(inspect(&mut f, &p, 0x0200_1040).is_err());
    }
    #[test]
    fn source_guest_thumb_5a_a5_is_not_execution_proof() {
        for code in ["APAE", "APAI"] {
            let p = profiles()
                .into_iter()
                .find(|p| p.game_code == code)
                .unwrap();
            let (p, mut f) = fixture(code, p.revision);
            let b = battle_for(code, p.revision).unwrap();
            f.bytes(b.thumb_cave_addr, &[0x5A, 0x20, 0x70, 0x47]);
            let s = inspect(&mut f, &p, 0x0200_1040).unwrap();
            let obs = s.battle.unwrap();
            assert!(!obs.cave_all_zero);
            assert!(!obs.guest_execution_confirmed && !obs.code_cave_restore_safe);
        }
    }
}
