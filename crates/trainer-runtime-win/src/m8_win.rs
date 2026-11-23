//! M8: Windows-only DeSmuME named-pipe + TCP owner PID verification.
//! This code obtains the digest from the running DeSmuME core's patched
//! NDS_LoadROM byte-stream bridge; never opens a user-selected ROM file.
//! No Guest writes. OS PID binding is diagnostic, not cryptographic attestation.
use std::{
    ffi::OsStr,
    mem::size_of,
    os::windows::ffi::OsStrExt,
    ptr::null_mut,
};
use trainer_hgss::m8::{
    decode_packet, evaluate, M8Report, OsBinding, WIRE_SIZE,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE},
    NetworkManagement::IpHelper::{GetExtendedTcpTable, MIB_TCPROW_OWNER_PID, TCP_TABLE_OWNER_PID_ALL},
    Networking::WinSock::AF_INET,
    Storage::FileSystem::{
        CreateFileW, ReadFile, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, OPEN_EXISTING,
    },
    System::{
        Pipes::{GetNamedPipeServerProcessId, WaitNamedPipeW},
        Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
    },
};
use crate::list_desmume_targets;

struct Owned(windows_sys::Win32::Foundation::HANDLE);
impl Drop for Owned {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE && !self.0.is_null() {
            unsafe { CloseHandle(self.0) };
        }
    }
}
fn utf16(s:&str)->Vec<u16>{OsStr::new(s).encode_wide().chain(Some(0)).collect()}
fn created_filetime(pid:u32)->Result<u64,String>{
    let handle=Owned(unsafe {OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,0,pid)});
    if handle.0.is_null() {return Err("OpenProcess failed for selected DeSmuME".into());}
    let mut created=FILETIME{dwLowDateTime:0,dwHighDateTime:0};
    let mut exit=created;let mut kernel=created;let mut user=created;
    if unsafe{GetProcessTimes(handle.0,&mut created,&mut exit,&mut kernel,&mut user)}==0 {
        return Err("GetProcessTimes failed".into());
    }
    Ok(((created.dwHighDateTime as u64)<<32)|(created.dwLowDateTime as u64))
}
fn owned_port_pair(pid:u32,arm9:u16,arm7:u16)->Result<bool,String>{
    if arm9<1024||arm7<1024||arm9==arm7 {return Ok(false);}
    let mut needed=0u32;
    unsafe {
        GetExtendedTcpTable(null_mut(),&mut needed,0,AF_INET as u32,
            TCP_TABLE_OWNER_PID_ALL,0);
    }
    if needed < 4 || needed > 4*1024*1024 {
        return Err("IPv4 TCP owner table size unavailable/invalid".into());
    }
    let mut buffer=vec![0u8;needed as usize];
    let code=unsafe {
        GetExtendedTcpTable(buffer.as_mut_ptr().cast(),&mut needed,0,
            AF_INET as u32,TCP_TABLE_OWNER_PID_ALL,0)
    };
    if code!=0 {return Err(format!("GetExtendedTcpTable failed: {code}"));}
    if buffer.len()<4 {return Err("short TCP PID table".into());}
    let n=u32::from_ne_bytes(buffer[..4].try_into().expect("4 bytes")) as usize;
    let row_size=size_of::<MIB_TCPROW_OWNER_PID>();
    if n > (buffer.len()-4)/row_size {return Err("truncated TCP PID owner table".into());}
    let mut found9=false;let mut found7=false;
    for i in 0..n {
        let at=4+i*row_size;
        // Win32 tables are DWORD-aligned; byte buffer is not guaranteed aligned.
        let row:MIB_TCPROW_OWNER_PID=unsafe {
            std::ptr::read_unaligned(buffer[at..].as_ptr().cast())
        };
        if row.dwOwningPid != pid {continue;}
        let port=u16::from_be((row.dwLocalPort & 0xffff) as u16);
        if port==arm9 {found9=true;}
        if port==arm7 {found7=true;}
    }
    Ok(found9&&found7)
}
fn read_one_pipe(pid:u32)->Result<([u8;WIRE_SIZE],u32),String>{
    let name=utf16(&format!(r"\\.\pipe\GameTrainer.HGSS.M8.{pid}"));
    if unsafe{WaitNamedPipeW(name.as_ptr(),1500)}==0 {
        return Err("M8 pipe absent: install source-patched GDB_STUB + GT_HGSS_M8_BRIDGE DeSmuME".into());
    }
    let file=Owned(unsafe {
        CreateFileW(name.as_ptr(),FILE_GENERIC_READ,0,null_mut(),
            OPEN_EXISTING,FILE_ATTRIBUTE_NORMAL,null_mut())
    });
    if file.0==INVALID_HANDLE_VALUE {
        return Err("M8 pipe open failed: check Windows permissions and bridge process".into());
    }
    let mut server=0u32;
    if unsafe {GetNamedPipeServerProcessId(file.0,&mut server)}==0 {
        return Err("Windows could not identify M8 pipe server PID".into());
    }
    let mut bytes=[0u8;WIRE_SIZE];
    let mut filled=0usize;
    while filled<bytes.len(){
        let mut count=0u32;
        let success=unsafe {
            ReadFile(file.0,bytes[filled..].as_mut_ptr().cast(),
                (WIRE_SIZE-filled) as u32,&mut count,null_mut())
        };
        if success==0 || count==0 {return Err("M8 pipe short response/disconnected".into());}
        filled+=count as usize;
    }
    Ok((bytes,server))
}

/// The selected PID is rechecked before/after pipe reads. Both GDB TCP ports
/// must be owned by that PID in Windows' kernel network table. A user-entered
/// port value or socket Stop Reply cannot impersonate the OS owner check.
pub fn read_desmume_runtime_rom_identity(
    pid:u32,code:&str,revision:u8,arm9:u16,arm7:u16
)->Result<M8Report,String>{
    if arm9<1024||arm7<1024||arm9==arm7 {
        return Err("M8 requires two distinct GDB ports >=1024".into());
    }
    let targets=list_desmume_targets().map_err(|e|e.to_string())?;
    if !targets.iter().any(|x|x.process_id==pid) {
        return Err("PID is not a visible DeSmuME target".into());
    }
    let before=created_filetime(pid)?;
    let (data,server_pid)=read_one_pipe(pid)?;
    let packet=decode_packet(&data)?;
    let ports_owned=owned_port_pair(pid,arm9,arm7)?;
    let after=created_filetime(pid)?;
    Ok(evaluate(&packet,pid,code,revision,arm9,arm7,OsBinding{
        pipe_server_pid_matches:server_pid==pid,
        creation_filetime_matches:before==packet.created_filetime,
        gdb_ports_owned_by_pid:ports_owned,
        process_still_same:before==after,
    }))
}
