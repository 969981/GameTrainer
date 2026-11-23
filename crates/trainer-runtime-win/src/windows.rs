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
use trainer_mgba::{
    guest_offset, parse_header, EmeraldReadOnlySession, GuestMemoryReader, ProbeError,
    RngSnapshot, RuntimeFingerprint,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, FILETIME, HANDLE, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::{
            Debug::ReadProcessMemory,
            ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS},
        },
        Memory::{VirtualQueryEx, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_GUARD, PAGE_NOACCESS},
        Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ},
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
    #[error("no verified Emerald ROM / GBAMemory mapping found; start Emerald in mGBA")]
    MappingUnavailable,
    #[error("ambiguous mGBA mappings; refused to guess active core")]
    Ambiguous,
    #[error("mGBA scan budget exceeded; no safe mapping selected")]
    ScanBudget,
    #[error("Emerald profile probe: {0}")]
    Probe(#[from] ProbeError),
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
    if handle.is_null() { return Err(RuntimeError::AccessDenied); }
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
    if snapshot.0 == INVALID_HANDLE_VALUE { return Err(os_error("CreateToolhelp32Snapshot")); }
    let mut entry: PROCESSENTRY32W = unsafe { zeroed() };
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut targets = Vec::new();
    // SAFETY: the entry length is set to the documented struct size.
    let mut ok = unsafe { Process32FirstW(snapshot.0, &mut entry) };
    while ok != 0 {
        let chars = entry.szExeFile.iter().position(|&n| n == 0).unwrap_or(entry.szExeFile.len());
        let name = String::from_utf16_lossy(&entry.szExeFile[..chars]);
        if is_mgba_executable(&name) {
            if let Ok(h) = open_handle(entry.th32ProcessID, PROCESS_QUERY_LIMITED_INFORMATION) {
                if let Ok(created_at) = creation_time(h.0) {
                    targets.push(MgbaTarget {process_id:entry.th32ProcessID,executable_name:name,created_at});
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
        let candidate = list_mgba_targets()?.into_iter()
            .find(|p| p.process_id == pid).ok_or(RuntimeError::InvalidTarget)?;
        let handle = open_handle(pid, PROCESS_QUERY_INFORMATION | PROCESS_VM_READ)?;
        let created_at = creation_time(handle.0)?;
        if created_at != candidate.created_at { return Err(RuntimeError::InvalidTarget); }
        Ok(Self {handle,pid,created_at})
    }
    fn read_host(&self, address: u64, data: &mut [u8]) -> Result<(), RuntimeError> {
        let addr = usize::try_from(address).map_err(|_|RuntimeError::MappingUnavailable)?;
        let mut got = 0usize;
        // SAFETY: read-only process handle, remote address never dereferenced locally,
        // destination points to owned initialized slice, no write access is requested.
        let success = unsafe {
            ReadProcessMemory(self.handle.0, addr as *const c_void, data.as_mut_ptr().cast::<c_void>(), data.len(), &mut got)
        };
        if success == 0 || got != data.len() {return Err(os_error("ReadProcessMemory"));}
        Ok(())
    }
    fn read_u32(&self, address: u64) -> Result<u32, RuntimeError> {
        let mut bytes=[0;4]; self.read_host(address,&mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }
    fn read_pointer(&self, address: u64, size: usize) -> Result<u64, RuntimeError> {
        let mut bytes=[0;8];self.read_host(address,&mut bytes[..size])?;
        Ok(u64::from_le_bytes(bytes))
    }
    fn memory_regions(&self) -> Vec<(u64, usize)> {
        let mut result=Vec::new();
        let mut at=0x10000u64;
        loop {
            let mut mbi: MEMORY_BASIC_INFORMATION=unsafe {zeroed()};
            // SAFETY: read-only metadata query, out pointer size explicitly provided.
            let written=unsafe {VirtualQueryEx(self.handle.0,at as *const c_void,&mut mbi,size_of::<MEMORY_BASIC_INFORMATION>())};
            if written==0 {break;}
            let base=mbi.BaseAddress as usize as u64;
            let end=base.saturating_add(mbi.RegionSize as u64);
            if mbi.State==MEM_COMMIT
                && mbi.Protect!=0 && (mbi.Protect & PAGE_GUARD)==0
                && (mbi.Protect & PAGE_NOACCESS)==0
                && (mbi.Protect & 0xff)!=0x10
            {
                result.push((base,mbi.RegionSize));
            }
            if end<=at {break;}
            at=end;
        }
        result
    }
    fn scan_pattern(&self, regions:&[(u64,usize)], needle:&[u8], limit:usize, cap:usize) -> Result<Vec<u64>,RuntimeError>{
        let mut found=Vec::new();
        let mut scanned=0usize;
        for &(base,size) in regions {
            let mut offset=0usize;
            while offset<size && scanned<limit {
                let count=(size-offset).min(CHUNK).min(limit-scanned);
                if count==0 {break;}
                let mut data=vec![0u8;count];
                if self.read_host(base+offset as u64,&mut data).is_ok() {
                    for (idx,part) in data.windows(needle.len()).enumerate() {
                        if part==needle {
                            found.push(base+offset as u64+idx as u64);
                            if found.len()>cap {return Err(RuntimeError::Ambiguous);}
                        }
                    }
                }
                // overlap across chunk boundaries, even if the previous read failed.
                let step=count.saturating_sub(needle.len().saturating_sub(1)).max(1);
                offset=offset.saturating_add(step);
                scanned=scanned.saturating_add(step);
            }
            if scanned>=limit {break;}
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
    fn read_mapping(&self) -> Result<(u64,u64,u64),ProbeError> {
        let size=self.pointer_size as u64;
        let rom=self.process.read_pointer(self.pointer_field,self.pointer_size)
            .map_err(|_|ProbeError::Stale)?;
        let iwram=self.process.read_pointer(self.pointer_field-size,self.pointer_size)
            .map_err(|_|ProbeError::Stale)?;
        let ewram=self.process.read_pointer(self.pointer_field-size*2,self.pointer_size)
            .map_err(|_|ProbeError::Stale)?;
        Ok((rom,ewram,iwram))
    }
}
impl GuestMemoryReader for GuestHostReader {
    fn read_guest(&self, guest_addr:u32, out:&mut[u8])->Result<(),ProbeError>{
        let (region,offset)=guest_offset(guest_addr,out.len())
            .ok_or_else(||ProbeError::Read{address:guest_addr,detail:"guest address outside ROM/EWRAM/IWRAM".into()})?;
        let base=match region {"rom"=>self.rom,"ewram"=>self.ewram,"iwram"=>self.iwram,_=>return Err(ProbeError::Stale)};
        let address=base.checked_add(offset as u64).ok_or(ProbeError::Stale)?;
        self.process.read_host(address,out)
            .map_err(|e|ProbeError::Read{address:guest_addr,detail:e.to_string()})
    }
    fn fingerprint(&self)->Result<RuntimeFingerprint,ProbeError>{
        if creation_time(self.process.handle.0).map_err(|_|ProbeError::Stale)? != self.process.created_at {
            return Err(ProbeError::Stale);
        }
        if self.read_mapping()? != (self.rom,self.ewram,self.iwram) {
            return Err(ProbeError::Stale);
        }
        let mut header=[0u8;0xC0];
        self.read_guest(0x0800_0000,&mut header)?;
        let (profile,rev)=parse_header(&header)?;
        if profile.code!=self.code || rev!=self.revision {return Err(ProbeError::Stale);}
        Ok(RuntimeFingerprint {
            process_id:self.process.pid,process_created_at:self.process.created_at,
            host_rom_base:self.rom,host_ewram_base:self.ewram,host_iwram_base:self.iwram,
            host_rom_pointer_field:self.pointer_field,game_code:self.code.clone(),revision:self.revision,
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
    pub fn attach(pid:u32)->Result<Self,RuntimeError>{
        let process=Process::open_known_mgba(pid)?;
        let regions=process.memory_regions();
        let raw_roms=process.scan_pattern(&regions,ROM_MARKER,SCAN_LIMIT,MAX_ROM_CANDIDATES)?;
        let mut roms=Vec::new();
        for marker in raw_roms {
            let Some(base)=marker.checked_sub(0xA0) else {continue};
            let mut header=[0;0xC0];
            if process.read_host(base,&mut header).is_ok() && parse_header(&header).is_ok() {
                roms.push(base);
            }
        }
        roms.sort_unstable();roms.dedup();
        if roms.is_empty(){return Err(RuntimeError::MappingUnavailable);}
        let mut mappings=Vec::new();
        let mut seen=HashSet::new();
        for &rom in &roms {
            for width in [8usize,4usize] {
                if width==4 && rom>u32::MAX as u64 {continue;}
                let pattern=rom.to_le_bytes();
                let hits=process.scan_pattern(&regions,&pattern[..width],SCAN_LIMIT,MAX_MAPPING_CANDIDATES)?;
                for slot in hits {
                    if slot%width as u64!=0 || slot<width as u64*2 {continue;}
                    let ewram=process.read_pointer(slot-(width*2) as u64,width).ok();
                    let iwram=process.read_pointer(slot-width as u64,width).ok();
                    let (Some(ewram),Some(iwram))=(ewram,iwram) else {continue};
                    if ewram==0 || iwram==0 || rom==0 {continue;}
                    let mut header=[0u8;0xC0];
                    if process.read_host(rom,&mut header).is_err(){continue;}
                    let Ok((profile,_))=parse_header(&header) else {continue};
                    // Strong candidate criterion mirrors the source's task-table plausibility;
                    // 16 task callback pointers live 0x28 bytes apart in IWRAM.
                    let task_guest=if profile.code=="BPEJ"{0x5B60u64}else{0x5E00u64};
                    let mut count=0;
                    for n in 0..16u64 {
                        if let Ok(callback)=process.read_u32(iwram+task_guest+n*0x28) {
                            if callback==0 || (0x0200_0000..0x0400_0000).contains(&callback)
                                || (0x0800_0000..0x0E00_0000).contains(&callback) {count+=1;}
                        }
                    }
                    if count<12 {continue;}
                    if seen.insert((rom,ewram,iwram,slot,width)) {mappings.push((rom,ewram,iwram,slot,width,profile.code.to_string()));}
                }
            }
        }
        if mappings.is_empty(){return Err(RuntimeError::MappingUnavailable);}
        if mappings.len()!=1 {return Err(RuntimeError::Ambiguous);}
        let (rom,ewram,iwram,pointer_field,pointer_size,code)=mappings.pop().expect("one mapping");
        let reader=GuestHostReader{process,rom,ewram,iwram,pointer_field,pointer_size,code,revision:0};
        let session=EmeraldReadOnlySession::attach(&reader)?;
        Ok(Self{reader,session})
    }
    pub fn session_id(&self)->&str{self.session.session_id()}
    pub fn game_code(&self)->&str{self.session.profile().code}
    pub fn language(&self)->&str{self.session.profile().language}
    pub fn snapshot(&mut self)->Result<RngSnapshot,ProbeError>{
        self.session.read_snapshot(&self.reader)
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
