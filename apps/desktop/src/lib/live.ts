import { invoke } from "@tauri-apps/api/core";
import { isTauriDesktop } from "@/lib/backend";

export interface LiveIdentity {
  session_id:string;game_id:string;build_id:string;game_code:string;revision:number;
  sha1:string;rom_bytes:number;gdb_paused:boolean;transport:string;
  experimental_hook_supported:boolean;active_feature:string|null;active_status:string|null;
}
export interface LivePreviewSpan {label:string;guest_address:string;before:string;after:string;}
export interface LivePreview{
  token:string;game_id:string;feature_id:string;rom_sha1:string;
  spans:LivePreviewSpan[];expires_in_seconds:number;risk:string;requires_confirmation:boolean;
}
export interface LiveReceipt{
  owner_id:string;feature_id:string;state:string;audit_committed:boolean;emulator_running:boolean;
}
export interface LiveAuditRow{
  event_id:string;owner_id:string|null;session_id:string;game_id:string;feature_id:string;
  action_kind:string;status:string;summary:string;error_code:string|null;occurred_at_ms:number;
}
function required(){if(!isTauriDesktop())throw Error("真实 GDB 修改仅支持 Tauri 桌面版。");}
export async function liveStatus():Promise<LiveIdentity|null>{required();return invoke("m3_live_status");}
export async function connectLive(port:number):Promise<LiveIdentity>{required();return invoke("m3_live_connect",{port});}
export async function previewCapture():Promise<LivePreview>{required();return invoke("m3_preview_live_capture");}
export async function previewCrit(template:number[]):Promise<LivePreview>{required();return invoke("m3_preview_live_crit",{template});}
export async function applyLive(token:string):Promise<LiveReceipt>{
  required();
  return invoke("m3_apply_live_preview",{token,confirmation:"我已备份存档，仅在离线单机测试"});
}
export async function restoreLive():Promise<LiveReceipt>{required();return invoke("m3_restore_live_patch");}
export async function cancelLive():Promise<void>{required();return invoke("m3_cancel_live_preview");}
export async function disconnectLive():Promise<void>{required();return invoke("m3_disconnect_live");}
export async function liveHistory():Promise<LiveAuditRow[]>{required();return invoke("m3_live_history");}
