import { invoke } from "@tauri-apps/api/core";
import { isTauriDesktop } from "@/lib/backend";
import catalog from "../../../../fixtures/dp-v0814-profiles.json";

export interface DpProfile {
  game_code:string;revision:number;header6c:number;game_id:string;game:string;language:string;
  arm9_sha256:string;overlay06_sha256:string;arm9_size:number;rom_sha256:string|null;
  current_seed_addr:number;mt_seed_addr:number;mt_index_addr:number;
  delay_addr:number;trainer_ptr_addr:number;pid_ptr_addr:number;
  field_global_addr:number;field_entry_addr:number;warp_addr:number;
  collision_addr:number;cave_addr:number;
  battle_static_audit:boolean;guest_exec_runtime_verified:false;write_allowed:false;
}
export interface DpBattle {
  version_static_evidence:boolean;overlay11_callsite:number;observed_thumb_call_bytes:string;
  thumb_bl_candidate:boolean;source_cave_addr:number;cave_all_zero:boolean;
  guest_execution_confirmed:false;code_cave_restore_safe:false;
}
export interface DpSnapshot {
  source_version:string;game_id:string;game_code:string;revision:number;language:string;
  arm9_pc:number;gdb_stop_observed:boolean;header6c:number;arm9_anchors_match:boolean;
  current_seed:number;mt_state0:number;mt_index:number;delay:number;
  trainer_ptr:number;pid_ptr:number;battle:DpBattle|null;
  complete_running_rom_sha256_verified:false;write_authorized:false;evidence_scope:string;
}
export const dpProfiles=catalog.profiles as DpProfile[];
export const hex8=(v:number)=>"0x"+v.toString(16).toUpperCase().padStart(8,"0");
export function canProbe(port:number,code:string,rev:number):boolean{
  return Number.isInteger(port)&&port>=1024&&port<=65535&&
    dpProfiles.some(p=>p.game_code===code&&p.revision===rev);
}
export async function probeDpArm9(port:number,code:string,rev:number):Promise<DpSnapshot>{
  if(!isTauriDesktop())throw Error("仅 Windows Tauri + ARM9 GDB Stub 支持真实读取");
  if(!canProbe(port,code,rev))throw Error("ARM9 端口或 DP Code/Revision 不支持");
  return invoke("m9_dp_read_arm9",{arm9Port:port,gameCode:code,revision:rev});
}
export interface DpOfflineEvidence{
  gameCode:string;revision:number;header6c:number;sha256:string;
  profile:DpProfile|null;romSha256Matched:boolean;
  fullRomReferenceAvailable:boolean;note:string;
}
export async function checkLocalDpRom(file:File):Promise<DpOfflineEvidence>{
  if(!file.name.toLowerCase().endsWith(".nds"))throw Error("仅支持本地 .nds 文件");
  if(file.size<0x200||file.size>512*1024*1024)throw Error("NDS 文件大小异常");
  const header=new Uint8Array(await file.slice(0,0x200).arrayBuffer());
  const gameCode=String.fromCharCode(...header.slice(0x0c,0x10));
  const revision=header[0x1e],header6c=header[0x6c];
  const digest=new Uint8Array(await crypto.subtle.digest("SHA-256",await file.arrayBuffer()));
  const sha256=Array.from(digest,b=>b.toString(16).padStart(2,"0")).join("");
  const profile=dpProfiles.find(p=>p.game_code===gameCode&&p.revision===revision&&p.header6c===header6c)??null;
  const expected=profile?.rom_sha256??null;
  return {gameCode,revision,header6c,sha256,profile,
    romSha256Matched:!!expected&&expected===sha256,fullRomReferenceAvailable:!!expected,
    note:!profile?"ROM Code/Rev/Header6C 未列入源包":
      !expected?"仅提供 ARM9 与 Overlay06 SHA256，无完整 ROM 指纹；不能判定镜像匹配":
      expected===sha256?"与原版完整 ROM SHA256 相符（仍是离线文件）":
      "完整 ROM SHA256 不同，可能是修改版或其它修订",
  };
}
