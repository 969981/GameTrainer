//! HGSS v1.4.5.1 read-only profile and NDS header identities.
//! Values transcribed from the user's HGSS v1.4.5.1 source archive.
//! In-memory Code+Revision is NOT proof of a full-ROM SHA1 match.
//! No guest writes, battle Hook, arbitrary address or patch operations exist here.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub mod m6;

pub const SOURCE_VERSION: &str = "1.4.5.1";
pub const PROFILES_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/hgss-v1451-profiles.json"
));

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HgssProfile {
    pub game_code: String,
    pub revision: u8,
    pub game_id: String,
    pub game: String,
    pub language: String,
    pub sha1: String,
    pub current_seed_addr: u32,
    pub mt_state0_addr: u32,
    pub mt_index_addr: u32,
    pub pid_pointer_addr: u32,
    pub trainer_ids_pointer_addr: u32,
    pub delay_addr: u32,
    pub static_audit_1451: bool,
    pub write_allowed: bool,
}

pub fn retail_profiles() -> Vec<HgssProfile> {
    serde_json::from_str(PROFILES_JSON).expect("checked-in HGSS manifest must be valid")
}

pub fn profile_for(code: &str, revision: u8) -> Option<HgssProfile> {
    retail_profiles()
        .into_iter()
        .find(|p| p.game_code == code && p.revision == revision)
}

/// The ROM header has game code at 0x0C and revision at 0x1E.
/// Header-only identification does NOT authorize writes or prove an unmodified ROM.
pub fn parse_rom_header(header: &[u8]) -> Option<(String, u8)> {
    let code = std::str::from_utf8(header.get(0x0C..0x10)?).ok()?;
    if !(code.starts_with("IPK") || code.starts_with("IPG"))
        || !code.bytes().all(|c| c.is_ascii_uppercase())
    {
        return None;
    }
    Some((code.to_string(), *header.get(0x1E)?))
}

/// Compare a complete file SHA1 supplied by an offline verifier; never use
/// this front-end argument to authorize live memory mutation.
pub fn matches_offline_rom(code: &str, revision: u8, file_sha1: &str) -> bool {
    profile_for(code, revision)
        .is_some_and(|p| p.sha1.eq_ignore_ascii_case(file_sha1) && file_sha1.len() == 40)
}

pub fn valid_profile_catalog() -> bool {
    let profiles = retail_profiles();
    let mut pairs = HashSet::new();
    profiles.len() == 14
        && profiles.iter().all(|p| {
            pairs.insert((p.game_code.clone(), p.revision))
                && p.sha1.len() == 40
                && p.sha1.bytes().all(|b| b.is_ascii_hexdigit())
                && !p.write_allowed
                && p.mt_state0_addr == p.current_seed_addr + 4
                && (0x0200_0000..0x0240_0000).contains(&p.mt_index_addr)
                && (0x0200_0000..0x0240_0000).contains(&p.pid_pointer_addr)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profiles_are_unique_source_backed_and_read_only() {
        assert!(valid_profile_catalog());
        let profiles = retail_profiles();
        assert_eq!(profiles.iter().filter(|p| p.static_audit_1451).count(), 12);
        assert_eq!(profiles.iter().filter(|p| p.game == "HeartGold").count(), 7);
        assert_eq!(
            profiles.iter().filter(|p| p.game == "SoulSilver").count(),
            7
        );
    }
    #[test]
    fn europ_english_revision_not_usa_revision() {
        let us = profile_for("IPKE", 0).unwrap();
        let eu = profile_for("IPKE", 10).unwrap();
        assert_ne!(us.sha1, eu.sha1);
        assert_ne!(us.pid_pointer_addr, eu.pid_pointer_addr);
        assert!(!us.static_audit_1451 && eu.static_audit_1451);
        assert!(profile_for("IPKE", 9).is_none());
    }
    #[test]
    fn korean_and_unknown_revisions_rejected() {
        assert!(profile_for("IPKK", 0).is_none());
        assert!(profile_for("IPGK", 0).is_none());
        assert!(profile_for("IPKJ", 1).is_none());
    }
    #[test]
    fn header_has_correct_nds_offsets() {
        let mut h = [0u8; 0x200];
        h[0x0c..0x10].copy_from_slice(b"IPGE");
        h[0x1e] = 10;
        assert_eq!(parse_rom_header(&h), Some(("IPGE".into(), 10)));
        assert!(profile_for("IPGE", 10).is_some());
        assert!(parse_rom_header(&h[..0x1e]).is_none());
        h[0x0c..0x10].copy_from_slice(b"IPGK");
        assert!(profile_for(&parse_rom_header(&h).unwrap().0, 10).is_none());
    }
    #[test]
    fn full_hash_must_match_catalog() {
        let p = profile_for("IPKJ", 0).unwrap();
        assert!(matches_offline_rom("IPKJ", 0, &p.sha1));
        assert!(!matches_offline_rom("IPKJ", 0, &"0".repeat(40)));
        assert!(!matches_offline_rom("IPKJ", 1, &p.sha1));
    }
}
