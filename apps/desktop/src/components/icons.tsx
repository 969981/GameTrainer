import {
  Activity, Boxes, Cpu, FileCode, Gem, Layers, Map, Monitor, Puzzle,
  Scan, Shield, Sparkles, Sprout, Target, Users,
  type LucideIcon,
} from "lucide-react";

const ICONS: Record<string, LucideIcon> = {
  activity: Activity, boxes: Boxes, cpu: Cpu, "file-code": FileCode, gem: Gem,
  layers: Layers, map: Map, monitor: Monitor, scan: Scan,
  shield: Shield, sparkles: Sparkles, sprout: Sprout, target: Target, users: Users,
};
export function GameIcon({ name, size = 20 }: { name: string; size?: number }) {
  const Icon = ICONS[name] ?? Puzzle;
  return <Icon size={size} strokeWidth={1.8} aria-hidden="true" />;
}
