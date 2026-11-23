import { invoke } from "@tauri-apps/api/core";
import { isTauriDesktop } from "@/lib/backend";

export interface M6OverlayReport {
  game_code: string;
  revision: number;
  arm9_pc: number;
  gdb_stop_verified: boolean;
  field_system: number | null;
  process_manager: number | null;
  overlay_child: number | null;
  overlay_id: number | null;
  execution_state: number | null;
  overlay_load_addr: number;
  capture_addr: number;
  observed_halfword: number | null;
  neighbor_signature_valid: boolean;
  state:
    | "not_loaded" | "pending_execution" | "original_verified"
    | "foreign_patched" | "signature_mismatch" | "unexpected_instruction";
  running_rom_sha1_verified: false;
  write_authorized: false;
  evidence_scope: string;
}

export async function probeArm9Stop(
  port: number,
  gameCode: string,
  revision: number,
): Promise<M6OverlayReport> {
  if (!isTauriDesktop()) throw Error("M6 需要 Tauri 桌面应用和支持 ARM9 GDB 的 DeSmuME");
  if (!Number.isInteger(port) || port < 1024 || port > 65535)
    throw Error("请输入 1024 至 65535 的本地 GDB 端口");
  return invoke<M6OverlayReport>("m6_hgss_probe_arm9_stop", {
    port, gameCode, revision,
  });
}

export function m6Transition(a: M6OverlayReport, b: M6OverlayReport): string {
  if (a.game_code !== b.game_code || a.revision !== b.revision)
    return "切换游戏版本，证据不可复用";
  const loaded = (x: M6OverlayReport) =>
    ["original_verified", "foreign_patched", "signature_mismatch", "unexpected_instruction"]
      .includes(x.state);
  if (!loaded(a) && loaded(b)) return "检测到 Overlay12 进入战斗状态";
  if (loaded(a) && !loaded(b)) return "检测到 Overlay12 退出战斗状态";
  if (loaded(a) && loaded(b) && a.overlay_child !== b.overlay_child)
    return "Overlay 管理器更换，旧字节所有权不可继承";
  if (loaded(a) && loaded(b) &&
      (a.observed_halfword !== b.observed_halfword ||
        a.neighbor_signature_valid !== b.neighbor_signature_valid))
    return "目标指令发生变化；存在第三方修改或重载风险";
  return "没有检测到可证明的状态变化";
}
