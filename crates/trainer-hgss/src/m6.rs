//! M6: safety evidence for NDS ARM9 / HGSS Overlay12.
//! All production-facing code below is READ ONLY. No Guest write operations
//! or disassembler-derived trampoline are exposed. This is not ROM identity proof.
use super::{profile_for, HgssProfile};
use serde::{Deserialize, Serialize};

const SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/hgss-m6-overlay12.json"
));

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OverlaySpec {
    pub game_code: String,
    pub revision: u8,
    pub field_system_ptr_addr: u32,
    pub overlay_load_addr: u32,
    pub capture_addr: u32,
    pub overlay_id: u32,
    pub original_halfword: u16,
    pub expected_patched_halfword: u16,
    pub prefix_hex: String,
    pub suffix_hex: String,
}

pub fn overlay_specs() -> Vec<OverlaySpec> {
    serde_json::from_str(SOURCE).expect("checked-in HGSS Overlay12 evidence is valid")
}
pub fn overlay_spec(code: &str, rev: u8) -> Option<OverlaySpec> {
    if !profile_for(code, rev)?.static_audit_1451 {
        return None;
    }
    overlay_specs()
        .into_iter()
        .find(|s| s.game_code == code && s.revision == rev)
}

pub trait GuestReader {
    fn read_guest(&mut self, address: u32, bytes: usize) -> Result<Vec<u8>, String>;
}
fn valid_main_pointer(p: u32) -> bool {
    (0x0200_0000..0x0240_0000).contains(&p) && p.is_multiple_of(4)
}
fn u32_guest<R: GuestReader>(r: &mut R, at: u32) -> Result<u32, String> {
    let b = r.read_guest(at, 4)?;
    if b.len() != 4 {
        return Err("short guest read".into());
    }
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}
fn decoded_hex(s: &str) -> Result<Vec<u8>, String> {
    if s.len() != 32 || !s.is_ascii() {
        return Err("signature must be exactly 16 bytes".into());
    }
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| {
            let pair = std::str::from_utf8(b).map_err(|_| "non-ascii signature")?;
            u8::from_str_radix(pair, 16).map_err(|_| "invalid signature hex".into())
        })
        .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverlayState {
    NotLoaded,
    PendingExecution,
    OriginalVerified,
    ForeignPatched,
    SignatureMismatch,
    UnexpectedInstruction,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverlayReport {
    pub game_code: String,
    pub revision: u8,
    pub arm9_pc: u32,
    pub gdb_stop_verified: bool,
    pub field_system: Option<u32>,
    pub process_manager: Option<u32>,
    pub overlay_child: Option<u32>,
    pub overlay_id: Option<u32>,
    pub execution_state: Option<u32>,
    pub overlay_load_addr: u32,
    pub capture_addr: u32,
    pub observed_halfword: Option<u16>,
    pub neighbor_signature_valid: bool,
    pub state: OverlayState,
    pub running_rom_sha1_verified: bool,
    pub write_authorized: bool,
    pub evidence_scope: String,
}
fn blank(spec: &OverlaySpec, pc: u32) -> OverlayReport {
    OverlayReport {
        game_code: spec.game_code.clone(),
        revision: spec.revision,
        arm9_pc: pc,
        gdb_stop_verified: true,
        field_system: None,
        process_manager: None,
        overlay_child: None,
        overlay_id: None,
        execution_state: None,
        overlay_load_addr: spec.overlay_load_addr,
        capture_addr: spec.capture_addr,
        observed_halfword: None,
        neighbor_signature_valid: false,
        state: OverlayState::NotLoaded,
        running_rom_sha1_verified: false,
        write_authorized: false,
        evidence_scope: "arm9_gdb_stopped_guest_read_only_not_full_rom_validation".into(),
    }
}

/// Requires an independently stopped ARM9 GDB session (the caller checks RSP
/// '?' and 'g'). Read-only inspection does not prove all emulator threads stop
/// or that the running NDS file is the same as an offline SHA1.
pub fn inspect<R: GuestReader>(
    reader: &mut R,
    profile: &HgssProfile,
    arm9_pc: u32,
) -> Result<OverlayReport, String> {
    if !profile.static_audit_1451 || profile.write_allowed {
        return Err("profile has no M6 v1.4.5.1 static audit".into());
    }
    if !(0x0200_0000..0x0300_0000).contains(&arm9_pc) {
        return Err("ARM9 GDB PC is outside expected game executable memory".into());
    }
    let spec = overlay_spec(&profile.game_code, profile.revision)
        .ok_or("no exact Code/Revision Overlay12 evidence")?;
    let mut result = blank(&spec, arm9_pc);

    let field_system = u32_guest(reader, spec.field_system_ptr_addr)?;
    if field_system == 0 {
        return Ok(result);
    }
    if !valid_main_pointer(field_system) {
        return Err("FieldSystem pointer invalid; refusing overlay inference".into());
    }
    result.field_system = Some(field_system);

    let manager = u32_guest(reader, field_system)?;
    if manager == 0 {
        return Ok(result);
    }
    if !valid_main_pointer(manager) {
        return Err("process manager pointer invalid".into());
    }
    result.process_manager = Some(manager);

    let child = u32_guest(reader, manager + 4)?;
    if child == 0 {
        return Ok(result);
    }
    if !valid_main_pointer(child) {
        return Err("overlay manager child pointer invalid".into());
    }
    result.overlay_child = Some(child);

    let id = u32_guest(reader, child + 0x0c)?;
    result.overlay_id = Some(id);
    if id != spec.overlay_id {
        return Ok(result);
    }
    let exec_state = u32_guest(reader, child + 0x10)?;
    result.execution_state = Some(exec_state);
    if exec_state == 0 {
        result.state = OverlayState::PendingExecution;
        return Ok(result);
    }

    // Full 16-byte context on each side plus the 2-byte instruction.
    let bytes = reader.read_guest(spec.capture_addr - 16, 34)?;
    if bytes.len() != 34 {
        return Err("incomplete Overlay12 instruction context".into());
    }
    let before = decoded_hex(&spec.prefix_hex)?;
    let after = decoded_hex(&spec.suffix_hex)?;
    result.observed_halfword = Some(u16::from_le_bytes([bytes[16], bytes[17]]));
    result.neighbor_signature_valid = bytes[..16] == before && bytes[18..] == after;
    result.state = if !result.neighbor_signature_valid {
        OverlayState::SignatureMismatch
    } else if result.observed_halfword == Some(spec.original_halfword) {
        OverlayState::OriginalVerified
    } else if result.observed_halfword == Some(spec.expected_patched_halfword) {
        // Do not adopt an identical existing patch: the owner is unknown.
        OverlayState::ForeignPatched
    } else {
        OverlayState::UnexpectedInstruction
    };
    Ok(result)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverlayTransition {
    NoChange,
    EnteredBattle,
    ExitedBattle,
    ReloadOrReplaced,
    CodeUnexpectedlyChanged,
}

fn loaded(s: &OverlayReport) -> bool {
    matches!(
        s.state,
        OverlayState::OriginalVerified
            | OverlayState::ForeignPatched
            | OverlayState::SignatureMismatch
            | OverlayState::UnexpectedInstruction
    )
}
pub fn compare_snapshots(a: &OverlayReport, b: &OverlayReport) -> OverlayTransition {
    if a.game_code != b.game_code || a.revision != b.revision {
        return OverlayTransition::ReloadOrReplaced;
    }
    match (loaded(a), loaded(b)) {
        (false, true) => OverlayTransition::EnteredBattle,
        (true, false) => OverlayTransition::ExitedBattle,
        (true, true) => {
            if a.overlay_child != b.overlay_child {
                OverlayTransition::ReloadOrReplaced
            } else if a.observed_halfword != b.observed_halfword
                || a.neighbor_signature_valid != b.neighbor_signature_valid
            {
                OverlayTransition::CodeUnexpectedlyChanged
            } else {
                OverlayTransition::NoChange
            }
        }
        _ => OverlayTransition::NoChange,
    }
}

/// Fixture ONLY: proving "original -> owned -> restored" under an explicit
/// simulated pause/epoch/byte ownership contract. Cannot access a live Guest.
#[cfg(test)]
mod fixture_restore {
    #[derive(Default)]
    pub struct MockPatch {
        active: bool,
        epoch: u64,
        original: u16,
        patched: u16,
    }
    impl MockPatch {
        pub fn install(
            &mut self,
            memory: &mut u16,
            expected: u16,
            replacement: u16,
            epoch: u64,
            paused: bool,
        ) -> Result<(), &'static str> {
            if !paused || self.active {
                return Err("not paused or already active");
            }
            if *memory != expected {
                return Err("not exact original; no adoption");
            }
            self.original = *memory;
            self.patched = replacement;
            self.epoch = epoch;
            *memory = replacement;
            if *memory != replacement {
                return Err("write readback failed");
            }
            self.active = true;
            Ok(())
        }
        pub fn restore(
            &mut self,
            memory: &mut u16,
            epoch: u64,
            paused: bool,
        ) -> Result<(), &'static str> {
            if !self.active || !paused || self.epoch != epoch {
                return Err("no owner, running CPU or overlay epoch changed");
            }
            if *memory != self.patched {
                return Err("third-party modification");
            }
            *memory = self.original;
            if *memory != self.original {
                return Err("restore readback failed");
            }
            self.active = false;
            Ok(())
        }
    }
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
        fn bytes(&mut self, a: u32, v: &[u8]) {
            for (i, b) in v.iter().enumerate() {
                self.0.insert(a + i as u32, *b);
            }
        }
        fn word(&mut self, a: u32, v: u32) {
            self.bytes(a, &v.to_le_bytes());
        }
    }
    impl GuestReader for Fake {
        fn read_guest(&mut self, a: u32, n: usize) -> Result<Vec<u8>, String> {
            (0..n)
                .map(|i| {
                    self.0
                        .get(&(a + i as u32))
                        .copied()
                        .ok_or_else(|| format!("unmapped {a:08x}+{i}"))
                })
                .collect()
        }
    }
    fn active_fixture(code: &str, rev: u8) -> (HgssProfile, OverlaySpec, Fake) {
        let p = profile_for(code, rev).unwrap();
        let spec = overlay_spec(code, rev).unwrap();
        let mut m = Fake::new();
        m.word(spec.field_system_ptr_addr, 0x0200_4000);
        m.word(0x0200_4000, 0x0200_5000);
        m.word(0x0200_5004, 0x0200_6000);
        m.word(0x0200_600c, 12);
        m.word(0x0200_6010, 1);
        let mut context = decoded_hex(&spec.prefix_hex).unwrap();
        context.extend_from_slice(&spec.original_halfword.to_le_bytes());
        context.extend_from_slice(&decoded_hex(&spec.suffix_hex).unwrap());
        m.bytes(spec.capture_addr - 16, &context);
        (p, spec, m)
    }
    #[test]
    fn source_catalog_has_exact_twelve_m6_audited_variants() {
        let s = overlay_specs();
        assert_eq!(s.len(), 12);
        assert_eq!(s.iter().filter(|s| s.overlay_id == 12).count(), 12);
        for v in s {
            assert!(
                profile_for(&v.game_code, v.revision)
                    .unwrap()
                    .static_audit_1451
            );
            assert_eq!(decoded_hex(&v.prefix_hex).unwrap().len(), 16);
            assert_eq!(decoded_hex(&v.suffix_hex).unwrap().len(), 16);
            assert!(v.capture_addr > v.overlay_load_addr);
            assert!(v.capture_addr < v.overlay_load_addr + 0x40000);
        }
        assert!(overlay_spec("IPKE", 0).is_none());
        assert!(overlay_spec("IPKK", 0).is_none());
    }
    #[test]
    fn overlay_loaded_and_original_signature_only_proves_read_evidence() {
        for (code, rev) in [("IPKJ", 0), ("IPGJ", 0), ("IPKE", 10), ("IPGS", 0)] {
            let (p, _, mut m) = active_fixture(code, rev);
            let r = inspect(&mut m, &p, 0x0223D3A6).unwrap();
            assert_eq!(r.state, OverlayState::OriginalVerified);
            assert!(r.gdb_stop_verified && r.neighbor_signature_valid);
            assert!(!r.running_rom_sha1_verified && !r.write_authorized);
        }
    }
    #[test]
    fn stopped_field_without_battle_is_not_loaded() {
        let (p, spec, mut m) = active_fixture("IPKJ", 0);
        m.word(spec.field_system_ptr_addr, 0);
        let r = inspect(&mut m, &p, 0x02001000).unwrap();
        assert_eq!(r.state, OverlayState::NotLoaded);
        assert!(r.observed_halfword.is_none());
    }
    #[test]
    fn pending_overlay_does_not_claim_loaded_code() {
        let (p, _, mut m) = active_fixture("IPKJ", 0);
        m.word(0x0200_6010, 0);
        assert_eq!(
            inspect(&mut m, &p, 0x02001000).unwrap().state,
            OverlayState::PendingExecution
        );
    }
    #[test]
    fn exact_foreign_patch_cannot_be_adopted_as_ours() {
        let (p, s, mut m) = active_fixture("IPKJ", 0);
        m.bytes(s.capture_addr, &s.expected_patched_halfword.to_le_bytes());
        let e = inspect(&mut m, &p, 0x02001000).unwrap();
        assert_eq!(e.state, OverlayState::ForeignPatched);
        assert!(!e.write_authorized);
    }
    #[test]
    fn mutated_surroundings_and_unknown_instruction_fail_closed() {
        let (p, s, mut m) = active_fixture("IPKJ", 0);
        m.bytes(s.capture_addr - 16, &[0xAA]);
        assert_eq!(
            inspect(&mut m, &p, 0x02001000).unwrap().state,
            OverlayState::SignatureMismatch
        );
        let (p, s, mut m) = active_fixture("IPKJ", 0);
        m.bytes(s.capture_addr, &0xFFFFu16.to_le_bytes());
        assert_eq!(
            inspect(&mut m, &p, 0x02001000).unwrap().state,
            OverlayState::UnexpectedInstruction
        );
    }
    #[test]
    fn invalid_pointer_and_unexpected_pc_are_rejected() {
        let (p, s, mut m) = active_fixture("IPKJ", 0);
        assert!(inspect(&mut m, &p, 0x0100_0000).is_err());
        m.word(s.field_system_ptr_addr, 0x7fff_ffff);
        assert!(inspect(&mut m, &p, 0x0200_1000).is_err());
    }
    #[test]
    fn detects_entry_exit_and_unexpected_code_changes() {
        let (p, s, mut m) = active_fixture("IPKJ", 0);
        let original = inspect(&mut m, &p, 0x0200_1000).unwrap();
        m.word(0x0200_600c, 5);
        let empty = inspect(&mut m, &p, 0x0200_1000).unwrap();
        assert_eq!(
            compare_snapshots(&empty, &original),
            OverlayTransition::EnteredBattle
        );
        assert_eq!(
            compare_snapshots(&original, &empty),
            OverlayTransition::ExitedBattle
        );
        m.word(0x0200_600c, 12);
        m.bytes(s.capture_addr, &s.expected_patched_halfword.to_le_bytes());
        let foreign = inspect(&mut m, &p, 0x0200_1000).unwrap();
        assert_eq!(
            compare_snapshots(&original, &foreign),
            OverlayTransition::CodeUnexpectedlyChanged
        );
    }
    #[test]
    fn fixture_restores_exact_owned_bytes_only_when_stopped_and_same_epoch() {
        use super::fixture_restore::MockPatch;
        let mut memory = 0x2801u16;
        let mut tx = MockPatch::default();
        assert!(tx.install(&mut memory, 0x2801, 0x4280, 9, false).is_err());
        tx.install(&mut memory, 0x2801, 0x4280, 9, true).unwrap();
        assert_eq!(memory, 0x4280);
        assert!(tx.restore(&mut memory, 10, true).is_err());
        assert!(tx.restore(&mut memory, 9, false).is_err());
        memory = 0x9999;
        assert!(tx.restore(&mut memory, 9, true).is_err());
        assert_eq!(memory, 0x9999);
        memory = 0x4280;
        tx.restore(&mut memory, 9, true).unwrap();
        assert_eq!(memory, 0x2801);
        assert!(tx.restore(&mut memory, 9, true).is_err());
    }
    #[test]
    fn fixture_rejects_third_party_exact_patch_instead_of_adopting() {
        use super::fixture_restore::MockPatch;
        let mut memory = 0x4280u16;
        let mut tx = MockPatch::default();
        assert!(tx.install(&mut memory, 0x2801, 0x4280, 1, true).is_err());
        assert_eq!(memory, 0x4280);
    }
}
