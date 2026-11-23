//! FireRed/LeafGreen v2.0.4 source-verified immutable retail profile table.
//! Full ROM SHA-1 is checked in the separate trusted GDB live editor.
//! CRC32 and headers are display evidence, NEVER mutation authorization.
pub mod live;

use serde::{Deserialize, Serialize};
use thiserror::Error;

const ROMS: &str = include_str!("../../../fixtures/frlg-v204-roms.json");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrlgRomProfile {
    pub profile_id: String,
    pub game_id: String,
    pub game_code: String,
    pub revision: u8,
    pub language: String,
    pub crc32: String,
    pub sha1: String,
    pub rom_bytes: usize,
    pub capture_entry: u32,
    pub capture_patch: u32,
    pub wall_patch: u32,
    pub wild_entry: u32,
    pub callback2: u32,
    pub rng_seed: u32,
    pub expected_capture_entry: String,
    pub expected_capture_context: String,
    pub expected_wall_context: String,
    pub expected_wild_entry: String,
}

#[derive(Deserialize)]
struct Profiles {
    roms: Vec<FrlgRomProfile>,
}

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("unsupported FireRed/LeafGreen ROM game code or revision")]
    Unsupported,
    #[error("invalid FRLG immutable profile manifest: {0}")]
    Manifest(#[from] serde_json::Error),
}

pub fn retail_profiles() -> Result<Vec<FrlgRomProfile>, ProfileError> {
    Ok(serde_json::from_str::<Profiles>(ROMS)?.roms)
}

pub fn profile_for(code: &str, revision: u8) -> Result<FrlgRomProfile, ProfileError> {
    if code.len() != 4
        || !matches!(&code[..3], "BPR" | "BPG")
        || !matches!(code.as_bytes()[3], b'E' | b'J' | b'D' | b'F' | b'I' | b'S')
    {
        return Err(ProfileError::Unsupported);
    }
    retail_profiles()?
        .into_iter()
        .find(|profile| profile.game_code == code && profile.revision == revision)
        .ok_or(ProfileError::Unsupported)
}

pub fn profile_from_header(header: &[u8]) -> Result<FrlgRomProfile, ProfileError> {
    if header.len() < 0xBD {
        return Err(ProfileError::Unsupported);
    }
    let code = std::str::from_utf8(&header[0xAC..0xB0])
        .map_err(|_| ProfileError::Unsupported)?;
    profile_for(code, header[0xBC])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retail_matrix_has_16_unique_exact_sha1_records() {
        let profiles = retail_profiles().unwrap();
        assert_eq!(profiles.len(), 16);
        let mut keys = std::collections::HashSet::new();
        let mut digests = std::collections::HashSet::new();
        for profile in &profiles {
            assert!(keys.insert(profile.profile_id.clone()));
            assert!(digests.insert(profile.sha1.clone()));
            assert_eq!(profile.sha1.len(), 40);
            assert_eq!(profile.rom_bytes, 16 * 1024 * 1024);
            assert_eq!(profile.capture_entry % 2, 0);
            assert_eq!(profile.capture_patch % 2, 0);
            assert_eq!(profile.wall_patch % 2, 0);
            assert_eq!(profile.expected_capture_context.len(), 20);
            assert_eq!(profile.expected_wall_context.len(), 20);
            assert_eq!(profile.expected_wild_entry.len(), 16);
            assert!(profile_for(&profile.game_code, profile.revision).is_ok());
        }
    }
    #[test]
    fn unknown_hacks_and_unverified_revisions_are_rejected() {
        for (code, rev) in [("AXVE", 0), ("BPEE", 0), ("BPRE", 2), ("BPRD", 1), ("BPGE", 7), ("BPRZ", 0)] {
            assert!(profile_for(code, rev).is_err());
        }
        assert_eq!(profile_for("BPRE", 0).unwrap().crc32, "DD88761C");
        assert_eq!(profile_for("BPGE", 1).unwrap().crc32, "DAFFECEC");
        assert_eq!(profile_for("BPRJ", 1).unwrap().crc32, "BB640DF7");
    }
    #[test]
    fn gba_header_selects_profile_by_code_and_revision() {
        let mut header = [0u8; 0xC0];
        header[0xAC..0xB0].copy_from_slice(b"BPGJ");
        header[0xBC] = 1;
        assert_eq!(profile_from_header(&header).unwrap().profile_id, "BPGJ_R1");
        header[0xBC] = 3;
        assert!(profile_from_header(&header).is_err());
    }
}
