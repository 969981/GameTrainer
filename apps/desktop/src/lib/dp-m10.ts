import { invoke } from "@tauri-apps/api/core";
import { isTauriDesktop } from "@/lib/backend";
import { dpProfiles, type DpSnapshot } from "@/lib/dp";

export interface DpM10Identity {
  process_id:number;process_created_filetime:string;rom_epoch:string;
  game_code:string;revision:number;header6c:number;rom_size:number;
  crc32:string;core_sha256:string;known_full_rom_sha256:string|null;
  full_rom_reference_available:boolean;full_rom_matches_catalog:boolean;
  core_rom_stream_complete:boolean;pipe_and_process_bound:boolean;
  both_gdb_ports_bound:boolean;diagnostic_identity_verified:boolean;
  arm9_port:number;arm7_port:number;jit_enabled_when_loaded:boolean;
  guest_write_allowed:false;blockers:string[];
}
export interface DpM10Cpu {
  cpu:string;port:number;stop_reply_observed:boolean;
  first_pc:number|null;second_pc:number|null;error:string|null;
}
export interface DpM10Audit {
  identity:DpM10Identity;
  rom_epoch_stable_across_gdb_samples:boolean;
  arm9:DpM10Cpu;arm7:DpM10Cpu;
  independent_stops_observed:boolean;
  cpu_pair_atomically_stopped:false;
  arm9_pc_inside_known_thumb_cave:boolean|null;
  overlay11_static_source_present:boolean;
  overlay11_guest_observation:DpSnapshot|null;
  jit_icache_coherence_verified:false;
  guest_thumb_execution_verified:false;
  original_instruction_restoration_verified:false;
  write_authorized:false;
  blockers:string[];
}
export const dpM10Blockers:Record<string,string>={
  core_loaded_rom_stream_incomplete:"DeSmuME 正在加载的 ROM 完整 SHA256 尚无有效内核字节流",
  pipe_pid_or_process_creation_not_bound:"命名管道真实服务 PID 与进程创建时间未绑定",
  arm9_arm7_tcp_owner_pid_not_bound:"ARM9 与 ARM7 GDB TCP 端口未证明同属该 Windows PID",
  game_code_revision_header6c_mismatch:"Game Code、ROM 修订或 Header 0x6C 与原版资料不一致",
  no_original_full_rom_sha256_reference:"本版本仅有 ARM9/Overlay06 分量指纹，无完整原版 ROM SHA256 参照",
  running_rom_sha256_not_catalog_match:"内核加载 ROM 的 SHA256 与完整原版指纹不匹配",
  arm9_arm7_atomic_stop_unverified:"两个 GDB 的独立 Stop Reply 尚不能证明双 CPU 原子同步停机",
  jit_icache_coherence_unverified:"JIT/ICache 代码可见性及一致性未实机验证",
  overlay11_loading_epoch_unverified:"Overlay11 本次加载/卸载/重入代次尚未实机核验",
  guest_thumb_5a_a5_execution_unverified:"0x5A／0xA5 Guest Thumb 原生执行未获取实机证据",
  overlay11_original_restore_faults_unverified:"原指令恢复和断线/故障注入未实机验收",
  nds_guest_writes_disabled:"NDS 写入 API 被禁用",
  rom_changed_during_gdb_observations:"采样 ARM9/ARM7 时 ROM 或模拟器会话身份发生变化",
  arm9_stop_reply_or_registers_unverified:"ARM9 停机或 PC 寄存器双次读取不可靠",
  arm7_stop_reply_or_registers_unverified:"ARM7 停机或 PC 寄存器双次读取不可靠",
  overlay11_live_read_only_observation_missing:"未在当前 Guest 快照中验证 Overlay11 静态调查指令",
  arm9_pc_currently_inside_thumb_cave:"ARM9 PC 当前处于候选 72B Thumb Cave 区间",
};
export function validDpM10(pid:number,p9:number,p7:number,code:string,rev:number):boolean{
  return [pid,p9,p7,rev].every(Number.isInteger) &&pid>0
    &&p9>=1024&&p9<=65535&&p7>=1024&&p7<=65535&&p9!==p7
    &&dpProfiles.some(p=>p.game_code===code&&p.revision===rev);
}
function guard(pid:number,port9:number,port7:number,code:string,rev:number){
  if(!isTauriDesktop())throw Error("仅 Windows Tauri + 自编译 DeSmuME 双 GDB / M10 ROM 内核桥可使用");
  if(!validDpM10(pid,port9,port7,code,rev))throw Error("DeSmuME PID、ARM9/ARM7 端口或 D/P 版本无效");
}
export async function dpM10Identity(
  processId:number,arm9Port:number,arm7Port:number,gameCode:string,revision:number,
):Promise<DpM10Identity>{
  guard(processId,arm9Port,arm7Port,gameCode,revision);
  return invoke("m10_dp_core_identity",{processId,gameCode,revision,arm9Port,arm7Port});
}
export async function dpM10RuntimeAudit(
  processId:number,arm9Port:number,arm7Port:number,gameCode:string,revision:number,
):Promise<DpM10Audit>{
  guard(processId,arm9Port,arm7Port,gameCode,revision);
  return invoke("m10_dp_dual_cpu_overlay11_readonly",{processId,gameCode,revision,arm9Port,arm7Port});
}
