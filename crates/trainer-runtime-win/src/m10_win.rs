//! M10 DP on Windows: source-patched in-core ROM SHA256 + same PID dual GDB
//! ports. No external ROM filename, no Guest memory write or CPU resume.
use crate::list_desmume_targets;
use crate::m8_win::{created_filetime, owned_port_pair, read_named_pipe};
use trainer_dp::m10::{decode_packet, evaluate, Identity, OsBinding, Packet, WIRE_SIZE};

/// Retain the exact packet for a second read after independent GDB probing.
/// A ROM change between the two samples invalidates the proof epoch.
pub fn read_dp_core_identity(
    pid:u32,game_code:&str,revision:u8,arm9_port:u16,arm7_port:u16,
)->Result<(Packet,Identity),String>{
    if pid==0 || arm9_port<1024 || arm7_port<1024 || arm9_port==arm7_port{
        return Err("M10 invalid DeSmuME PID or ARM9/ARM7 port pair".into());
    }
    let targets=list_desmume_targets().map_err(|e|e.to_string())?;
    if !targets.iter().any(|t|t.process_id==pid){
        return Err("M10 selected PID is not a visible DeSmuME process".into());
    }
    let created_before=created_filetime(pid)?;
    let (bytes,server_pid)=read_named_pipe::<WIRE_SIZE>(
        &format!(r"\\.\pipe\GameTrainer.DP.M10.{pid}"))?;
    let packet=decode_packet(&bytes)?;
    let port_owners=owned_port_pair(pid,arm9_port,arm7_port)?;
    let created_after=created_filetime(pid)?;
    let identity=evaluate(&packet,pid,game_code,revision,arm9_port,arm7_port,
        OsBinding{
            named_pipe_server_pid_matches:server_pid==pid,
            process_creation_filetime_matches:created_before==packet.process_created_filetime,
            both_gdb_tcp_ports_owned_by_pid:port_owners,
            process_stable_after_read:created_before==created_after,
        });
    Ok((packet,identity))
}
