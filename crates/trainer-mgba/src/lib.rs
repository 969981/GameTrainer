//! M1: read-only Emerald ROM-header/anchor verification and RNG snapshots.
//! The transport is abstract: no arbitrary host memory operations or writes.
//! The anchor set is PARTIAL compared with the original GPL-3.0 CE source.
//! Never use anchor verification from this crate as authorization for patching.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

const GBA_ROM: u32 = 0x0800_0000;
const GBA_EWRAM: u32 = 0x0200_0000;
const GBA_IWRAM: u32 = 0x0300_0000;
const ROM_HEADER_SIZE: usize = 0xC0;
const SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeFingerprint {
    pub process_id: u32,
    /// Win32 process creation time; distinguish PID reuse.
    pub process_created_at: u64,
    pub host_rom_base: u64,
    pub host_ewram_base: u64,
    pub host_iwram_base: u64,
    pub host_rom_pointer_field: u64,
    pub game_code: String,
    pub revision: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStrength {
    /// Header, revision, Nintendo header checksum and selected source anchors.
    PartialAnchorsReadOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmeraldProfile {
    pub code: &'static str,
    pub language: &'static str,
    pub expected_rom_crc32: &'static str,
    pub seed_address: u32,
    pub initial_seed_address: u32,
    pub advances_address: u32,
    pub wild_anchor: u32,
    pub fishing_anchor: u32,
    pub warp_anchor: u32,
    pub do_warp_anchor: u32,
    pub land_anchor: u32,
    pub water_anchor: u32,
    pub do_warp_expected: &'static [u8],
}

const DO_WARP_JPN: [u8; 12] = [0, 0xB5, 0xE9, 0xF7, 0xD3, 0xFB, 0xD6, 0xF7, 0x1F, 0xF9, 0xFF, 0xF7];
const DO_WARP_COMMON: [u8; 12] = [0, 0xB5, 0xE9, 0xF7, 0x8D, 0xFB, 0xD6, 0xF7, 0xE9, 0xF8, 0xFF, 0xF7];
const DO_WARP_FRA: [u8; 12] = [0, 0xB5, 0xE9, 0xF7, 0x8B, 0xFB, 0xD6, 0xF7, 0xE7, 0xF8, 0xFF, 0xF7];
const WILD: &[u8] = &[0x70, 0xB5, 0x81, 0xB0, 0x05, 0x1C, 0x09, 0x06];
const FISHING: &[u8] = &[0x70, 0xB5, 0, 0x06, 0x04, 0x0E, 0x26, 0x1C];
const WARP: &[u8] = &[0x70, 0xB5, 0x82, 0xB0, 0x04, 0x1C, 0x0D, 0x1C];
const LAND: &[u8] = &[0x20, 0x1C, 0, 0x21, 0, 0x22];
const WATER: &[u8] = &[0x20, 0x1C, 0x01, 0x21, 0, 0x22];

pub const PROFILES: [EmeraldProfile; 6] = [
    EmeraldProfile {code:"BPEJ",language:"JPN",expected_rom_crc32:"4881F3F8",seed_address:0x03005AE0,initial_seed_address:0x02020000,advances_address:0x02024664,wild_anchor:0x080B46A4,fishing_anchor:0x080B4330,warp_anchor:0x08084554,do_warp_anchor:0x080AF030,land_anchor:0x080B4DD8,water_anchor:0x080B4E38,do_warp_expected:&DO_WARP_JPN},
    EmeraldProfile {code:"BPEE",language:"ENG",expected_rom_crc32:"1F1C08FB",seed_address:0x03005D80,initial_seed_address:0x02020000,advances_address:0x020249C0,wild_anchor:0x080B4F4C,fishing_anchor:0x080B4BD8,warp_anchor:0x08084BEC,do_warp_anchor:0x080AF734,land_anchor:0x080B5680,water_anchor:0x080B56E0,do_warp_expected:&DO_WARP_COMMON},
    EmeraldProfile {code:"BPED",language:"DEU",expected_rom_crc32:"34C9DF89",seed_address:0x03005D80,initial_seed_address:0x02020000,advances_address:0x020249C0,wild_anchor:0x080B4F68,fishing_anchor:0x080B4BF4,warp_anchor:0x08084C08,do_warp_anchor:0x080AF750,land_anchor:0x080B569C,water_anchor:0x080B56FC,do_warp_expected:&DO_WARP_COMMON},
    EmeraldProfile {code:"BPEF",language:"FRA",expected_rom_crc32:"A3FDCCB1",seed_address:0x03005D80,initial_seed_address:0x02020000,advances_address:0x020249C0,wild_anchor:0x080B4F60,fishing_anchor:0x080B4BEC,warp_anchor:0x08084BFC,do_warp_anchor:0x080AF748,land_anchor:0x080B5694,water_anchor:0x080B56F4,do_warp_expected:&DO_WARP_FRA},
    EmeraldProfile {code:"BPEI",language:"ITA",expected_rom_crc32:"A0AEC80A",seed_address:0x03005D80,initial_seed_address:0x02020000,advances_address:0x020249C0,wild_anchor:0x080B4F60,fishing_anchor:0x080B4BEC,warp_anchor:0x08084C00,do_warp_anchor:0x080AF748,land_anchor:0x080B5694,water_anchor:0x080B56F4,do_warp_expected:&DO_WARP_COMMON},
    EmeraldProfile {code:"BPES",language:"ESP",expected_rom_crc32:"8C4D3108",seed_address:0x03005D80,initial_seed_address:0x02020000,advances_address:0x020249C0,wild_anchor:0x080B4F60,fishing_anchor:0x080B4BEC,warp_anchor:0x08084C00,do_warp_anchor:0x080AF748,land_anchor:0x080B5694,water_anchor:0x080B56F4,do_warp_expected:&DO_WARP_COMMON},
];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProbeError {
    #[error("guest memory read failed at {address:#010x}: {detail}")]
    Read { address: u32, detail: String },
    #[error("invalid Emerald ROM header: {0}")]
    Header(&'static str),
    #[error("unknown or unsupported Emerald ROM: {0}")]
    Unsupported(String),
    #[error("ROM bytes do not match source-derived anchor at {0:#010x}")]
    AnchorMismatch(u32),
    #[error("session is stale; rediscover the runtime before reading again")]
    Stale,
    #[error("session is already invalidated")]
    Invalidated,
}

pub trait GuestMemoryReader {
    /// All target accesses MUST remain read-only.
    fn read_guest(&self, guest_addr: u32, out: &mut [u8]) -> Result<(), ProbeError>;
    /// Must independently re-read process identity and ROM/EWRAM/IWRAM mapping.
    fn fingerprint(&self) -> Result<RuntimeFingerprint, ProbeError>;
}

fn bytes<R: GuestMemoryReader>(reader: &R, address: u32, len: usize) -> Result<Vec<u8>, ProbeError> {
    let mut data = vec![0; len];
    reader.read_guest(address, &mut data)?;
    Ok(data)
}

pub fn profile_for(code: &str) -> Option<&'static EmeraldProfile> {
    PROFILES.iter().find(|profile| profile.code == code)
}

pub fn parse_header(header: &[u8]) -> Result<(&'static EmeraldProfile, u8), ProbeError> {
    if header.len() < ROM_HEADER_SIZE { return Err(ProbeError::Header("short header")); }
    if !header[0xA0..0xAC].starts_with(b"POKEMON EMER") {
        return Err(ProbeError::Header("not Pokémon Emerald"));
    }
    if header[0xB2] != 0x96 { return Err(ProbeError::Header("invalid GBA fixed value")); }
    let checksum = header[0xA0..=0xBC].iter().fold(0x19u8, |sum, n| sum.wrapping_add(*n)).wrapping_neg();
    if header[0xBD] != checksum { return Err(ProbeError::Header("GBA header checksum mismatch")); }
    let code = std::str::from_utf8(&header[0xAC..0xB0]).map_err(|_| ProbeError::Header("invalid game code"))?;
    let revision = header[0xBC];
    let profile = profile_for(code).ok_or_else(|| ProbeError::Unsupported(code.to_string()))?;
    if revision != 0 { return Err(ProbeError::Unsupported(format!("{code} Rev {revision}"))); }
    Ok((profile, revision))
}

/// Check a deliberately limited subset of the older CE anchor table.
/// This NEVER grants write permission, and is not a full ROM checksum.
pub fn verify_partial_rom_anchors<R: GuestMemoryReader>(reader: &R, profile: &EmeraldProfile) -> Result<(), ProbeError> {
    for (address, expected) in [
        (profile.wild_anchor, WILD),
        (profile.fishing_anchor, FISHING),
        (profile.warp_anchor, WARP),
        (profile.do_warp_anchor, profile.do_warp_expected),
        (profile.land_anchor, LAND),
        (profile.water_anchor, WATER),
    ] {
        if bytes(reader, address, expected.len())? != expected {
            return Err(ProbeError::AnchorMismatch(address));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RngSnapshot {
    pub session_id: String,
    pub epoch: u64,
    pub game_code: String,
    pub language: String,
    pub current_seed: u32,
    pub initial_seed: u16,
    pub advances: u32,
    pub verification: ProbeStrength,
    pub read_only: bool,
}

pub struct EmeraldReadOnlySession {
    id: String,
    epoch: u64,
    fingerprint: RuntimeFingerprint,
    profile: &'static EmeraldProfile,
    invalidated: bool,
}

impl EmeraldReadOnlySession {
    pub fn attach<R: GuestMemoryReader>(reader: &R) -> Result<Self, ProbeError> {
        let fingerprint = reader.fingerprint()?;
        let header = bytes(reader, GBA_ROM, ROM_HEADER_SIZE)?;
        let (profile, revision) = parse_header(&header)?;
        if fingerprint.game_code != profile.code || fingerprint.revision != revision {
            return Err(ProbeError::Stale);
        }
        verify_partial_rom_anchors(reader, profile)?;
        if fingerprint != reader.fingerprint()? { return Err(ProbeError::Stale); }
        let epoch = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
        let id = format!("mgba-m1-{}-{}-{epoch}", fingerprint.process_id, fingerprint.process_created_at);
        Ok(Self { id, epoch, fingerprint, profile, invalidated: false })
    }

    pub fn session_id(&self) -> &str { &self.id }
    pub fn profile(&self) -> &EmeraldProfile { self.profile }
    pub fn read_snapshot<R: GuestMemoryReader>(&mut self, reader: &R) -> Result<RngSnapshot, ProbeError> {
        if self.invalidated { return Err(ProbeError::Invalidated); }
        if reader.fingerprint()? != self.fingerprint {
            self.invalidated = true;
            return Err(ProbeError::Stale);
        }
        let snapshot = RngSnapshot {
            session_id: self.id.clone(),
            epoch: self.epoch,
            game_code: self.profile.code.to_string(),
            language: self.profile.language.to_string(),
            current_seed: read_u32(reader, self.profile.seed_address)?,
            initial_seed: read_u16(reader, self.profile.initial_seed_address)?,
            advances: read_u32(reader, self.profile.advances_address)?,
            verification: ProbeStrength::PartialAnchorsReadOnly,
            read_only: true,
        };
        // Prevent mixed-session snapshots when ROM or core mappings change mid-read.
        if reader.fingerprint()? != self.fingerprint {
            self.invalidated = true;
            return Err(ProbeError::Stale);
        }
        Ok(snapshot)
    }
}
fn read_u16<R: GuestMemoryReader>(reader: &R, address: u32) -> Result<u16, ProbeError> {
    Ok(u16::from_le_bytes(bytes(reader, address, 2)?.try_into().expect("fixed size")))
}
fn read_u32<R: GuestMemoryReader>(reader: &R, address: u32) -> Result<u32, ProbeError> {
    Ok(u32::from_le_bytes(bytes(reader, address, 4)?.try_into().expect("fixed size")))
}

pub fn guest_offset(guest_addr: u32, len: usize) -> Option<(&'static str, usize)> {
    let len = u32::try_from(len).ok()?;
    for (label, base, size) in [
        ("rom", GBA_ROM, 0x0100_0000u32),
        ("ewram", GBA_EWRAM, 0x0004_0000u32),
        ("iwram", GBA_IWRAM, 0x0000_8000u32),
    ] {
        if let Some(off) = guest_addr.checked_sub(base) {
            if off.checked_add(len).is_some_and(|end| end <= size) {
                return Some((label, off as usize));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::{Cell, RefCell}, collections::BTreeMap};

    struct Fake {
        map: BTreeMap<u32, u8>,
        fingerprint: RefCell<RuntimeFingerprint>,
        mutate_on_read: Cell<bool>,
    }
    impl Fake {
        fn new(profile: &'static EmeraldProfile) -> Self {
            let mut map = BTreeMap::new();
            let mut header = [0u8; ROM_HEADER_SIZE];
            header[0xA0..0xAC].copy_from_slice(b"POKEMON EMER");
            header[0xAC..0xB0].copy_from_slice(profile.code.as_bytes());
            header[0xB2] = 0x96;
            header[0xBC] = 0;
            header[0xBD] = header[0xA0..=0xBC].iter().fold(0x19u8, |sum, n| sum.wrapping_add(*n)).wrapping_neg();
            fn write(map: &mut BTreeMap<u32,u8>, addr:u32, bytes:&[u8]) {
                for (index, value) in bytes.iter().enumerate() { map.insert(addr + index as u32,*value); }
            }
            write(&mut map,GBA_ROM,&header);
            for (addr, bytes) in [
                (profile.wild_anchor, WILD),(profile.fishing_anchor,FISHING),
                (profile.warp_anchor,WARP),(profile.do_warp_anchor,profile.do_warp_expected),
                (profile.land_anchor,LAND),(profile.water_anchor,WATER),
            ] { write(&mut map,addr,bytes); }
            write(&mut map,profile.seed_address,&0x1234ABCDu32.to_le_bytes());
            write(&mut map,profile.initial_seed_address,&0xBEEFu16.to_le_bytes());
            write(&mut map,profile.advances_address,&991u32.to_le_bytes());
            Self {
                map, mutate_on_read: Cell::new(false),
                fingerprint: RefCell::new(RuntimeFingerprint {
                    process_id: 1234, process_created_at: 777, host_rom_base: 0x10000,
                    host_ewram_base: 0x20000, host_iwram_base: 0x30000,
                    host_rom_pointer_field: 0x40000,
                    game_code: profile.code.into(),revision: 0,
                }),
            }
        }
    }
    impl GuestMemoryReader for Fake {
        fn read_guest(&self, address:u32, out:&mut[u8]) -> Result<(),ProbeError> {
            for (i,slot) in out.iter_mut().enumerate() {
                *slot = *self.map.get(&(address+i as u32)).ok_or_else(|| ProbeError::Read {address,detail:"unmapped".into()})?;
            }
            if self.mutate_on_read.get() {
                self.mutate_on_read.set(false);
                self.fingerprint.borrow_mut().host_rom_base+=1;
            }
            Ok(())
        }
        fn fingerprint(&self)->Result<RuntimeFingerprint,ProbeError>{
            Ok(self.fingerprint.borrow().clone())
        }
    }

    #[test]
    fn source_profiles_cover_six_retail_rev0_codes() {
        let codes:Vec<_>=PROFILES.iter().map(|p|p.code).collect();
        assert_eq!(codes,vec!["BPEJ","BPEE","BPED","BPEF","BPEI","BPES"]);
        assert_eq!(PROFILES[0].seed_address,0x03005AE0);
        assert_eq!(PROFILES[1].seed_address,0x03005D80);
    }
    #[test]
    fn valid_six_profiles_read_seed() {
        for profile in PROFILES.iter() {
            let fake=Fake::new(profile);
            let mut session=EmeraldReadOnlySession::attach(&fake).unwrap();
            let snapshot=session.read_snapshot(&fake).unwrap();
            assert_eq!(snapshot.current_seed,0x1234ABCD);
            assert_eq!(snapshot.initial_seed,0xBEEF);
            assert_eq!(snapshot.advances,991);
            assert!(snapshot.read_only);
        }
    }
    #[test]
    fn rejects_header_checksum_mismatch() {
        let mut fake=Fake::new(&PROFILES[1]);fake.map.insert(GBA_ROM+0xBD,0);
        assert!(matches!(EmeraldReadOnlySession::attach(&fake),Err(ProbeError::Header(_))));
    }
    #[test]
    fn rejects_unknown_revision() {
        let mut fake=Fake::new(&PROFILES[1]);
        fake.map.insert(GBA_ROM+0xBC,1);
        let mut data=bytes(&fake,GBA_ROM,ROM_HEADER_SIZE).unwrap();
        data[0xBD]=data[0xA0..=0xBC].iter().fold(0x19u8, |sum,n|sum.wrapping_add(*n)).wrapping_neg();
        fake.map.insert(GBA_ROM+0xBD,data[0xBD]);
        assert!(matches!(EmeraldReadOnlySession::attach(&fake),Err(ProbeError::Unsupported(_))));
    }
    #[test]
    fn rejects_modified_anchor_without_ever_writing() {
        let mut fake=Fake::new(&PROFILES[1]);fake.map.insert(PROFILES[1].wild_anchor,0xFF);
        assert!(matches!(EmeraldReadOnlySession::attach(&fake),Err(ProbeError::AnchorMismatch(_))));
    }
    #[test]
    fn invalidates_session_on_mapping_change() {
        let fake=Fake::new(&PROFILES[1]);
        let mut session=EmeraldReadOnlySession::attach(&fake).unwrap();
        fake.fingerprint.borrow_mut().host_rom_base+=1;
        assert_eq!(session.read_snapshot(&fake).unwrap_err(),ProbeError::Stale);
        assert_eq!(session.read_snapshot(&fake).unwrap_err(),ProbeError::Invalidated);
    }
    #[test]
    fn invalidates_when_fingerprint_changes_mid_read() {
        let fake=Fake::new(&PROFILES[0]);
        let mut session=EmeraldReadOnlySession::attach(&fake).unwrap();
        fake.mutate_on_read.set(true);
        assert_eq!(session.read_snapshot(&fake).unwrap_err(),ProbeError::Stale);
    }
    #[test]
    fn rejects_wrong_process_creation_time() {
        let fake=Fake::new(&PROFILES[1]);
        let mut session=EmeraldReadOnlySession::attach(&fake).unwrap();
        fake.fingerprint.borrow_mut().process_created_at+=1;
        assert_eq!(session.read_snapshot(&fake).unwrap_err(),ProbeError::Stale);
    }
    #[test]
    fn memory_ranges_are_bounded() {
        assert_eq!(guest_offset(0x0300_5D80,4),Some(("iwram",0x5D80)));
        assert_eq!(guest_offset(0x0202_0000,2),Some(("ewram",0x20000)));
        assert_eq!(guest_offset(0x0800_00AC,4),Some(("rom",0xAC)));
        assert_eq!(guest_offset(0x0300_7FFF,2),None);
        assert_eq!(guest_offset(0x0,4),None);
    }
}
