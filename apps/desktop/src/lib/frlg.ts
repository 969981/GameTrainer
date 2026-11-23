import { invoke } from "@tauri-apps/api/core";
import { isTauriDesktop } from "@/lib/backend";

export interface FrlgStatus {
  session_id: string;
  game_id: string;
  profile_id: string;
  game_code: string;
  revision: number;
  language: string;
  rom_sha1: string;
  transport: string;
  guest_paused: boolean;
  active_feature: string | null;
  active_state: string | null;
}
export interface FrlgProfile {
  profile_id: string;
  game_code: string;
  revision: number;
  game_id: string;
  language: string;
  crc32: string;
  sha1: string;
  capture_patch: number;
  wall_patch: number;
  wild_entry: number;
}
export interface FrlgSpan {
  label: string;
  address: string;
  before: string;
  after: string;
}
export interface FrlgDiff {
  token: string;
  feature_id: string;
  rom_sha1: string;
  expires_in_seconds: number;
  spans: FrlgSpan[];
  experimental: boolean;
}
export interface FrlgReceipt {
  owner_id: string;
  feature_id: string;
  status: string;
  guest_running: boolean;
  audit_recorded: boolean;
}
export interface FrlgMailbox {
  target_seed: number;
  mode: number;
  active_hook: number;
  last_trigger: number;
  hit_count: number;
  last_old_seed: number;
}
export interface FrlgEvent {
  event_id: string;
  session_id: string;
  feature_id: string;
  action_kind: string;
  status: string;
  summary: string;
  occurred_at_ms: number;
}
function liveOnly() {
  if (!isTauriDesktop()) throw new Error("FRLG 真实写入仅在 Tauri 桌面应用中可用。");
}
const confirm = "已备份存档并确认仅供离线单机实验";

export async function frlgProfiles(): Promise<FrlgProfile[]> {
  liveOnly();
  return invoke("m4_frlg_profiles");
}
export async function frlgStatus(): Promise<FrlgStatus | null> {
  liveOnly();
  return invoke("m4_frlg_status");
}
export async function frlgConnect(port: number): Promise<FrlgStatus> {
  liveOnly();
  return invoke("m4_frlg_connect", { port });
}
export async function frlgPreview(feature: "capture" | "wall" | "wild", template?: number[]): Promise<FrlgDiff> {
  liveOnly();
  return invoke("m4_frlg_preview", { feature, template: template ?? null });
}
export async function frlgApply(token: string): Promise<FrlgReceipt> {
  liveOnly();
  return invoke("m4_frlg_apply", { token, confirmation: confirm });
}
export async function frlgArmWild(targetSeed: number): Promise<FrlgMailbox> {
  liveOnly();
  return invoke("m4_frlg_arm_wild", { targetSeed, confirmation: confirm });
}
export async function frlgRestore(): Promise<FrlgReceipt> {
  liveOnly();
  return invoke("m4_frlg_restore");
}
export async function frlgCancel(): Promise<void> {
  liveOnly();
  return invoke("m4_frlg_cancel");
}
export async function frlgDisconnect(): Promise<void> {
  liveOnly();
  return invoke("m4_frlg_disconnect");
}
export async function frlgPause(): Promise<FrlgStatus> {
  liveOnly();
  return invoke("m4_frlg_pause");
}
export async function frlgResume(): Promise<FrlgStatus> {
  liveOnly();
  return invoke("m4_frlg_resume");
}
export async function frlgMailbox(): Promise<FrlgMailbox> {
  liveOnly();
  return invoke("m4_frlg_mailbox");
}
export async function frlgHistory(): Promise<FrlgEvent[]> {
  liveOnly();
  return invoke("m4_frlg_history");
}
