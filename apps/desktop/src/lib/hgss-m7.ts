import { invoke } from "@tauri-apps/api/core";
import { isTauriDesktop } from "@/lib/backend";

export type CpuObservation = {cpu:string;port:number;rsp_stop_reply:boolean;pc_first:number|null;pc_second:number|null;error:string|null};
export type M7Report = {
  game_code:string;revision:number;arm9:CpuObservation;arm7:CpuObservation;
  source_overlay12_original_observed:boolean;running_rom_sha1_verified:false;
  shared_emulator_instance_verified:false;simultaneous_cpu_stop_verified:false;
  jit_icache_coherence_verified:false;live_restore_faults_verified:false;
  write_authorized:false;blockers:string[];evidence_scope:string;
};
export const blockerLabels:Record<string,string>={
  unsupported_or_unaudited_rom:"ROM 版本不在 M7 审计名单",
  missing_arm9_stop:"ARM9 停机/PC 验证不足",
  missing_arm7_stop:"ARM7 停机/PC 验证不足",
  different_or_unknown_emulator_process:"两个 GDB Stub 未可信绑定到同一 DeSmuME 进程",
  running_rom_digest_not_bound:"未证明运行中的 ROM 具有登记的完整 SHA1",
  cpu_pair_not_atomically_stopped:"未证明 ARM9 与 ARM7 同步原子停机",
  emulator_jit_and_icache_not_verified:"JIT 和指令缓存一致性未经过实机验收",
  overlay_epoch_not_verified:"Overlay12 本次加载代次和原始指令证据不足",
  guest_restore_fault_injection_not_verified:"尚无真实 Guest 写入、断连和恢复故障注入",
  live_write_backend_intentionally_absent:"M7 后端未提供 NDS 写入接口",
};
export function portsValid(a:number,b:number){
  return Number.isInteger(a)&&Number.isInteger(b)&&a>=1024&&a<=65535&&b>=1024&&b<=65535&&a!==b;
}
export function cpuStable(v:CpuObservation){
  return v.rsp_stop_reply&&v.error===null&&v.pc_first!==null&&v.pc_first===v.pc_second;
}
export async function checkM7(arm9Port:number,arm7Port:number,gameCode:string,revision:number):Promise<M7Report>{
  if(!isTauriDesktop())throw Error("仅 Windows Tauri 桌面版支持实际 GDB 检查");
  if(!portsValid(arm9Port,arm7Port))throw Error("ARM9/ARM7 本地端口必须互不相同，范围 1024–65535");
  return invoke("m7_hgss_preflight",{arm9Port,arm7Port,gameCode,revision});
}
