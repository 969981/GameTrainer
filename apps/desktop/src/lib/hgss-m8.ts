import { invoke } from "@tauri-apps/api/core";
import { isTauriDesktop } from "@/lib/backend";

export interface M8Report {
  process_id:number;
  process_created_filetime:string;
  rom_load_epoch:number;
  code:string;revision:number;
  rom_size:number;
  rom_crc32:string;
  rom_sha1:string;
  catalog_sha1:string|null;
  arm9_port:number;arm7_port:number;
  jit_enabled_when_loaded:boolean;
  core_rom_stream_complete:boolean;
  pipe_process_identity_verified:boolean;
  gdb_socket_pid_binding_verified:boolean;
  running_rom_digest_matches_catalog:boolean;
  diagnostic_identity_verified:boolean;
  dual_cpu_atomic_stop_verified:false;
  jit_cache_coherence_verified:false;
  overlay_restore_verified:false;
  write_authorized:false;
  blockers:string[];
  evidence_scope:string;
}
export const m8Blockers:Record<string,string>={
  core_loaded_rom_hash_incomplete:"模拟器 ROM 字节流尚未完成指纹计算",
  pipe_process_creation_identity_mismatch:"命名管道服务 PID/创建时间与模拟器不匹配",
  both_gdb_ports_not_owned_by_pid:"Windows TCP 表未证明两个 GDB 端口属于该 PID",
  loaded_rom_hash_or_revision_not_verified:"内核运行态 ROM SHA1 / Code / Revision 不在已审计原版名单",
  atomic_arm9_arm7_stop_not_proven:"未证明 ARM9/ARM7 同步原子暂停",
  jit_instruction_cache_not_proven:"未证明本构建 JIT/指令缓存一致性",
  overlay_epoch_restore_not_proven:"未通过 Overlay12 重载与真实恢复验收",
  nds_writes_not_implemented:"GameTrainer 未开放 NDS Guest 写入 API",
};
export function validateM8(pid:number,p9:number,p7:number){
  return [pid,p9,p7].every(Number.isInteger)
    &&pid>0&&p9>=1024&&p9<=65535&&p7>=1024&&p7<=65535&&p9!==p7;
}
export async function probeM8(
  processId:number,gameCode:string,revision:number,arm9Port:number,arm7Port:number,
):Promise<M8Report>{
  if(!isTauriDesktop())throw Error("需要 Windows Tauri 与自行编译的 DeSmuME M8 诊断桥");
  if(!validateM8(processId,arm9Port,arm7Port))throw Error("请输入正确 PID 与不同的 ARM9/ARM7 端口");
  return invoke("m8_hgss_core_rom_identity",{
    processId,gameCode,revision,arm9Port,arm7Port,
  });
}
