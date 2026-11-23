export type PlatformFamily = "gen3" | "gen4" | "gen5" | "gen6" | "pc";
export type ImplementationStatus = "mock" | "planned" | "template_only";
export type ParameterKind = "text" | "hex" | "integer";
export type ViewId = "home" | "library" | "workspace" | "modules" | "runtime" | "history" | "journal" | "settings";

export interface ParameterDescriptor {
  id: string;
  label: string;
  kind: ParameterKind;
  placeholder: string;
}
export interface FeatureDescriptor {
  feature_id: string;
  name: string;
  group: string;
  description: string;
  icon: string;
  status: ImplementationStatus;
  required_capabilities: string[];
  parameter_schema: ParameterDescriptor[];
}
export interface GameDescriptor {
  game_id: string;
  name: string;
  platform_family: PlatformFamily;
  target_platform: string;
  summary: string;
  icon: string;
  status: ImplementationStatus;
  runtime_candidates: string[];
  features: FeatureDescriptor[];
}
export interface RuntimeDescriptor {
  runtime_id: string;
  name: string;
  description: string;
  status: ImplementationStatus;
}
export interface GameCatalog {
  schema_version: number;
  source: "mock";
  read_only: true;
  runtimes: RuntimeDescriptor[];
  games: GameDescriptor[];
}
export interface SessionSnapshot {
  state: "detached" | "attached_read_only";
  read_only: true;
  session_id: string | null;
  game_id: string | null;
  runtime_id: string | null;
  profile_id: string | null;
  message: string;
}
export const PLATFORMS: { id: "all" | PlatformFamily; label: string; subtitle: string }[] = [
  { id: "all", label: "全部游戏", subtitle: "All games" },
  { id: "gen3", label: "Gen 3", subtitle: "GBA" },
  { id: "gen4", label: "Gen 4", subtitle: "NDS" },
  { id: "gen5", label: "Gen 5", subtitle: "NDS" },
  { id: "gen6", label: "Gen 6", subtitle: "3DS" },
  { id: "pc", label: "PC Native", subtitle: "Windows" },
];
export const PLATFORM_LABEL: Record<PlatformFamily, string> = {
  gen3: "Gen 3 · GBA",
  gen4: "Gen 4 · NDS",
  gen5: "Gen 5 · NDS",
  gen6: "Gen 6 · 3DS",
  pc: "PC Native · Windows",
};
