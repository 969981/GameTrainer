//! M8 core-loaded DeSmuME ROM identity protocol.
//! The source of this packet is the DeSmuME NDS_LoadROM byte stream, not a
//! file path chosen by the user. OS process ownership must be checked natively.
//! This crate does NOT authorize NDS writes, CPU/JIT coherence, or restoration.
use crate::profile_for;
use serde::{Deserialize, Serialize};

pub const MAGIC: &[u8; 8] = b"GTHGSS08";
pub const VERSION: u32 = 1;
pub const WIRE_SIZE: usize = 76;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgePacket {
    pub pid: u32,
    pub created_filetime: u64,
    pub rom_epoch: u64,
    pub code: String,
    pub revision: u8,
    pub size: u64,
    pub crc32: u32,
    pub sha1: [u8; 20],
    pub loaded: bool,
    pub arm9_port: u16,
    pub arm7_port: u16,
    pub jit_at_load: bool,
    pub hash_complete: bool,
}
fn le16(x: &[u8]) -> u16 {
    u16::from_le_bytes([x[0], x[1]])
}
fn le32(x: &[u8]) -> u32 {
    u32::from_le_bytes(x.try_into().expect("4-byte slice"))
}
fn le64(x: &[u8]) -> u64 {
    u64::from_le_bytes(x.try_into().expect("8-byte slice"))
}
pub fn decode_packet(b: &[u8]) -> Result<BridgePacket, String> {
    if b.len() != WIRE_SIZE {
        return Err("M8 pipe packet size mismatch".into());
    }
    if &b[0..8] != MAGIC || le32(&b[8..12]) != VERSION {
        return Err("invalid or unsupported M8 bridge header".into());
    }
    let code = std::str::from_utf8(&b[32..36]).map_err(|_| "non-ascii game code")?;
    if !(code.starts_with("IPK") || code.starts_with("IPG"))
        || !code.bytes().all(|x| x.is_ascii_uppercase())
    {
        return Err("not an HGSS retail game code".into());
    }
    if b[69] > 1 || b[74] > 1 || b[75] > 1 {
        return Err("invalid bridge flag".into());
    }
    let mut sha1 = [0u8; 20];
    sha1.copy_from_slice(&b[49..69]);
    Ok(BridgePacket {
        pid: le32(&b[12..16]),
        created_filetime: le64(&b[16..24]),
        rom_epoch: le64(&b[24..32]),
        code: code.into(),
        revision: b[36],
        size: le64(&b[37..45]),
        crc32: le32(&b[45..49]),
        sha1,
        loaded: b[69] != 0,
        arm9_port: le16(&b[70..72]),
        arm7_port: le16(&b[72..74]),
        jit_at_load: b[74] != 0,
        hash_complete: b[75] != 0,
    })
}
#[derive(Clone, Copy, Debug)]
pub struct OsBinding {
    pub pipe_server_pid_matches: bool,
    pub creation_filetime_matches: bool,
    pub gdb_ports_owned_by_pid: bool,
    pub process_still_same: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct M8Report {
    pub process_id: u32,
    pub process_created_filetime: u64,
    pub rom_load_epoch: u64,
    pub code: String,
    pub revision: u8,
    pub rom_size: u64,
    pub rom_crc32: String,
    pub rom_sha1: String,
    pub catalog_sha1: Option<String>,
    pub arm9_port: u16,
    pub arm7_port: u16,
    pub jit_enabled_when_loaded: bool,
    pub core_rom_stream_complete: bool,
    pub pipe_process_identity_verified: bool,
    pub gdb_socket_pid_binding_verified: bool,
    pub running_rom_digest_matches_catalog: bool,
    pub diagnostic_identity_verified: bool,
    pub dual_cpu_atomic_stop_verified: bool,
    pub jit_cache_coherence_verified: bool,
    pub overlay_restore_verified: bool,
    pub write_authorized: bool,
    pub blockers: Vec<String>,
    pub evidence_scope: String,
}
fn sha1_hex(x: &[u8; 20]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(40);
    for b in x {
        s.push(HEX[(*b >> 4) as usize] as char);
        s.push(HEX[(*b & 15) as usize] as char);
    }
    s
}
pub fn evaluate(
    p: &BridgePacket,
    expected_pid: u32,
    expected_code: &str,
    expected_revision: u8,
    expected_arm9: u16,
    expected_arm7: u16,
    os: OsBinding,
) -> M8Report {
    let prof = profile_for(expected_code, expected_revision);
    let hash = sha1_hex(&p.sha1);
    let pipe_ok = p.pid != 0
        && p.pid == expected_pid
        && p.created_filetime != 0
        && os.pipe_server_pid_matches
        && os.creation_filetime_matches
        && os.process_still_same;
    let gdb_ok = os.gdb_ports_owned_by_pid
        && p.arm9_port == expected_arm9
        && p.arm7_port == expected_arm7
        && p.arm9_port >= 1024
        && p.arm7_port >= 1024
        && p.arm9_port != p.arm7_port;
    let catalog_ok = prof.as_ref().is_some_and(|x| {
        x.static_audit_1451
            && !x.write_allowed
            && p.code == expected_code
            && p.revision == expected_revision
            && x.sha1.eq_ignore_ascii_case(&hash)
    });
    let complete = p.loaded
        && p.hash_complete
        && p.rom_epoch > 0
        && (1024 * 1024..=512 * 1024 * 1024).contains(&p.size);
    let mut blockers = Vec::new();
    if !complete {
        blockers.push("core_loaded_rom_hash_incomplete".into());
    }
    if !pipe_ok {
        blockers.push("pipe_process_creation_identity_mismatch".into());
    }
    if !gdb_ok {
        blockers.push("both_gdb_ports_not_owned_by_pid".into());
    }
    if !catalog_ok {
        blockers.push("loaded_rom_hash_or_revision_not_verified".into());
    }
    blockers.extend([
        "atomic_arm9_arm7_stop_not_proven".into(),
        "jit_instruction_cache_not_proven".into(),
        "overlay_epoch_restore_not_proven".into(),
        "nds_writes_not_implemented".into(),
    ]);
    M8Report {
        process_id: p.pid,
        process_created_filetime: p.created_filetime,
        rom_load_epoch: p.rom_epoch,
        code: p.code.clone(),
        revision: p.revision,
        rom_size: p.size,
        rom_crc32: format!("{:08X}", p.crc32),
        rom_sha1: hash,
        catalog_sha1: prof.map(|x| x.sha1),
        arm9_port: p.arm9_port,
        arm7_port: p.arm7_port,
        jit_enabled_when_loaded: p.jit_at_load,
        core_rom_stream_complete: complete,
        pipe_process_identity_verified: pipe_ok,
        gdb_socket_pid_binding_verified: gdb_ok,
        running_rom_digest_matches_catalog: catalog_ok,
        diagnostic_identity_verified: complete && pipe_ok && gdb_ok && catalog_ok,
        dual_cpu_atomic_stop_verified: false,
        jit_cache_coherence_verified: false,
        overlay_restore_verified: false,
        write_authorized: false,
        blockers,
        evidence_scope: "m8_core_loaded_rom_digest_and_windows_pid_gdb_bindings_read_only".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> BridgePacket {
        let v = profile_for("IPKJ", 0).unwrap();
        let mut sha1 = [0u8; 20];
        for (i, b) in sha1.iter_mut().enumerate() {
            *b = u8::from_str_radix(&v.sha1[i * 2..i * 2 + 2], 16).unwrap();
        }
        BridgePacket {
            pid: 101,
            created_filetime: 123456789,
            rom_epoch: 4,
            code: "IPKJ".into(),
            revision: 0,
            size: 128 * 1024 * 1024,
            crc32: 123456,
            sha1,
            loaded: true,
            arm9_port: 3333,
            arm7_port: 3334,
            jit_at_load: false,
            hash_complete: true,
        }
    }
    fn os() -> OsBinding {
        OsBinding {
            pipe_server_pid_matches: true,
            creation_filetime_matches: true,
            gdb_ports_owned_by_pid: true,
            process_still_same: true,
        }
    }
    fn wire(p: &BridgePacket) -> Vec<u8> {
        let mut x = Vec::new();
        x.extend(MAGIC);
        x.extend(VERSION.to_le_bytes());
        x.extend(p.pid.to_le_bytes());
        x.extend(p.created_filetime.to_le_bytes());
        x.extend(p.rom_epoch.to_le_bytes());
        x.extend(p.code.as_bytes());
        x.push(p.revision);
        x.extend(p.size.to_le_bytes());
        x.extend(p.crc32.to_le_bytes());
        x.extend(p.sha1);
        x.push(p.loaded as u8);
        x.extend(p.arm9_port.to_le_bytes());
        x.extend(p.arm7_port.to_le_bytes());
        x.push(p.jit_at_load as u8);
        x.push(p.hash_complete as u8);
        assert_eq!(x.len(), WIRE_SIZE);
        x
    }
    #[test]
    fn strict_fixed_wire_layout() {
        let p = valid();
        assert_eq!(decode_packet(&wire(&p)).unwrap(), p);
        assert!(decode_packet(&wire(&p)[..75]).is_err());
        let mut w = wire(&p);
        w[0] = 0;
        assert!(decode_packet(&w).is_err());
        let mut w = wire(&p);
        w[69] = 3;
        assert!(decode_packet(&w).is_err());
    }
    #[test]
    fn valid_core_bytes_remain_non_authorizing() {
        let r = evaluate(&valid(), 101, "IPKJ", 0, 3333, 3334, os());
        assert!(r.diagnostic_identity_verified);
        assert!(!r.write_authorized && !r.dual_cpu_atomic_stop_verified);
        assert!(r
            .blockers
            .contains(&"jit_instruction_cache_not_proven".into()));
    }
    #[test]
    fn wrong_digest_revision_and_legacy_us_never_match() {
        let mut p = valid();
        p.sha1 = [0; 20];
        assert!(!evaluate(&p, 101, "IPKJ", 0, 3333, 3334, os()).diagnostic_identity_verified);
        let mut p = valid();
        p.revision = 1;
        assert!(!evaluate(&p, 101, "IPKJ", 0, 3333, 3334, os()).diagnostic_identity_verified);
        let p = valid();
        assert!(!evaluate(&p, 101, "IPKE", 0, 3333, 3334, os()).diagnostic_identity_verified);
    }
    #[test]
    fn pipe_pid_creation_or_network_binding_fail_closed() {
        let p = valid();
        let mut o = os();
        o.pipe_server_pid_matches = false;
        assert!(!evaluate(&p, 101, "IPKJ", 0, 3333, 3334, o).diagnostic_identity_verified);
        let mut o = os();
        o.creation_filetime_matches = false;
        assert!(!evaluate(&p, 101, "IPKJ", 0, 3333, 3334, o).diagnostic_identity_verified);
        let mut o = os();
        o.gdb_ports_owned_by_pid = false;
        assert!(!evaluate(&p, 101, "IPKJ", 0, 3333, 3334, o).diagnostic_identity_verified);
        assert!(!evaluate(&p, 101, "IPKJ", 0, 3333, 3335, os()).diagnostic_identity_verified);
    }
    #[test]
    fn incomplete_rom_stream_and_missing_epoch_rejected() {
        let mut p = valid();
        p.hash_complete = false;
        assert!(!evaluate(&p, 101, "IPKJ", 0, 3333, 3334, os()).diagnostic_identity_verified);
        let mut p = valid();
        p.rom_epoch = 0;
        assert!(!evaluate(&p, 101, "IPKJ", 0, 3333, 3334, os()).diagnostic_identity_verified);
    }
}
