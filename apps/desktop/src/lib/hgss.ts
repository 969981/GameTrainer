import { invoke } from "@tauri-apps/api/core";
import { isTauriDesktop } from "@/lib/backend";
import profilesFixture from "../../../../fixtures/hgss-v1451-profiles.json";

export interface HgssProfile {
  game_code: string; revision: number; game_id: string; game: string;
  language: string; sha1: string; current_seed_addr: number;
  mt_state0_addr: number; mt_index_addr: number; pid_pointer_addr: number;
  trainer_ids_pointer_addr: number; delay_addr: number;
  static_audit_1451: boolean; write_allowed: false;
}
export interface DesmumeProcess {
  process_id: number; executable_name: string; created_at: number;
}
export interface HgssSnapshot {
  session_id: string; game_id: string; game_code: string;
  revision: number; language: string; current_seed: number;
  mt_state0: number; mt_index: number; delay: number;
  trainer_id: number | null; secret_id: number | null;
  verification: string; read_only: true; advances_verified: false;
}
export const hgssProfiles = profilesFixture as HgssProfile[];
const requireDesktop = () => {
  if (!isTauriDesktop()) throw Error("此操作仅支持 Windows Tauri + DeSmuME，不提供模拟成功回执。");
};
export async function getDesmumeProcesses(): Promise<DesmumeProcess[]> {
  requireDesktop(); return invoke("m5_list_desmume");
}
export async function attachHgss(processId: number, gameId: string): Promise<HgssSnapshot> {
  requireDesktop(); return invoke("m5_attach_hgss", { processId, gameId });
}
export async function readHgss(): Promise<HgssSnapshot> {
  requireDesktop(); return invoke("m5_hgss_snapshot");
}
export async function detachHgss(): Promise<void> {
  requireDesktop(); return invoke("m5_detach_hgss");
}
export interface LocalRomEvidence {
  filename: string; gameCode: string; revision: number; sha1: string;
  match: HgssProfile | null; matched: boolean; error: string | null;
}
/** SHA1 is computed in the local WebView. This verifies an offline file only;
 * it does NOT bind the file to a DeSmuME process and cannot grant patch rights. */
export async function verifyLocalHgssRom(file: File): Promise<LocalRomEvidence> {
  if (!file.name.toLowerCase().endsWith(".nds")) throw Error("请选择本地 .nds 文件");
  if (file.size < 0x200 || file.size > 512 * 1024 * 1024) throw Error("NDS 镜像大小异常");
  const header = new Uint8Array(await file.slice(0, 0x200).arrayBuffer());
  const gameCode = String.fromCharCode(...header.slice(0x0c, 0x10));
  const revision = header[0x1e];
  const bytes = await file.arrayBuffer();
  const digest = await crypto.subtle.digest("SHA-1", bytes);
  const sha1 = Array.from(new Uint8Array(digest), b => b.toString(16).padStart(2, "0")).join("");
  const match = hgssProfiles.find(p => p.game_code === gameCode && p.revision === revision) ?? null;
  return {
    filename: file.name, gameCode, revision, sha1, match, matched: !!match && match.sha1 === sha1,
    error: !match ? "此 Game Code/Revision 不在 14 版本目录" :
      match.sha1 !== sha1 ? "SHA1 与登记的原版 ROM 不一致（汉化版/补丁版/不同发行版）" : null,
  };
}
