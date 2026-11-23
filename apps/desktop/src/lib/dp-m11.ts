import { invoke } from "@tauri-apps/api/core";
import { isTauriDesktop } from "@/lib/backend";
import { validDpM10 } from "@/lib/dp-m10";

export type M11Feature="wall_collision"|"capture_english";
export type M11Phase="prepared"|"applied"|"uncertain"|"restored";
export interface M11Plan {
  feature:M11Feature;game_code:string;revision:number;address:number;
  original:number[];replacement:number[];expected_rom_sha256:string;
  source:string;risk_warning:string;
}
export interface M11Preview {
  plan:M11Plan;pid:number;process_created_filetime:string;rom_epoch:string;
  arm9_pc:number;arm7_pc:number;journal_last_phase:M11Phase|null;
  write_risk_ack_required:boolean;warning:string;
}
export interface M11Record {
  owner_id:string;phase:M11Phase;process_id:number;
  process_created_filetime:string;rom_epoch:string;
  arm9_port:number;arm7_port:number;plan:M11Plan;
}
export const M11_ACK="I_ACCEPT_DP_M11_EXPERIMENTAL_GUEST_WRITES";
export function m11SourceAllowed(code:string,revision:number,feature:M11Feature):boolean{
  return (code==="APAE"&&revision===13)||
    (code==="APAI"&&revision===5&&feature==="wall_collision");
}
export function m11Valid(pid:number,p9:number,p7:number,code:string,rev:number,feature:M11Feature):boolean{
  return validDpM10(pid,p9,p7,code,rev)&&m11SourceAllowed(code,rev,feature);
}
function guard(pid:number,p9:number,p7:number,code:string,rev:number,feature:M11Feature){
  if(!isTauriDesktop())throw Error("仅 Windows Tauri 支持真实 DeSmuME 实验写入");
  if(!m11Valid(pid,p9,p7,code,rev,feature))throw Error("原版 SHA256/ROM 版本/双 GDB 端口/补丁类型未在实验允许名单");
}
export async function m11Preview(
  processId:number,gameCode:string,revision:number,
  arm9Port:number,arm7Port:number,feature:M11Feature
):Promise<M11Preview>{
  guard(processId,arm9Port,arm7Port,gameCode,revision,feature);
  return invoke("m11_dp_preview",{processId,gameCode,revision,arm9Port,arm7Port,feature});
}
export async function m11Apply(
  processId:number,gameCode:string,revision:number,
  arm9Port:number,arm7Port:number,feature:M11Feature,
  experimentalAck:string
):Promise<M11Record>{
  guard(processId,arm9Port,arm7Port,gameCode,revision,feature);
  if(experimentalAck!==M11_ACK)throw Error("缺少明确实验风险确认");
  return invoke("m11_dp_apply",{
    processId,gameCode,revision,arm9Port,arm7Port,feature,experimentalAck
  });
}
export async function m11Restore(
  processId:number,gameCode:string,revision:number,
  arm9Port:number,arm7Port:number,feature:M11Feature
):Promise<M11Record>{
  guard(processId,arm9Port,arm7Port,gameCode,revision,feature);
  return invoke("m11_dp_restore",{processId,gameCode,revision,arm9Port,arm7Port,feature});
}
export function hexBytes(b:number[]):string{
  return b.map(x=>x.toString(16).toUpperCase().padStart(2,"0")).join(" ");
}
