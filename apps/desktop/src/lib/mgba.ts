import { invoke } from "@tauri-apps/api/core";
import { isTauriDesktop } from "@/lib/backend";

export interface MgbaProcess {
  process_id: number;
  executable_name: string;
  created_at: number;
}

export interface EmeraldConnectReceipt {
  session_id: string;
  game_code: string;
  language: string;
  game_id: string;
  revision: number;
  verification: "partial_anchors_read_only" | "header_revision_boot_anchor_only";
  read_only: true;
}

export interface RngReadOnlySnapshot {
  session_id: string;
  epoch: number;
  game_code: string;
  language: string;
  current_seed: number;
  initial_seed: number;
  advances: number;
  verification: "partial_anchors_read_only";
  read_only: true;
}

function desktopRequired() {
  if (!isTauriDesktop()) {
    throw new Error("浏览器 Mock 模式不能连接真实游戏。请使用 Windows Tauri 桌面版。");
  }
}
export async function getMgbaProcesses(): Promise<MgbaProcess[]> {
  desktopRequired();
  return invoke<MgbaProcess[]>("list_mgba_processes");
}
export async function connectEmeraldReadOnly(processId: number): Promise<EmeraldConnectReceipt> {
  desktopRequired();
  return invoke<EmeraldConnectReceipt>("attach_emerald_read_only", { processId });
}
export async function readEmeraldRng(): Promise<RngReadOnlySnapshot> {
  desktopRequired();
  return invoke<RngReadOnlySnapshot>("read_emerald_rng");
}
export async function disconnectEmerald(): Promise<void> {
  desktopRequired();
  return invoke<void>("detach_emerald");
}


export type RsGameId = "games.pokemon.ruby" | "games.pokemon.sapphire";
export interface RsProfile {
  game_code: string;
  revision: number;
  game: "ruby" | "sapphire";
  language: string;
  build_id: string;
  layout: "usa" | "japanese" | "europe";
  seed_address: number;
  timer_address: number;
  profile_strength: string;
  write_allowed: false;
}
export interface RsRngSnapshot {
  session_id: string;
  epoch: number;
  game_id: RsGameId;
  game_code: string;
  revision: number;
  language: string;
  current_seed: number;
  timer: number;
  verification: string;
  read_only: true;
}
export async function listRsProfiles(): Promise<RsProfile[]> {
  desktopRequired();
  return invoke<RsProfile[]>("list_rs_profiles");
}
export async function connectRsReadOnly(processId: number, gameId: RsGameId): Promise<EmeraldConnectReceipt> {
  desktopRequired();
  return invoke<EmeraldConnectReceipt>("attach_rs_read_only", { processId, gameId });
}
export async function readRsRng(): Promise<RsRngSnapshot> {
  desktopRequired();
  return invoke<RsRngSnapshot>("read_rs_rng");
}
