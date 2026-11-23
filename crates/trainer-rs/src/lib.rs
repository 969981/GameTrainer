//! Ruby/Sapphire v4.6.0 Gen3 ROM identity and RNG snapshot (read-only).
//! Facts (GameCode/Rev/anchor/Seed/Timer) were checked against the user-provided
//! GPL-3.0 RubySapphire CE v4.6.0 src/main.lua. This is an independently authored
//! adapter using factual constants, not a port of CE runtime code or Thumb payloads.
//!
//! Header+16-byte boot-table anchor are PARTIAL identity only: they never authorize
//! PatchManager writes, ROM code cave hooks, or save edits.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;
use trainer_mgba::{GuestMemoryReader, RuntimeFingerprint};

const ROM_BASE: u32 = 0x0800_0000;
const HEADER_LEN: usize = 0xC0;
pub const ROM_ANCHOR_ADDRESS: u32 = 0x0800_0128;
static ID_COUNTER: AtomicU64 = AtomicU64::new(1);
const BOOT_ANCHOR_COMMON: [u8; 16] = [
    0xB8, 0x00, 0xC3, 0xE1, 0x22, 0x18, 0x02, 0xE0, 0x00, 0xC0, 0xA0, 0xE3, 0x80, 0x00, 0x11, 0xE2,
];
const ANCHOR_RUBY_DE: [u8; 16] = [
    0xD0, 0x52, 0x1F, 0x08, 0x70, 0x67, 0x1F, 0x08, 0x30, 0x75, 0x1F, 0x08, 0xF0, 0x82, 0x1F, 0x08,
];
const ANCHOR_SAPPHIRE_DE: [u8; 16] = [
    0x64, 0x52, 0x1F, 0x08, 0x04, 0x67, 0x1F, 0x08, 0xC4, 0x74, 0x1F, 0x08, 0x84, 0x82, 0x1F, 0x08,
];
const ANCHOR_RUBY_FR: [u8; 16] = [
    0x5C, 0x07, 0x1F, 0x08, 0xFC, 0x1B, 0x1F, 0x08, 0xBC, 0x29, 0x1F, 0x08, 0x7C, 0x37, 0x1F, 0x08,
];
const ANCHOR_SAPPHIRE_FR: [u8; 16] = [
    0xEC, 0x06, 0x1F, 0x08, 0x8C, 0x1B, 0x1F, 0x08, 0x4C, 0x29, 0x1F, 0x08, 0x0C, 0x37, 0x1F, 0x08,
];
const ANCHOR_RUBY_IT: [u8; 16] = [
    0xF0, 0x9F, 0x1E, 0x08, 0x90, 0xB4, 0x1E, 0x08, 0x50, 0xC2, 0x1E, 0x08, 0x10, 0xD0, 0x1E, 0x08,
];
const ANCHOR_SAPPHIRE_IT: [u8; 16] = [
    0x80, 0x9F, 0x1E, 0x08, 0x20, 0xB4, 0x1E, 0x08, 0xE0, 0xC1, 0x1E, 0x08, 0xA0, 0xCF, 0x1E, 0x08,
];
const ANCHOR_RUBY_ES: [u8; 16] = [
    0x74, 0xD0, 0x1E, 0x08, 0x14, 0xE5, 0x1E, 0x08, 0xD4, 0xF2, 0x1E, 0x08, 0x94, 0x00, 0x1F, 0x08,
];
const ANCHOR_SAPPHIRE_ES: [u8; 16] = [
    0x04, 0xD0, 0x1E, 0x08, 0xA4, 0xE4, 0x1E, 0x08, 0x64, 0xF2, 0x1E, 0x08, 0x24, 0x00, 0x1F, 0x08,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RsGame {
    Ruby,
    Sapphire,
}
impl RsGame {
    pub fn id(self) -> &'static str {
        match self {
            Self::Ruby => "games.pokemon.ruby",
            Self::Sapphire => "games.pokemon.sapphire",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Ruby => "Pokémon Ruby",
            Self::Sapphire => "Pokémon Sapphire",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RsLayout {
    Usa,
    Japanese,
    Europe,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RsProfile {
    pub game_code: String,
    pub revision: u8,
    pub game: RsGame,
    pub language: String,
    pub build_id: String,
    pub layout: RsLayout,
    pub seed_address: u32,
    pub timer_address: u32,
    pub profile_strength: String,
    pub write_allowed: bool,
}
fn anchor_for(code: &str) -> Option<&'static [u8; 16]> {
    Some(match code {
        "AXVE" | "AXPE" | "AXVJ" | "AXPJ" => &BOOT_ANCHOR_COMMON,
        "AXVD" => &ANCHOR_RUBY_DE,
        "AXPD" => &ANCHOR_SAPPHIRE_DE,
        "AXVF" => &ANCHOR_RUBY_FR,
        "AXPF" => &ANCHOR_SAPPHIRE_FR,
        "AXVI" => &ANCHOR_RUBY_IT,
        "AXPI" => &ANCHOR_SAPPHIRE_IT,
        "AXVS" => &ANCHOR_RUBY_ES,
        "AXPS" => &ANCHOR_SAPPHIRE_ES,
        _ => return None,
    })
}

#[derive(Debug, Error)]
pub enum RsError {
    #[error("invalid Ruby/Sapphire header: {0}")]
    Header(&'static str),
    #[error("unsupported Ruby/Sapphire version: {0}")]
    Unsupported(String),
    #[error("retail ROM boot-table anchor does not match; debug/hack/unknown variant rejected")]
    AnchorMismatch,
    #[error("guest memory failed at {0:#010x}: {1}")]
    Read(u32, String),
    #[error("runtime identity changed or process memory mapping is stale")]
    StaleSession,
    #[error("session permanently invalidated")]
    Invalidated,
}

/// True only for source-listed 16-byte retail boot-table anchors.
/// Identifies a candidate, never authorizes any mutation.
pub fn matches_anchor(code: &str, actual: &[u8; 16]) -> bool {
    anchor_for(code).is_some_and(|expected| actual == expected)
}
pub fn profile_for(code: &str, revision: u8) -> Result<RsProfile, RsError> {
    let c = code.as_bytes();
    if c.len() != 4 || &c[..2] != b"AX" || !matches!(c[2], b'V' | b'P') {
        return Err(RsError::Unsupported(code.to_string()));
    }
    let (language, layout, seed, timer, max_rev) = match c[3] {
        b'E' => ("ENG", RsLayout::Usa, 0x03004818, 0x03001790, 2u8),
        b'J' => ("JPN", RsLayout::Japanese, 0x03004748, 0x03001700, 1u8),
        b'D' => ("DEU", RsLayout::Europe, 0x03004828, 0x03001790, 1u8),
        b'F' => ("FRA", RsLayout::Europe, 0x03004828, 0x03001790, 1u8),
        b'I' => ("ITA", RsLayout::Europe, 0x03004828, 0x03001790, 1u8),
        b'S' => ("ESP", RsLayout::Europe, 0x03004828, 0x03001790, 1u8),
        _ => return Err(RsError::Unsupported(code.to_string())),
    };
    if anchor_for(code).is_none() || revision > max_rev {
        return Err(RsError::Unsupported(format!("{code} Rev{revision}")));
    }
    let game = if c[2] == b'V' {
        RsGame::Ruby
    } else {
        RsGame::Sapphire
    };
    Ok(RsProfile {
        game_code: code.to_string(),
        revision,
        game,
        language: language.into(),
        build_id: format!("{code}_R{revision}"),
        layout,
        seed_address: seed,
        timer_address: timer,
        profile_strength: "header_revision_boot_anchor_only".into(),
        write_allowed: false,
    })
}
/// All 26 official retail Header Profiles; Box JP Rev0 variants are aliases,
/// not additional version/revision rows.
pub fn retail_profiles() -> Vec<RsProfile> {
    let mut found = Vec::with_capacity(26);
    for code in [
        "AXVE", "AXPE", "AXVJ", "AXPJ", "AXVD", "AXPD", "AXVF", "AXPF", "AXVI", "AXPI", "AXVS",
        "AXPS",
    ] {
        let max = if code.ends_with('E') { 2 } else { 1 };
        for revision in 0..=max {
            found.push(profile_for(code, revision).expect("registered retail profile"));
        }
    }
    found
}
pub fn parse_header(header: &[u8]) -> Result<RsProfile, RsError> {
    if header.len() < HEADER_LEN {
        return Err(RsError::Header("header too short"));
    }
    if header[0xB2] != 0x96 {
        return Err(RsError::Header("GBA fixed value"));
    }
    let checksum = header[0xA0..=0xBC]
        .iter()
        .fold(0x19u8, |sum, b| sum.wrapping_add(*b))
        .wrapping_neg();
    if header[0xBD] != checksum {
        return Err(RsError::Header("header checksum"));
    }
    let code = std::str::from_utf8(&header[0xAC..0xB0])
        .map_err(|_| RsError::Header("game code is not ASCII"))?;
    let profile = profile_for(code, header[0xBC])?;
    let expected = match profile.game {
        RsGame::Ruby => b"POKEMON RUBY".as_slice(),
        RsGame::Sapphire => b"POKEMON SAPP".as_slice(),
    };
    if !header[0xA0..0xAC].starts_with(expected) {
        return Err(RsError::Header("title/game code mismatch"));
    }
    Ok(profile)
}
pub fn verify_boot_anchor<R: GuestMemoryReader>(
    reader: &R,
    profile: &RsProfile,
) -> Result<(), RsError> {
    let mut data = [0u8; 16];
    reader
        .read_guest(ROM_ANCHOR_ADDRESS, &mut data)
        .map_err(|e| RsError::Read(ROM_ANCHOR_ADDRESS, e.to_string()))?;
    if data
        != *anchor_for(&profile.game_code)
            .ok_or_else(|| RsError::Unsupported(profile.game_code.clone()))?
    {
        return Err(RsError::AnchorMismatch);
    }
    Ok(())
}
fn read_u32<R: GuestMemoryReader>(reader: &R, address: u32) -> Result<u32, RsError> {
    let mut value = [0; 4];
    reader
        .read_guest(address, &mut value)
        .map_err(|e| RsError::Read(address, e.to_string()))?;
    Ok(u32::from_le_bytes(value))
}
fn read_u16<R: GuestMemoryReader>(reader: &R, address: u32) -> Result<u16, RsError> {
    let mut value = [0; 2];
    reader
        .read_guest(address, &mut value)
        .map_err(|e| RsError::Read(address, e.to_string()))?;
    Ok(u16::from_le_bytes(value))
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RsSeedSnapshot {
    pub session_id: String,
    pub epoch: u64,
    pub game_id: String,
    pub game_code: String,
    pub revision: u8,
    pub language: String,
    pub current_seed: u32,
    pub timer: u16,
    pub verification: String,
    pub read_only: bool,
}
pub struct RsReadOnlySession {
    session_id: String,
    epoch: u64,
    fingerprint: RuntimeFingerprint,
    profile: RsProfile,
    invalidated: bool,
}
impl RsReadOnlySession {
    pub fn attach<R: GuestMemoryReader>(reader: &R) -> Result<Self, RsError> {
        let fingerprint = reader.fingerprint().map_err(|_| RsError::StaleSession)?;
        let mut header = [0; HEADER_LEN];
        reader
            .read_guest(ROM_BASE, &mut header)
            .map_err(|e| RsError::Read(ROM_BASE, e.to_string()))?;
        let profile = parse_header(&header)?;
        if fingerprint.game_code != profile.game_code || fingerprint.revision != profile.revision {
            return Err(RsError::StaleSession);
        }
        verify_boot_anchor(reader, &profile)?;
        let _ = read_u32(reader, profile.seed_address)?;
        let _ = read_u16(reader, profile.timer_address)?;
        if reader.fingerprint().map_err(|_| RsError::StaleSession)? != fingerprint {
            return Err(RsError::StaleSession);
        }
        let epoch = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let session_id = format!(
            "mgba-rs-{}-{}-{}",
            fingerprint.process_id, fingerprint.process_created_at, epoch
        );
        Ok(Self {
            session_id,
            epoch,
            fingerprint,
            profile,
            invalidated: false,
        })
    }
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    pub fn profile(&self) -> &RsProfile {
        &self.profile
    }
    pub fn snapshot<R: GuestMemoryReader>(
        &mut self,
        reader: &R,
    ) -> Result<RsSeedSnapshot, RsError> {
        if self.invalidated {
            return Err(RsError::Invalidated);
        }
        if !matches!(reader.fingerprint(), Ok(fingerprint) if fingerprint == self.fingerprint) {
            self.invalidated = true;
            return Err(RsError::StaleSession);
        }
        // Any short/failed guest read also invalidates this target session.
        let current_seed = match read_u32(reader, self.profile.seed_address) {
            Ok(seed) => seed,
            Err(error) => {
                self.invalidated = true;
                return Err(error);
            }
        };
        let timer = match read_u16(reader, self.profile.timer_address) {
            Ok(timer) => timer,
            Err(error) => {
                self.invalidated = true;
                return Err(error);
            }
        };
        let result = RsSeedSnapshot {
            session_id: self.session_id.clone(),
            epoch: self.epoch,
            game_id: self.profile.game.id().to_owned(),
            game_code: self.profile.game_code.clone(),
            revision: self.profile.revision,
            language: self.profile.language.clone(),
            current_seed,
            timer,
            verification: "header_revision_boot_anchor_only".into(),
            read_only: true,
        };
        if !matches!(reader.fingerprint(), Ok(fingerprint) if fingerprint == self.fingerprint) {
            self.invalidated = true;
            return Err(RsError::StaleSession);
        }
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, collections::HashMap};
    struct Mock {
        bytes: HashMap<u32, u8>,
        id: RefCell<RuntimeFingerprint>,
    }
    impl Mock {
        fn new(profile: &RsProfile) -> Self {
            let mut bytes = HashMap::new();
            let mut header = [0u8; HEADER_LEN];
            let title = match profile.game {
                RsGame::Ruby => b"POKEMON RUBY".as_slice(),
                RsGame::Sapphire => b"POKEMON SAPP".as_slice(),
            };
            header[0xA0..0xAC].copy_from_slice(title);
            header[0xAC..0xB0].copy_from_slice(profile.game_code.as_bytes());
            header[0xB2] = 0x96;
            header[0xBC] = profile.revision;
            header[0xBD] = header[0xA0..=0xBC]
                .iter()
                .fold(0x19u8, |s, b| s.wrapping_add(*b))
                .wrapping_neg();
            fn write(bytes: &mut HashMap<u32, u8>, addr: u32, data: &[u8]) {
                for (i, b) in data.iter().enumerate() {
                    bytes.insert(addr + i as u32, *b);
                }
            }
            write(&mut bytes, ROM_BASE, &header);
            write(
                &mut bytes,
                ROM_ANCHOR_ADDRESS,
                anchor_for(&profile.game_code).unwrap(),
            );
            write(
                &mut bytes,
                profile.seed_address,
                &0xDEADBEEFu32.to_le_bytes(),
            );
            write(&mut bytes, profile.timer_address, &0x1234u16.to_le_bytes());
            Self {
                bytes,
                id: RefCell::new(RuntimeFingerprint {
                    process_id: 8841,
                    process_created_at: 4000,
                    host_rom_base: 0x1000,
                    host_ewram_base: 0x2000,
                    host_iwram_base: 0x3000,
                    host_rom_pointer_field: 0x4000,
                    game_code: profile.game_code.clone(),
                    revision: profile.revision,
                }),
            }
        }
    }
    impl GuestMemoryReader for Mock {
        fn read_guest(&self, address: u32, out: &mut [u8]) -> Result<(), trainer_mgba::ProbeError> {
            for (i, v) in out.iter_mut().enumerate() {
                *v = *self.bytes.get(&(address + i as u32)).ok_or_else(|| {
                    trainer_mgba::ProbeError::Read {
                        address,
                        detail: "unmapped fixture".into(),
                    }
                })?;
            }
            Ok(())
        }
        fn fingerprint(&self) -> Result<RuntimeFingerprint, trainer_mgba::ProbeError> {
            Ok(self.id.borrow().clone())
        }
    }
    #[test]
    fn all_26_retail_codes_revisions_are_independent() {
        let all = retail_profiles();
        assert_eq!(all.len(), 26);
        assert_eq!(all.iter().filter(|p| p.game == RsGame::Ruby).count(), 13);
        assert_eq!(
            all.iter().filter(|p| p.game == RsGame::Sapphire).count(),
            13
        );
        for p in all {
            let reader = Mock::new(&p);
            let mut session = RsReadOnlySession::attach(&reader).unwrap();
            let snap = session.snapshot(&reader).unwrap();
            assert_eq!(snap.current_seed, 0xDEADBEEF);
            assert_eq!(snap.timer, 0x1234);
            assert_eq!(snap.revision, p.revision);
            assert_eq!(snap.game_id, p.game.id());
            assert!(snap.read_only);
            assert!(!p.write_allowed);
        }
    }
    #[test]
    fn language_layouts_never_inherit_emerald_offsets() {
        assert_eq!(profile_for("AXVE", 0).unwrap().seed_address, 0x03004818);
        assert_eq!(profile_for("AXPJ", 1).unwrap().seed_address, 0x03004748);
        assert_eq!(profile_for("AXPD", 0).unwrap().seed_address, 0x03004828);
        assert_eq!(profile_for("AXPS", 1).unwrap().layout, RsLayout::Europe);
    }
    #[test]
    fn bad_revisions_and_unknown_codes_are_rejected() {
        for (code, rev) in [
            ("AXVD", 2),
            ("AXVJ", 2),
            ("AXPE", 3),
            ("BPEE", 0),
            ("AXVX", 0),
            ("AXPJ", 9),
        ] {
            assert!(profile_for(code, rev).is_err());
        }
    }
    #[test]
    fn known_german_debug_anchor_mismatch_is_rejected() {
        let p = profile_for("AXVD", 0).unwrap();
        let mut reader = Mock::new(&p);
        reader.bytes.insert(ROM_ANCHOR_ADDRESS, 0xFF);
        assert!(matches!(
            RsReadOnlySession::attach(&reader),
            Err(RsError::AnchorMismatch)
        ));
    }
    #[test]
    fn title_code_checksum_reject_header_corruption() {
        let p = profile_for("AXVE", 1).unwrap();
        let mut reader = Mock::new(&p);
        reader.bytes.insert(ROM_BASE + 0xBD, 0xFF);
        assert!(matches!(
            RsReadOnlySession::attach(&reader),
            Err(RsError::Header(_))
        ));
        let mut other = Mock::new(&p);
        other.bytes.insert(ROM_BASE + 0xA0, b'X');
        assert!(RsReadOnlySession::attach(&other).is_err());
    }
    #[test]
    fn change_in_process_identity_invalidates_readonly_session() {
        let p = profile_for("AXPS", 1).unwrap();
        let reader = Mock::new(&p);
        let mut session = RsReadOnlySession::attach(&reader).unwrap();
        reader.id.borrow_mut().host_rom_base += 1;
        assert!(matches!(
            session.snapshot(&reader),
            Err(RsError::StaleSession)
        ));
        assert!(matches!(
            session.snapshot(&reader),
            Err(RsError::Invalidated)
        ));
    }
    #[test]
    fn changed_profile_or_region_is_not_reused() {
        let p = profile_for("AXPJ", 1).unwrap();
        let reader = Mock::new(&p);
        let mut session = RsReadOnlySession::attach(&reader).unwrap();
        reader.id.borrow_mut().game_code = "AXVE".to_owned();
        assert!(session.snapshot(&reader).is_err());
    }
    #[test]
    fn guest_read_error_permanently_invalidates_session() {
        let profile = profile_for("AXVE", 2).unwrap();
        let mut reader = Mock::new(&profile);
        let mut session = RsReadOnlySession::attach(&reader).unwrap();
        reader.bytes.remove(&profile.seed_address);
        assert!(matches!(
            session.snapshot(&reader),
            Err(RsError::Read(_, _))
        ));
        assert!(matches!(
            session.snapshot(&reader),
            Err(RsError::Invalidated)
        ));
    }
}
