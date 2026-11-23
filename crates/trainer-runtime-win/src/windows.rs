//! Windows mGBA external-process adapter. Strictly read-only.
//! This module never requests PROCESS_VM_WRITE, VM_OPERATION or PROCESS_CREATE_THREAD.
//! Candidate discovery is intentionally conservative and rejects ambiguous layouts.

use serde::Serialize;
use std::{
    collections::HashSet,
    ffi::c_void,
    mem::{size_of, zeroed},
    ptr::{null, null_mut},
};
use thiserror::Error;
use trainer_hgss::{profile_for as hgss_profile_for, HgssProfile};
use trainer_mgba::{
    guest_offset, parse_header, EmeraldReadOnlySession, GuestMemoryReader, ProbeError, RngSnapshot,
    RuntimeFingerprint,
};
use trainer_rs::{
    parse_header as parse_rs_header, profile_for as rs_profile_for, RsError, RsGame,
    RsReadOnlySession, RsSeedSnapshot,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, FILETIME, HANDLE, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::{
            Debug::ReadProcessMemory,
            ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
        },
        Memory::{VirtualQueryEx, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_GUARD, PAGE_NOACCESS},
        Threading::{
            GetProcessTimes, OpenProcess, PROCESS_QUERY_INFORMATION,
            PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
        },
    },
};

const ROM_MARKER: &[u8] = b"POKEMON EMER";
const SCAN_LIMIT: usize = 768 * 1024 * 1024;
const CHUNK: usize = 1024 * 1024;
const MAX_ROM_CANDIDATES: usize = 16;
const MAX_MAPPING_CANDIDATES: usize = 32;

#[derive(Clone, Debug, Serialize)]
pub struct MgbaTarget {
    pub process_id: u32,
    pub executable_name: String,
    pub created_at: u64,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Windows API failed: {0}")]
    Windows(String),
    #[error("not an allowed mGBA process or PID no longer exists")]
    InvalidTarget,
    #[error("mGBA process could not be opened read-only")]
    AccessDenied,
    #[error("no verified GBA ROM / GBAMemory mapping found; start a supported game in mGBA")]
    MappingUnavailable,
    #[error("ambiguous mGBA mappings; refused to guess active core")]
    Ambiguous,
    #[error("mGBA scan budget exceeded; no safe mapping selected")]
    ScanBudget,
    #[error("GBA read-only profile probe: {0}")]
    Probe(#[from] ProbeError),
    #[error("HGSS / DeSmuME read-only probe: {0}")]
    Hgss(String),
    #[error("Ruby/Sapphire read-only profile probe: {0}")]
    RsProbe(#[from] RsError),
}
fn os_error(op: &str) -> RuntimeError {
    RuntimeError::Windows(format!("{op}: {}", std::io::Error::last_os_error()))
}
fn is_mgba_executable(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(lower.as_str(), "mgba.exe" | "mgba-qt.exe" | "mgba-sdl.exe")
}

struct OwnedHandle(HANDLE);
// Windows process handles can be moved between threads; access remains limited
// to PROCESS_QUERY_INFORMATION | PROCESS_VM_READ and all operations use immutable reads.
unsafe impl Send for OwnedHandle {}
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            // SAFETY: Windows owns this valid HANDLE.
            unsafe { CloseHandle(self.0) };
        }
    }
}
fn open_handle(pid: u32, access: u32) -> Result<OwnedHandle, RuntimeError> {
    // SAFETY: caller supplies PID; a failed OpenProcess returns null, never dereferenced.
    let handle = unsafe { OpenProcess(access, 0, pid) };
    if handle.is_null() {
        return Err(RuntimeError::AccessDenied);
    }
    Ok(OwnedHandle(handle))
}
fn creation_time(handle: HANDLE) -> Result<u64, RuntimeError> {
    // SAFETY: FILETIME buffers are initialized and only passed to Win32.
    let (mut create, mut exit, mut kernel, mut user): (FILETIME, FILETIME, FILETIME, FILETIME) =
        unsafe { (zeroed(), zeroed(), zeroed(), zeroed()) };
    // SAFETY: valid process handle and four output pointers.
    if unsafe { GetProcessTimes(handle, &mut create, &mut exit, &mut kernel, &mut user) } == 0 {
        return Err(os_error("GetProcessTimes"));
    }
    Ok((u64::from(create.dwHighDateTime) << 32) | u64::from(create.dwLowDateTime))
}

pub fn list_mgba_targets() -> Result<Vec<MgbaTarget>, RuntimeError> {
    // SAFETY: toolhelp snapshot owns a kernel handle; it is closed by RAII.
    let snapshot = OwnedHandle(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) });
    if snapshot.0 == INVALID_HANDLE_VALUE {
        return Err(os_error("CreateToolhelp32Snapshot"));
    }
    let mut entry: PROCESSENTRY32W = unsafe { zeroed() };
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut targets = Vec::new();
    // SAFETY: the entry length is set to the documented struct size.
    let mut ok = unsafe { Process32FirstW(snapshot.0, &mut entry) };
    while ok != 0 {
        let chars = entry
            .szExeFile
            .iter()
            .position(|&n| n == 0)
            .unwrap_or(entry.szExeFile.len());
        let name = String::from_utf16_lossy(&entry.szExeFile[..chars]);
        if is_mgba_executable(&name) {
            if let Ok(h) = open_handle(entry.th32ProcessID, PROCESS_QUERY_LIMITED_INFORMATION) {
                if let Ok(created_at) = creation_time(h.0) {
                    targets.push(MgbaTarget {
                        process_id: entry.th32ProcessID,
                        executable_name: name,
                        created_at,
                    });
                }
            }
        }
        ok = unsafe { Process32NextW(snapshot.0, &mut entry) };
    }
    targets.sort_by_key(|p| p.process_id);
    Ok(targets)
}

struct Process {
    handle: OwnedHandle,
    pid: u32,
    created_at: u64,
}
impl Process {
    fn open_known_mgba(pid: u32) -> Result<Self, RuntimeError> {
        let candidate = list_mgba_targets()?
            .into_iter()
            .find(|p| p.process_id == pid)
            .ok_or(RuntimeError::InvalidTarget)?;
        let handle = open_handle(pid, PROCESS_QUERY_INFORMATION | PROCESS_VM_READ)?;
        let created_at = creation_time(handle.0)?;
        if created_at != candidate.created_at {
            return Err(RuntimeError::InvalidTarget);
        }
        Ok(Self {
            handle,
            pid,
            created_at,
        })
    }
    fn read_host(&self, address: u64, data: &mut [u8]) -> Result<(), RuntimeError> {
        let addr = usize::try_from(address).map_err(|_| RuntimeError::MappingUnavailable)?;
        let mut got = 0usize;
        // SAFETY: read-only process handle, remote address never dereferenced locally,
        // destination points to owned initialized slice, no write access is requested.
        let success = unsafe {
            ReadProcessMemory(
                self.handle.0,
                addr as *const c_void,
                data.as_mut_ptr().cast::<c_void>(),
                data.len(),
                &mut got,
            )
        };
        if success == 0 || got != data.len() {
            return Err(os_error("ReadProcessMemory"));
        }
        Ok(())
    }
    fn read_u32(&self, address: u64) -> Result<u32, RuntimeError> {
        let mut bytes = [0; 4];
        self.read_host(address, &mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }
    fn read_pointer(&self, address: u64, size: usize) -> Result<u64, RuntimeError> {
        let mut bytes = [0; 8];
        self.read_host(address, &mut bytes[..size])?;
        Ok(u64::from_le_bytes(bytes))
    }
    fn memory_regions(&self) -> Vec<(u64, usize)> {
        let mut result = Vec::new();
        let mut at = 0x10000u64;
        loop {
            let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { zeroed() };
            // SAFETY: read-only metadata query, out pointer size explicitly provided.
            let written = unsafe {
                VirtualQueryEx(
                    self.handle.0,
                    at as *const c_void,
                    &mut mbi,
                    size_of::<MEMORY_BASIC_INFORMATION>(),
                )
            };
            if written == 0 {
                break;
            }
            let base = mbi.BaseAddress as usize as u64;
            let end = base.saturating_add(mbi.RegionSize as u64);
            if mbi.State == MEM_COMMIT
                && mbi.Protect != 0
                && (mbi.Protect & PAGE_GUARD) == 0
                && (mbi.Protect & PAGE_NOACCESS) == 0
                && (mbi.Protect & 0xff) != 0x10
            {
                result.push((base, mbi.RegionSize));
            }
            if end <= at {
                break;
            }
            at = end;
        }
        result
    }
    fn scan_pattern(
        &self,
        regions: &[(u64, usize)],
        needle: &[u8],
        limit: usize,
        cap: usize,
    ) -> Result<Vec<u64>, RuntimeError> {
        let mut found = Vec::new();
        let mut scanned = 0usize;
        for &(base, size) in regions {
            let mut offset = 0usize;
            while offset < size && scanned < limit {
                let count = (size - offset).min(CHUNK).min(limit - scanned);
                if count == 0 {
                    break;
                }
                let mut data = vec![0u8; count];
                if self.read_host(base + offset as u64, &mut data).is_ok() {
                    for (idx, part) in data.windows(needle.len()).enumerate() {
                        if part == needle {
                            found.push(base + offset as u64 + idx as u64);
                            if found.len() > cap {
                                return Err(RuntimeError::Ambiguous);
                            }
                        }
                    }
                }
                // overlap across chunk boundaries, even if the previous read failed.
                let step = count.saturating_sub(needle.len().saturating_sub(1)).max(1);
                offset = offset.saturating_add(step);
                scanned = scanned.saturating_add(step);
            }
            if scanned >= limit {
                break;
            }
        }
        Ok(found)
    }
}

struct GuestHostReader {
    process: Process,
    rom: u64,
    ewram: u64,
    iwram: u64,
    pointer_field: u64,
    pointer_size: usize,
    code: String,
    revision: u8,
}
impl GuestHostReader {
    fn read_mapping(&self) -> Result<(u64, u64, u64), ProbeError> {
        let size = self.pointer_size as u64;
        let rom = self
            .process
            .read_pointer(self.pointer_field, self.pointer_size)
            .map_err(|_| ProbeError::Stale)?;
        let iwram = self
            .process
            .read_pointer(self.pointer_field - size, self.pointer_size)
            .map_err(|_| ProbeError::Stale)?;
        let ewram = self
            .process
            .read_pointer(self.pointer_field - size * 2, self.pointer_size)
            .map_err(|_| ProbeError::Stale)?;
        Ok((rom, ewram, iwram))
    }
}
impl GuestMemoryReader for GuestHostReader {
    fn read_guest(&self, guest_addr: u32, out: &mut [u8]) -> Result<(), ProbeError> {
        let (region, offset) =
            guest_offset(guest_addr, out.len()).ok_or_else(|| ProbeError::Read {
                address: guest_addr,
                detail: "guest address outside ROM/EWRAM/IWRAM".into(),
            })?;
        let base = match region {
            "rom" => self.rom,
            "ewram" => self.ewram,
            "iwram" => self.iwram,
            _ => return Err(ProbeError::Stale),
        };
        let address = base.checked_add(offset as u64).ok_or(ProbeError::Stale)?;
        self.process
            .read_host(address, out)
            .map_err(|e| ProbeError::Read {
                address: guest_addr,
                detail: e.to_string(),
            })
    }
    fn fingerprint(&self) -> Result<RuntimeFingerprint, ProbeError> {
        if creation_time(self.process.handle.0).map_err(|_| ProbeError::Stale)?
            != self.process.created_at
        {
            return Err(ProbeError::Stale);
        }
        if self.read_mapping()? != (self.rom, self.ewram, self.iwram) {
            return Err(ProbeError::Stale);
        }
        let mut header = [0u8; 0xC0];
        self.read_guest(0x0800_0000, &mut header)?;
        // Identity is polymorphic across Gen3 cartridges. A changed game or
        // revision is rejected even if an emulator reused the same ROM pointer.
        let (code, rev) = if let Ok((emerald, rev)) = parse_header(&header) {
            (emerald.code.to_owned(), rev)
        } else if let Ok(rs) = parse_rs_header(&header) {
            (rs.game_code, rs.revision)
        } else {
            return Err(ProbeError::Stale);
        };
        if code != self.code || rev != self.revision {
            return Err(ProbeError::Stale);
        }
        Ok(RuntimeFingerprint {
            process_id: self.process.pid,
            process_created_at: self.process.created_at,
            host_rom_base: self.rom,
            host_ewram_base: self.ewram,
            host_iwram_base: self.iwram,
            host_rom_pointer_field: self.pointer_field,
            game_code: self.code.clone(),
            revision: self.revision,
        })
    }
}
/// Scans a selected mGBA process for a known Emerald ROM and GBAMemory pointer tuple.
/// Any ambiguity or invalid profile is a hard failure. There is no write mode.
pub struct AttachedEmerald {
    reader: GuestHostReader,
    session: EmeraldReadOnlySession,
}
impl AttachedEmerald {
    pub fn attach(pid: u32) -> Result<Self, RuntimeError> {
        let process = Process::open_known_mgba(pid)?;
        let regions = process.memory_regions();
        let raw_roms =
            process.scan_pattern(&regions, ROM_MARKER, SCAN_LIMIT, MAX_ROM_CANDIDATES)?;
        let mut roms = Vec::new();
        for marker in raw_roms {
            let Some(base) = marker.checked_sub(0xA0) else {
                continue;
            };
            let mut header = [0; 0xC0];
            if process.read_host(base, &mut header).is_ok() && parse_header(&header).is_ok() {
                roms.push(base);
            }
        }
        roms.sort_unstable();
        roms.dedup();
        if roms.is_empty() {
            return Err(RuntimeError::MappingUnavailable);
        }
        let mut mappings = Vec::new();
        let mut seen = HashSet::new();
        for &rom in &roms {
            for width in [8usize, 4usize] {
                if width == 4 && rom > u32::MAX as u64 {
                    continue;
                }
                let pattern = rom.to_le_bytes();
                let hits = process.scan_pattern(
                    &regions,
                    &pattern[..width],
                    SCAN_LIMIT,
                    MAX_MAPPING_CANDIDATES,
                )?;
                for slot in hits {
                    if slot % width as u64 != 0 || slot < width as u64 * 2 {
                        continue;
                    }
                    let ewram = process.read_pointer(slot - (width * 2) as u64, width).ok();
                    let iwram = process.read_pointer(slot - width as u64, width).ok();
                    let (Some(ewram), Some(iwram)) = (ewram, iwram) else {
                        continue;
                    };
                    if ewram == 0 || iwram == 0 || rom == 0 {
                        continue;
                    }
                    let mut header = [0u8; 0xC0];
                    if process.read_host(rom, &mut header).is_err() {
                        continue;
                    }
                    let Ok((profile, _)) = parse_header(&header) else {
                        continue;
                    };
                    // Strong candidate criterion mirrors the source's task-table plausibility;
                    // 16 task callback pointers live 0x28 bytes apart in IWRAM.
                    let task_guest = if profile.code == "BPEJ" {
                        0x5B60u64
                    } else {
                        0x5E00u64
                    };
                    let mut count = 0;
                    for n in 0..16u64 {
                        if let Ok(callback) = process.read_u32(iwram + task_guest + n * 0x28) {
                            if callback == 0
                                || (0x0200_0000..0x0400_0000).contains(&callback)
                                || (0x0800_0000..0x0E00_0000).contains(&callback)
                            {
                                count += 1;
                            }
                        }
                    }
                    if count < 12 {
                        continue;
                    }
                    if seen.insert((rom, ewram, iwram, slot, width)) {
                        mappings.push((rom, ewram, iwram, slot, width, profile.code.to_string()));
                    }
                }
            }
        }
        if mappings.is_empty() {
            return Err(RuntimeError::MappingUnavailable);
        }
        if mappings.len() != 1 {
            return Err(RuntimeError::Ambiguous);
        }
        let (rom, ewram, iwram, pointer_field, pointer_size, code) =
            mappings.pop().expect("one mapping");
        let reader = GuestHostReader {
            process,
            rom,
            ewram,
            iwram,
            pointer_field,
            pointer_size,
            code,
            revision: 0,
        };
        let session = EmeraldReadOnlySession::attach(&reader)?;
        Ok(Self { reader, session })
    }
    pub fn session_id(&self) -> &str {
        self.session.session_id()
    }
    pub fn game_code(&self) -> &str {
        self.session.profile().code
    }
    pub fn language(&self) -> &str {
        self.session.profile().language
    }
    pub fn snapshot(&mut self) -> Result<RngSnapshot, ProbeError> {
        self.session.read_snapshot(&self.reader)
    }
}

/// Ruby/Sapphire read-only mGBA connection. The conservative layout check
/// rejects unknown ROMs/identity and ambiguous host pointer tuples.
/// Note: Header + retail boot-table anchor is only partial ROM verification.
pub struct AttachedRubySapphire {
    reader: GuestHostReader,
    session: RsReadOnlySession,
}
impl AttachedRubySapphire {
    pub fn attach(pid: u32, requested: RsGame) -> Result<Self, RuntimeError> {
        let process = Process::open_known_mgba(pid)?;
        let regions = process.memory_regions();
        // Game-specific title prevents Ruby/Sapphire/Emerald cross-detection.
        let marker = match requested {
            RsGame::Ruby => b"POKEMON RUBY".as_slice(),
            RsGame::Sapphire => b"POKEMON SAPP".as_slice(),
        };
        let raw_roms = process.scan_pattern(&regions, marker, SCAN_LIMIT, MAX_ROM_CANDIDATES)?;
        let mut roms = Vec::new();
        for title_at in raw_roms {
            let Some(base) = title_at.checked_sub(0xA0) else {
                continue;
            };
            let mut header = [0u8; 0xC0];
            if process.read_host(base, &mut header).is_ok() {
                if let Ok(profile) = parse_rs_header(&header) {
                    if profile.game == requested {
                        roms.push((base, profile));
                    }
                }
            }
        }
        roms.sort_by_key(|(base, _)| *base);
        roms.dedup_by_key(|(base, _)| *base);
        if roms.is_empty() {
            return Err(RuntimeError::MappingUnavailable);
        }
        let mut candidates = Vec::new();
        let mut seen = HashSet::new();
        for &(rom, ref profile) in &roms {
            // Verify the retail boot-table anchor BEFORE any pointer resolution.
            // This rejects the known AXVD Rev0 German debug build by default.
            let mut actual_anchor = [0u8; 16];
            if process.read_host(rom + 0x128, &mut actual_anchor).is_err()
                || rs_profile_for(&profile.game_code, profile.revision).is_err()
            {
                continue;
            }
            // Host candidate anchor check is identical to the core's source
            // registry. The session below repeats it via guest.read.
            if !trainer_rs::matches_anchor(&profile.game_code, &actual_anchor) {
                continue;
            }
            for pointer_size in [8usize, 4usize] {
                if pointer_size == 4 && rom > u32::MAX as u64 {
                    continue;
                }
                let rom_pointer = rom.to_le_bytes();
                let slots = process.scan_pattern(
                    &regions,
                    &rom_pointer[..pointer_size],
                    SCAN_LIMIT,
                    MAX_MAPPING_CANDIDATES,
                )?;
                for field in slots {
                    let width = pointer_size as u64;
                    if field % width != 0 || field < width * 2 {
                        continue;
                    }
                    let ewram = process.read_pointer(field - width * 2, pointer_size).ok();
                    let iwram = process.read_pointer(field - width, pointer_size).ok();
                    let (Some(ewram), Some(iwram)) = (ewram, iwram) else {
                        continue;
                    };
                    if ewram == 0 || iwram == 0 {
                        continue;
                    }
                    // Source RS layout evidence: observe Seed+Timer+SaveBlock1/2
                    // using game/locale-specific addresses, never Emerald offsets.
                    let seed_off = (profile.seed_address - 0x0300_0000) as u64;
                    let timer_off = (profile.timer_address - 0x0300_0000) as u64;
                    let save1_off = if profile.game_code.ends_with('J') {
                        0x25494u64
                    } else {
                        0x25734u64
                    };
                    let save2_off = if profile.game_code.ends_with('J') {
                        0x24C04u64
                    } else {
                        0x24EA4u64
                    };
                    let mut timer = [0u8; 2];
                    if process.read_u32(iwram + seed_off).is_err()
                        || process.read_host(iwram + timer_off, &mut timer).is_err()
                        || process.read_u32(ewram + save1_off).is_err()
                        || process.read_u32(ewram + save2_off).is_err()
                    {
                        continue;
                    }
                    if seen.insert((rom, ewram, iwram, field, pointer_size)) {
                        candidates.push((
                            rom,
                            ewram,
                            iwram,
                            field,
                            pointer_size,
                            profile.game_code.clone(),
                            profile.revision,
                        ));
                    }
                }
            }
        }
        if candidates.is_empty() {
            return Err(RuntimeError::MappingUnavailable);
        }
        if candidates.len() != 1 {
            return Err(RuntimeError::Ambiguous);
        }
        let (rom, ewram, iwram, pointer_field, pointer_size, code, revision) =
            candidates.pop().expect("exactly one unique mapping");
        let reader = GuestHostReader {
            process,
            rom,
            ewram,
            iwram,
            pointer_field,
            pointer_size,
            code,
            revision,
        };
        let session = RsReadOnlySession::attach(&reader)?;
        Ok(Self { reader, session })
    }
    pub fn session_id(&self) -> &str {
        self.session.session_id()
    }
    pub fn game_id(&self) -> &str {
        self.session.profile().game.id()
    }
    pub fn game_code(&self) -> &str {
        &self.session.profile().game_code
    }
    pub fn revision(&self) -> u8 {
        self.session.profile().revision
    }
    pub fn language(&self) -> &str {
        &self.session.profile().language
    }
    pub fn snapshot(&mut self) -> Result<RsSeedSnapshot, RsError> {
        self.session.snapshot(&self.reader)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_executable_names_only() {
        assert!(is_mgba_executable("mGBA.exe"));
        assert!(is_mgba_executable("mgba-qt.exe"));
        assert!(!is_mgba_executable("mgba-backup.exe"));
        assert!(!is_mgba_executable("not-mgba.exe"));
    }
}

// M5: DeSmuME NDS read-only provider. Separate from mGBA/GDB and never requests
// PROCESS_VM_WRITE, PROCESS_VM_OPERATION, or PROCESS_CREATE_THREAD.
fn is_desmume_executable(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.starts_with("desmume") && name.ends_with(".exe")
}

pub fn list_desmume_targets() -> Result<Vec<MgbaTarget>, RuntimeError> {
    let snapshot = OwnedHandle(unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) });
    if snapshot.0 == INVALID_HANDLE_VALUE {
        return Err(os_error("CreateToolhelp32Snapshot"));
    }
    let mut entry: PROCESSENTRY32W = unsafe { zeroed() };
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut targets = Vec::new();
    let mut ok = unsafe { Process32FirstW(snapshot.0, &mut entry) };
    while ok != 0 {
        let end = entry
            .szExeFile
            .iter()
            .position(|&ch| ch == 0)
            .unwrap_or(entry.szExeFile.len());
        let name = String::from_utf16_lossy(&entry.szExeFile[..end]);
        if is_desmume_executable(&name) {
            if let Ok(handle) = open_handle(entry.th32ProcessID, PROCESS_QUERY_LIMITED_INFORMATION)
            {
                if let Ok(created_at) = creation_time(handle.0) {
                    targets.push(MgbaTarget {
                        process_id: entry.th32ProcessID,
                        executable_name: name,
                        created_at,
                    });
                }
            }
        }
        ok = unsafe { Process32NextW(snapshot.0, &mut entry) };
    }
    targets.sort_by_key(|t| t.process_id);
    Ok(targets)
}

#[derive(Clone, Debug, Serialize)]
pub struct HgssReadOnlySnapshot {
    pub session_id: String,
    pub game_id: String,
    pub game_code: String,
    pub language: String,
    pub revision: u8,
    pub current_seed: u32,
    pub mt_state0: u32,
    pub mt_index: u32,
    pub delay: u32,
    pub trainer_id: Option<u16>,
    pub secret_id: Option<u16>,
    pub verification: &'static str,
    pub read_only: bool,
    pub advances_verified: bool,
}

pub struct AttachedHgss {
    process: Process,
    profile: HgssProfile,
    nds_base: u64,
    header_hit: u64,
    session_id: String,
}

impl AttachedHgss {
    fn host_addr(base: u64, guest: u32) -> Result<u64, RuntimeError> {
        if !(0x0200_0000..0x0240_0000).contains(&guest) {
            return Err(RuntimeError::Hgss(
                "guest address outside NDS main RAM".into(),
            ));
        }
        Ok(base + u64::from(guest - 0x0200_0000))
    }

    fn read_guest(process: &Process, base: u64, guest: u32) -> Result<u32, RuntimeError> {
        process.read_u32(Self::host_addr(base, guest)?)
    }

    /// Probe the unique 4 MiB DeSmuME main-RAM base using source-backed
    /// Seed/MT/pointer structure. Scoring never makes an ambiguous candidate writable.
    pub fn attach(pid: u32, requested_game_id: &str) -> Result<Self, RuntimeError> {
        if !matches!(
            requested_game_id,
            "games.pokemon.heartgold" | "games.pokemon.soulsilver"
        ) {
            return Err(RuntimeError::Hgss("unsupported requested game ID".into()));
        }
        let target = list_desmume_targets()?
            .into_iter()
            .find(|target| target.process_id == pid)
            .ok_or(RuntimeError::InvalidTarget)?;
        let handle = open_handle(pid, PROCESS_QUERY_INFORMATION | PROCESS_VM_READ)?;
        let created_at = creation_time(handle.0)?;
        if created_at != target.created_at {
            return Err(RuntimeError::Hgss("DeSmuME PID was replaced".into()));
        }
        let process = Process {
            handle,
            pid,
            created_at,
        };
        let regions = process.memory_regions();
        let mut hits = Vec::new();
        let mut scanned = 0usize;
        // A single bounded pass; scanning every supported game-code separately
        // would scan remote RAM 14 times and freeze the UI.
        for &(region, size) in &regions {
            let mut off = 0usize;
            while off < size && scanned < SCAN_LIMIT {
                let count = (size - off).min(CHUNK).min(SCAN_LIMIT - scanned);
                if count == 0 {
                    break;
                }
                let mut bytes = vec![0u8; count];
                if process.read_host(region + off as u64, &mut bytes).is_ok() {
                    for (idx, w) in bytes.windows(4).enumerate() {
                        if (&w[..3] == b"IPK" || &w[..3] == b"IPG")
                            && matches!(w[3], b'J' | b'E' | b'D' | b'F' | b'I' | b'S' | b'K')
                        {
                            // A matched game code is still untrusted until the
                            // Header Rev and 4MiB main-RAM probes pass.
                            hits.push(region + off as u64 + idx as u64);
                            if hits.len() > 256 {
                                return Err(RuntimeError::Hgss(
                                    "too many ambiguous ROM markers".into(),
                                ));
                            }
                        }
                    }
                }
                let step = count.saturating_sub(3).max(1);
                off = off.saturating_add(step);
                scanned = scanned.saturating_add(step);
            }
            if scanned >= SCAN_LIMIT {
                break;
            }
        }
        const MIRRORS: [u64; 4] = [0x003FFE0C, 0x007FFE0C, 0x00BFFE0C, 0x00FFFE0C];
        let mut candidates = Vec::new();
        let mut seen = HashSet::new();
        for hit in hits {
            let mut code = [0u8; 4];
            let mut rev = [0u8; 1];
            if process.read_host(hit, &mut code).is_err()
                || process.read_host(hit + 0x12, &mut rev).is_err()
            {
                continue;
            }
            let Ok(code) = std::str::from_utf8(&code) else {
                continue;
            };
            let Some(profile) = hgss_profile_for(code, rev[0]) else {
                continue;
            };
            if profile.game_id != requested_game_id {
                continue;
            }
            for mirror in MIRRORS {
                let Some(base) = hit.checked_sub(mirror) else {
                    continue;
                };
                if !seen.insert(base) {
                    continue;
                }
                let probe = (
                    Self::read_guest(&process, base, profile.current_seed_addr),
                    Self::read_guest(&process, base, profile.mt_state0_addr),
                    Self::read_guest(&process, base, profile.mt_index_addr),
                    Self::read_guest(&process, base, profile.pid_pointer_addr),
                    Self::read_guest(&process, base, profile.trainer_ids_pointer_addr),
                    Self::read_guest(&process, base, profile.delay_addr),
                );
                let (Ok(seed), Ok(mt), Ok(index), Ok(pidptr), Ok(trainerptr), Ok(delay)) = probe
                else {
                    continue;
                };
                if index > 624
                    || !(0x0200_0000..0x0240_0000).contains(&pidptr)
                    || !(0x0200_0000..0x0240_0000).contains(&trainerptr)
                {
                    continue;
                }
                let score: u32 = 100
                    + if seed != 0 { 30 } else { 0 }
                    + if mt != 0 { 20 } else { 0 }
                    + if index == 624 { 15 } else { 8 }
                    + if delay < 0x0010_0000 { 8 } else { 0 }
                    + if base % 16 == 0 { 2 } else { 0 };
                candidates.push((score, base, hit, profile.clone()));
            }
        }
        candidates.sort_by(|a, b| b.0.cmp(&a.0));
        let Some((score, base, hit, profile)) = candidates.first().cloned() else {
            return Err(RuntimeError::Hgss(
                "no verified DeSmuME MAIN_MEM mapping; unsupported ROM/revision or no game loaded"
                    .into(),
            ));
        };
        if candidates.get(1).is_some_and(|next| next.0 == score) {
            return Err(RuntimeError::Hgss(
                "multiple equally scored NDS RAM mappings".into(),
            ));
        }
        let session_id = format!("hgss-readonly-{}-{}-{:X}", pid, created_at, base);
        Ok(Self {
            process,
            profile,
            nds_base: base,
            header_hit: hit,
            session_id,
        })
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    pub fn game_id(&self) -> &str {
        &self.profile.game_id
    }
    pub fn snapshot(&self) -> Result<HgssReadOnlySnapshot, RuntimeError> {
        if creation_time(self.process.handle.0)? != self.process.created_at {
            return Err(RuntimeError::Hgss(
                "emulator process identity changed".into(),
            ));
        }
        let mut header = [0u8; 0x13];
        self.process.read_host(self.header_hit, &mut header)?;
        if &header[..4] != self.profile.game_code.as_bytes()
            || header[0x12] != self.profile.revision
        {
            return Err(RuntimeError::Hgss(
                "ROM code/revision changed; reconnect required".into(),
            ));
        }
        let p = &self.profile;
        let current_seed = Self::read_guest(&self.process, self.nds_base, p.current_seed_addr)?;
        let mt_state0 = Self::read_guest(&self.process, self.nds_base, p.mt_state0_addr)?;
        let mt_index = Self::read_guest(&self.process, self.nds_base, p.mt_index_addr)?;
        let delay = Self::read_guest(&self.process, self.nds_base, p.delay_addr)?;
        let trainer_ptr =
            Self::read_guest(&self.process, self.nds_base, p.trainer_ids_pointer_addr)?;
        let pid_ptr = Self::read_guest(&self.process, self.nds_base, p.pid_pointer_addr)?;
        if mt_index > 624
            || !(0x0200_0000..0x0240_0000).contains(&trainer_ptr)
            || !(0x0200_0000..0x0240_0000).contains(&pid_ptr)
        {
            return Err(RuntimeError::Hgss(
                "MT index/pointers became invalid; session revoked".into(),
            ));
        }
        let ids_addr = trainer_ptr
            .checked_add(0x84)
            .ok_or_else(|| RuntimeError::Hgss("trainer pointer overflow".into()))?;
        let ids = Self::read_guest(&self.process, self.nds_base, ids_addr).ok();
        Ok(HgssReadOnlySnapshot {
            session_id: self.session_id.clone(),
            game_id: p.game_id.clone(),
            game_code: p.game_code.clone(),
            language: p.language.clone(),
            revision: p.revision,
            current_seed,
            mt_state0,
            mt_index,
            delay,
            trainer_id: ids.map(|v| v as u16),
            secret_id: ids.map(|v| (v >> 16) as u16),
            verification: "code_revision_ram_pointer_probe_only",
            read_only: true,
            advances_verified: false,
        })
    }
}

