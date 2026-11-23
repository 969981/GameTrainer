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
  verification: "partial_anchors_read_only";
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
