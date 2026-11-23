import { invoke } from "@tauri-apps/api/core";
import { isTauriDesktop } from "@/lib/backend";

export type PatchStatus = "pending" | "active" | "conflict" | "restore_failed" | "restored";
export type AuditEnvironment = "fixture" | "live";
export interface PatchPreview {
  preview_id: string;
  session_id: string;
  epoch: number;
  game_id: string;
  feature_id: string;
  profile_id: string;
  address_display: string;
  before_display: string;
  after_display: string;
  expires_in_seconds: number;
  risk: string;
  environment: "fixture";
  status: PatchStatus;
}
export interface PatchReceipt {
  owner_id: string;
  session_id: string;
  feature_id: string;
  status: PatchStatus;
  environment: "fixture";
  audited: boolean;
}
export interface FixtureSnapshot {
  session_id: string;
  epoch: number;
  seed: number;
  game_id: string;
  seed_address_display: string;
  active_patches: PatchReceipt[];
  read_only_game_runtime: true;
  environment: "fixture";
}
export interface FixtureStatus {
  connected: boolean;
  fixture: FixtureSnapshot | null;
  unresolved_fixture_intents: number;
  readonly_live_runtime: true;
  audit_scope: "fixture";
}
export interface ActionEvent {
  event_id: string;
  owner_id: string | null;
  preview_id: string | null;
  session_id: string;
  game_id: string;
  feature_id: string;
  environment: AuditEnvironment;
  action_kind: string;
  status: string;
  summary: string;
  error_code: string | null;
  occurred_at_ms: number;
}
function requireTauri(): void {
  if (!isTauriDesktop()) throw Error("浏览器预览不提供本地 SQLite 或 M2 事务命令，请使用桌面程序。");
}
export function parseHexSeed(text: string): number | null {
  const trimmed = text.trim();
  if (!/^(?:0x)?[0-9a-f]{1,8}$/i.test(trimmed)) return null;
  const value = Number.parseInt(trimmed.replace(/^0x/i, ""), 16);
  return Number.isSafeInteger(value) && value >= 0 && value <= 0xFFFF_FFFF ? value : null;
}
export function hexSeed(value: number): string {
  return "0x" + (value >>> 0).toString(16).toUpperCase().padStart(8, "0");
}
export async function getFixtureStatus(): Promise<FixtureStatus> {
  requireTauri();
  return invoke<FixtureStatus>("m2_fixture_status");
}
export type FixtureGameId = "games.pokemon.emerald" | "games.pokemon.ruby" | "games.pokemon.sapphire";
export async function startFixture(gameId: FixtureGameId): Promise<FixtureStatus> {
  requireTauri();
  return invoke<FixtureStatus>("m2_start_fixture", { gameId });
}
export async function previewFixtureSeed(seed: number): Promise<PatchPreview> {
  requireTauri();
  return invoke<PatchPreview>("m2_preview_fixture_seed", { targetSeed: seed });
}
export async function cancelFixturePreview(previewId: string): Promise<void> {
  requireTauri();
  return invoke<void>("m2_cancel_fixture_preview", { previewId });
}
export async function applyFixturePreview(previewId: string): Promise<PatchReceipt> {
  requireTauri();
  return invoke<PatchReceipt>("m2_apply_fixture_preview", { previewId });
}
export async function restoreFixturePatch(ownerId: string): Promise<PatchReceipt> {
  requireTauri();
  return invoke<PatchReceipt>("m2_restore_fixture_patch", { ownerId });
}
export async function listActionHistory(environment: AuditEnvironment): Promise<ActionEvent[]> {
  requireTauri();
  return invoke<ActionEvent[]>("m2_list_action_history", { environment });
}
