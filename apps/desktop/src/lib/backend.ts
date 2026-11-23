import { invoke } from "@tauri-apps/api/core";
import mockFixture from "../../../../fixtures/m0-registry.json";
import type { GameCatalog, SessionSnapshot } from "@/types/catalog";

export function isTauriDesktop(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function getCatalog(): Promise<GameCatalog> {
  if (isTauriDesktop()) {
    // A failed Tauri invocation must surface as an error, not fake a successful Mock fallback.
    return invoke<GameCatalog>("get_catalog");
  }
  return mockFixture as GameCatalog;
}

export async function getSessionSnapshot(): Promise<SessionSnapshot> {
  if (isTauriDesktop()) {
    return invoke<SessionSnapshot>("get_session_snapshot");
  }
  return {
    state: "detached",
    read_only: true,
    session_id: null,
    game_id: null,
    runtime_id: null,
    profile_id: null,
    message: "浏览器 Mock 模式：未连接游戏进程",
  };
}
