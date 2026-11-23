//! Diamond/Pearl M10: in-core DeSmuME loaded ROM SHA256 + Win32 identity.
//! This packet is independent from HGSS M8's 76-byte SHA1 pipe. Its source
//! is the *running emulator's in-memory ROM Reader*, not a file picker.
//! ROM identity, even when complete, never authorizes Guest writes.
use crate::{profile_for, DpReadOnlySnapshot};
use serde::{Deserialize, Serialize};

pub const WIRE_SIZE: usize = 89;
const MAGIC: &[u8; 8] = b"GTPDPM10";
const VERSION: u32 = 1;
fn le16(x: &[u8]) -> u16 {
    u16::from_le_bytes([x[0], x[1]])
}
fn le32(x: &[u8]) -> u32 {
    u32::from_le_bytes(x.try_into().expect("4 bytes"))
}
fn le64(x: &[u8]) -> u64 {
    u64::from_le_bytes(x.try_into().expect("8 bytes"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet {
    pub pid: u32,
    pub process_created_filetime: u64,
    pub rom_epoch: u64,
    pub game_code: String,
    pub revision: u8,
    pub header6c: u8,
    pub size: u64,
    pub crc32: u32,
    pub sha256: [u8; 32],
    pub loaded: bool,
    pub arm9_port: u16,
    pub arm7_port: u16,
    pub jit_at_load: bool,
    pub hash_complete: bool,
}
pub fn decode_packet(bytes: &[u8]) -> Result<Packet, String> {
    if bytes.len() != WIRE_SIZE {
        return Err("M10 pipe response size mismatch".into());
    }
    if &bytes[0..8] != MAGIC || le32(&bytes[8..12]) != VERSION {
        return Err("M10 protocol magic/version unsupported".into());
    }
    let code = std::str::from_utf8(&bytes[32..36]).map_err(|_| "M10 invalid ROM Code UTF-8")?;
    if code.len() != 4
        || !code.starts_with('A')
        || !matches!(code.as_bytes()[1], b'P' | b'D')
        || code.as_bytes()[2] != b'A'
        || !matches!(code.as_bytes()[3], b'E' | b'D' | b'F' | b'I' | b'S')
    {
        return Err("M10 ROM Code outside reviewed Western Diamond/Pearl profiles".into());
    }
    if bytes[82] > 1 || bytes[87] > 1 || bytes[88] > 1 {
        return Err("M10 invalid boolean flag".into());
    }
    let mut sha256 = [0u8; 32];
    sha256.copy_from_slice(&bytes[50..82]);
    Ok(Packet {
        pid: le32(&bytes[12..16]),
        process_created_filetime: le64(&bytes[16..24]),
        rom_epoch: le64(&bytes[24..32]),
        game_code: code.into(),
        revision: bytes[36],
        header6c: bytes[37],
        size: le64(&bytes[38..46]),
        crc32: le32(&bytes[46..50]),
        sha256,
        loaded: bytes[82] != 0,
        arm9_port: le16(&bytes[83..85]),
        arm7_port: le16(&bytes[85..87]),
        jit_at_load: bytes[87] != 0,
        hash_complete: bytes[88] != 0,
    })
}
#[derive(Clone, Copy, Debug)]
pub struct OsBinding {
    pub named_pipe_server_pid_matches: bool,
    pub process_creation_filetime_matches: bool,
    pub both_gdb_tcp_ports_owned_by_pid: bool,
    pub process_stable_after_read: bool,
}
fn hex(bytes: &[u8]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(H[(*b >> 4) as usize] as char);
        s.push(H[(*b & 15) as usize] as char);
    }
    s
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    pub process_id: u32,
    pub process_created_filetime: String,
    pub rom_epoch: String,
    pub game_code: String,
    pub revision: u8,
    pub header6c: u8,
    pub rom_size: u64,
    pub crc32: String,
    pub core_sha256: String,
    pub known_full_rom_sha256: Option<String>,
    pub full_rom_reference_available: bool,
    pub full_rom_matches_catalog: bool,
    pub core_rom_stream_complete: bool,
    pub pipe_and_process_bound: bool,
    pub both_gdb_ports_bound: bool,
    pub diagnostic_identity_verified: bool,
    pub arm9_port: u16,
    pub arm7_port: u16,
    pub jit_enabled_when_loaded: bool,
    pub guest_write_allowed: bool,
    pub blockers: Vec<String>,
}
/// Returns only proof of ROM identity. No CPU sync, ICache or Overlay11 proof.
pub fn evaluate(
    p: &Packet,
    expected_pid: u32,
    code: &str,
    revision: u8,
    arm9: u16,
    arm7: u16,
    os: OsBinding,
) -> Identity {
    let profile = profile_for(code, revision);
    let full_reference = profile
        .as_ref()
        .and_then(|x| x.rom_sha256.as_ref())
        .cloned();
    let digest = hex(&p.sha256);
    let header_ok = profile.as_ref().is_some_and(|x| {
        x.header6c == p.header6c
            && p.game_code == code
            && p.revision == revision
            && !x.write_allowed
            && !x.guest_exec_runtime_verified
    });
    let sha_ok = full_reference
        .as_ref()
        .is_some_and(|s| s.eq_ignore_ascii_case(&digest));
    let complete = p.loaded
        && p.hash_complete
        && p.rom_epoch > 0
        && (1024 * 1024..=512 * 1024 * 1024).contains(&p.size);
    let pid_ok = p.pid != 0
        && p.pid == expected_pid
        && p.process_created_filetime != 0
        && os.named_pipe_server_pid_matches
        && os.process_creation_filetime_matches
        && os.process_stable_after_read;
    let ports_ok = arm9 >= 1024
        && arm7 >= 1024
        && arm9 != arm7
        && p.arm9_port == arm9
        && p.arm7_port == arm7
        && os.both_gdb_tcp_ports_owned_by_pid;
    let verified = complete && pid_ok && ports_ok && header_ok && sha_ok;
    let mut blockers = Vec::new();
    if !complete {
        blockers.push("core_loaded_rom_stream_incomplete".into());
    }
    if !pid_ok {
        blockers.push("pipe_pid_or_process_creation_not_bound".into());
    }
    if !ports_ok {
        blockers.push("arm9_arm7_tcp_owner_pid_not_bound".into());
    }
    if !header_ok {
        blockers.push("game_code_revision_header6c_mismatch".into());
    }
    if full_reference.is_none() {
        blockers.push("no_original_full_rom_sha256_reference".into());
    } else if !sha_ok {
        blockers.push("running_rom_sha256_not_catalog_match".into());
    }
    blockers.extend([
        "arm9_arm7_atomic_stop_unverified".into(),
        "jit_icache_coherence_unverified".into(),
        "overlay11_loading_epoch_unverified".into(),
        "guest_thumb_5a_a5_execution_unverified".into(),
        "overlay11_original_restore_faults_unverified".into(),
        "nds_guest_writes_disabled".into(),
    ]);
    Identity {
        process_id: p.pid,
        process_created_filetime: p.process_created_filetime.to_string(),
        rom_epoch: p.rom_epoch.to_string(),
        game_code: p.game_code.clone(),
        revision: p.revision,
        header6c: p.header6c,
        rom_size: p.size,
        crc32: format!("{:08X}", p.crc32),
        core_sha256: digest,
        known_full_rom_sha256: full_reference,
        full_rom_reference_available: profile.as_ref().is_some_and(|x| x.rom_sha256.is_some()),
        full_rom_matches_catalog: header_ok && sha_ok,
        core_rom_stream_complete: complete,
        pipe_and_process_bound: pid_ok,
        both_gdb_ports_bound: ports_ok,
        diagnostic_identity_verified: verified,
        arm9_port: p.arm9_port,
        arm7_port: p.arm7_port,
        jit_enabled_when_loaded: p.jit_at_load,
        guest_write_allowed: false,
        blockers,
    }
}
pub fn same_loaded_rom_session(a: &Packet, b: &Packet) -> bool {
    a.pid != 0
        && a.pid == b.pid
        && a.process_created_filetime != 0
        && a.process_created_filetime == b.process_created_filetime
        && a.rom_epoch != 0
        && a.rom_epoch == b.rom_epoch
        && a.game_code == b.game_code
        && a.revision == b.revision
        && a.header6c == b.header6c
        && a.sha256 == b.sha256
        && a.arm9_port == b.arm9_port
        && a.arm7_port == b.arm7_port
        && a.loaded
        && b.loaded
        && a.hash_complete
        && b.hash_complete
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpuObservation {
    pub cpu: String,
    pub port: u16,
    pub stop_reply_observed: bool,
    pub first_pc: Option<u32>,
    pub second_pc: Option<u32>,
    pub error: Option<String>,
}
impl CpuObservation {
    pub fn stable(&self) -> bool {
        self.stop_reply_observed
            && self.error.is_none()
            && self.first_pc.is_some()
            && self.first_pc == self.second_pc
    }
    pub fn absent(cpu: &str, port: u16, message: String) -> Self {
        Self {
            cpu: cpu.into(),
            port,
            stop_reply_observed: false,
            first_pc: None,
            second_pc: None,
            error: Some(message),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeAudit {
    pub identity: Identity,
    pub rom_epoch_stable_across_gdb_samples: bool,
    pub arm9: CpuObservation,
    pub arm7: CpuObservation,
    pub independent_stops_observed: bool,
    pub cpu_pair_atomically_stopped: bool,
    pub arm9_pc_inside_known_thumb_cave: Option<bool>,
    pub overlay11_static_source_present: bool,
    pub overlay11_guest_observation: Option<DpReadOnlySnapshot>,
    pub jit_icache_coherence_verified: bool,
    pub guest_thumb_execution_verified: bool,
    pub original_instruction_restoration_verified: bool,
    pub write_authorized: bool,
    pub blockers: Vec<String>,
}
pub fn runtime_audit(
    mut identity: Identity,
    epoch_stable: bool,
    arm9: CpuObservation,
    arm7: CpuObservation,
    guest: Option<DpReadOnlySnapshot>,
) -> RuntimeAudit {
    let independent_stops =
        arm9.stable() && arm7.stable() && arm9.cpu == "ARM9" && arm7.cpu == "ARM7";
    let cave = profile_for(&identity.game_code, identity.revision).and_then(|p| {
        if !p.battle_static_audit {
            return None;
        }
        let a = p.cave_addr;
        arm9.second_pc.map(|pc| (a..a + 72).contains(&(pc & !1)))
    });
    let overlay_static = guest
        .as_ref()
        .is_some_and(|g| g.battle.as_ref().is_some_and(|b| b.version_static_evidence));
    let mut blockers = identity.blockers.clone();
    if !epoch_stable {
        // A previously valid single snapshot is stale after a ROM/OS epoch
        // mismatch. Do not leave a green identity field in the audit.
        identity.diagnostic_identity_verified = false;
        blockers.push("rom_changed_during_gdb_observations".into());
    }
    if !arm9.stable() {
        blockers.push("arm9_stop_reply_or_registers_unverified".into());
    }
    if !arm7.stable() {
        blockers.push("arm7_stop_reply_or_registers_unverified".into());
    }
    if !overlay_static {
        blockers.push("overlay11_live_read_only_observation_missing".into());
    }
    if cave == Some(true) {
        blockers.push("arm9_pc_currently_inside_thumb_cave".into());
    }
    RuntimeAudit {
        identity,
        rom_epoch_stable_across_gdb_samples: epoch_stable,
        arm9,
        arm7,
        independent_stops_observed: independent_stops,
        cpu_pair_atomically_stopped: false,
        arm9_pc_inside_known_thumb_cave: cave,
        overlay11_static_source_present: overlay_static,
        overlay11_guest_observation: guest,
        jit_icache_coherence_verified: false,
        guest_thumb_execution_verified: false,
        original_instruction_restoration_verified: false,
        write_authorized: false,
        blockers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn packet(code: &str) -> Packet {
        let p = profile_for(code, if code.ends_with('E') { 13 } else { 5 }).unwrap();
        let mut sha = [0u8; 32];
        if let Some(d) = p.rom_sha256.as_ref() {
            for (i, b) in sha.iter_mut().enumerate() {
                *b = u8::from_str_radix(&d[i * 2..i * 2 + 2], 16).unwrap();
            }
        }
        Packet {
            pid: 981,
            process_created_filetime: 1234567,
            rom_epoch: 4,
            game_code: p.game_code,
            revision: p.revision,
            header6c: p.header6c,
            size: 128 * 1024 * 1024,
            crc32: 0x10002244,
            sha256: sha,
            loaded: true,
            arm9_port: 3333,
            arm7_port: 3334,
            jit_at_load: false,
            hash_complete: true,
        }
    }
    fn bytes(p: &Packet) -> Vec<u8> {
        let mut s = Vec::new();
        s.extend(MAGIC);
        s.extend(VERSION.to_le_bytes());
        s.extend(p.pid.to_le_bytes());
        s.extend(p.process_created_filetime.to_le_bytes());
        s.extend(p.rom_epoch.to_le_bytes());
        s.extend(p.game_code.as_bytes());
        s.push(p.revision);
        s.push(p.header6c);
        s.extend(p.size.to_le_bytes());
        s.extend(p.crc32.to_le_bytes());
        s.extend(p.sha256);
        s.push(p.loaded as u8);
        s.extend(p.arm9_port.to_le_bytes());
        s.extend(p.arm7_port.to_le_bytes());
        s.push(p.jit_at_load as u8);
        s.push(p.hash_complete as u8);
        assert_eq!(s.len(), WIRE_SIZE);
        s
    }
    fn os() -> OsBinding {
        OsBinding {
            named_pipe_server_pid_matches: true,
            process_creation_filetime_matches: true,
            both_gdb_tcp_ports_owned_by_pid: true,
            process_stable_after_read: true,
        }
    }
    #[test]
    fn roundtrip_strict_wire() {
        let p = packet("APAE");
        assert_eq!(decode_packet(&bytes(&p)).unwrap(), p);
        assert!(decode_packet(&bytes(&p)[..88]).is_err());
        let mut raw = bytes(&p);
        raw[0] = 0;
        assert!(decode_packet(&raw).is_err());
        let mut raw = bytes(&p);
        raw[82] = 2;
        assert!(decode_packet(&raw).is_err());
    }
    #[test]
    fn only_two_original_pearls_have_complete_rom_reference() {
        for p in crate::profiles() {
            let pkt = packet(&p.game_code);
            let r = evaluate(&pkt, 981, &p.game_code, p.revision, 3333, 3334, os());
            assert_eq!(
                r.full_rom_reference_available,
                matches!(p.game_code.as_str(), "APAE" | "APAI")
            );
            assert_eq!(
                r.diagnostic_identity_verified,
                matches!(p.game_code.as_str(), "APAE" | "APAI")
            );
            assert!(!r.guest_write_allowed);
        }
    }
    #[test]
    fn pipe_pid_process_and_port_swap_all_fail_closed() {
        let p = packet("APAI");
        let mut x = os();
        x.named_pipe_server_pid_matches = false;
        assert!(!evaluate(&p, 981, "APAI", 5, 3333, 3334, x).diagnostic_identity_verified);
        let mut x = os();
        x.process_creation_filetime_matches = false;
        assert!(!evaluate(&p, 981, "APAI", 5, 3333, 3334, x).diagnostic_identity_verified);
        let mut x = os();
        x.both_gdb_tcp_ports_owned_by_pid = false;
        assert!(!evaluate(&p, 981, "APAI", 5, 3333, 3334, x).diagnostic_identity_verified);
        assert!(!evaluate(&p, 981, "APAI", 5, 3334, 3333, os()).diagnostic_identity_verified);
    }
    #[test]
    fn changed_rom_epoch_header_and_sha_refused() {
        let p = packet("APAE");
        let mut p2 = p.clone();
        p2.rom_epoch += 1;
        assert!(!same_loaded_rom_session(&p, &p2));
        p2 = p.clone();
        p2.header6c = 42;
        assert!(!evaluate(&p2, 981, "APAE", 13, 3333, 3334, os()).diagnostic_identity_verified);
        p2 = p.clone();
        p2.sha256 = [0u8; 32];
        assert!(!evaluate(&p2, 981, "APAE", 13, 3333, 3334, os()).diagnostic_identity_verified);
        p2 = p.clone();
        p2.hash_complete = false;
        assert!(!evaluate(&p2, 981, "APAE", 13, 3333, 3334, os()).diagnostic_identity_verified);
    }
    #[test]
    fn even_valid_rom_identity_and_two_independent_stops_cannot_authorize_patch() {
        let p = packet("APAE");
        let r = evaluate(&p, 981, "APAE", 13, 3333, 3334, os());
        let cpu = |name: &str, port| CpuObservation {
            cpu: name.into(),
            port,
            stop_reply_observed: true,
            first_pc: Some(0x02001000),
            second_pc: Some(0x02001000),
            error: None,
        };
        let a = runtime_audit(r, true, cpu("ARM9", 3333), cpu("ARM7", 3334), None);
        assert!(a.identity.diagnostic_identity_verified);
        assert!(a.independent_stops_observed);
        assert!(!a.write_authorized && !a.cpu_pair_atomically_stopped);
        assert!(!a.jit_icache_coherence_verified);
    }
    #[test]
    fn observed_pc_in_thumb_cave_is_explicit_blocker() {
        let p = packet("APAI");
        let r = evaluate(&p, 981, "APAI", 5, 3333, 3334, os());
        let cpu = |name: &str, port, pc| CpuObservation {
            cpu: name.into(),
            port,
            stop_reply_observed: true,
            first_pc: Some(pc),
            second_pc: Some(pc),
            error: None,
        };
        let a = runtime_audit(
            r,
            true,
            cpu("ARM9", 3333, 0x02106853),
            cpu("ARM7", 3334, 0x03801200),
            None,
        );
        assert_eq!(a.arm9_pc_inside_known_thumb_cave, Some(true));
        assert!(a
            .blockers
            .contains(&"arm9_pc_currently_inside_thumb_cave".into()));
    }
    #[test]
    fn gdb_samples_across_a_rom_epoch_change_revoke_identity() {
        let packet = packet("APAE");
        let identity = evaluate(&packet, 981, "APAE", 13, 3333, 3334, os());
        assert!(identity.diagnostic_identity_verified);
        let cpu = |cpu: &str, port| CpuObservation {
            cpu: cpu.into(),
            port,
            stop_reply_observed: true,
            first_pc: Some(0x0200_1000),
            second_pc: Some(0x0200_1000),
            error: None,
        };
        let r = runtime_audit(identity, false, cpu("ARM9", 3333),
            cpu("ARM7", 3334), None);
        assert!(!r.identity.diagnostic_identity_verified);
        assert!(!r.write_authorized);
        assert!(r.blockers.contains(&"rom_changed_during_gdb_observations".into()));
    }
}
