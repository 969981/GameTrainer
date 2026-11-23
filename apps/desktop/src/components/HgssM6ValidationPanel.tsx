import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Activity, AlertTriangle, Copy, LockKeyhole, ShieldCheck, Search } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { isTauriDesktop } from "@/lib/backend";
import { hgssProfiles } from "@/lib/hgss";
import { m6Transition, probeArm9Stop, type M6OverlayReport } from "@/lib/hgss-m6";

const options = hgssProfiles.filter(p => p.static_audit_1451);
const toHex = (n: number | null, pad = 8) => n === null ? "—" :
  "0x" + n.toString(16).toUpperCase().padStart(pad, "0");
const desc: Record<M6OverlayReport["state"], string> = {
  not_loaded: "未定位到运行中的 Overlay12",
  pending_execution: "Overlay12 管理器存在，但尚未执行",
  original_verified: "Overlay12 目标及 32 字节邻域与来源原指令一致",
  foreign_patched: "目标已是补丁指令，所有权未知，禁止接管",
  signature_mismatch: "目标指令邻域不符，拒绝继续",
  unexpected_instruction: "目标是未知指令，拒绝继续",
};

export function HgssM6ValidationPanel() {
  const desktop = isTauriDesktop();
  const [port, setPort] = useState("3333");
  const [selected, setSelected] = useState("IPKJ:0");
  const [samples, setSamples] = useState<M6OverlayReport[]>([]);
  const [copied, setCopied] = useState(false);
  const [copyError, setCopyError] = useState<string | null>(null);
  const profile = options.find(p => p.game_code + ":" + p.revision === selected);
  const probe = useMutation({
    mutationFn: async () => {
      if (!profile) throw Error("请选择已通过静态审计的 ROM 版本");
      return probeArm9Stop(Number(port), profile.game_code, profile.revision);
    },
    onSuccess: report => {
      setSamples(prev => [...prev.filter(x =>
        x.game_code === report.game_code && x.revision === report.revision
      ), report].slice(-8));
      setCopied(false);
    },
  });
  const last = samples.at(-1);
  const previous = samples.at(-2);
  const transition = last && previous ? m6Transition(previous, last) : null;
  async function copy() {
    try {
      await navigator.clipboard.writeText(JSON.stringify({
        protocol: "M6_ARM9_DEBUG_EVIDENCE_V1",
        samples, 
        note: "仅 GDB 停止和指令邻域证据，不代表正在运行 ROM 的 SHA1 验证，不构成写入授权",
      }, null, 2));
      setCopied(true); setCopyError(null);
    } catch (e) { setCopyError(String(e)); }
  }
  return <Card>
    <CardHeader><div className="section-row">
      <h3>HGSS M6 · ARM9 停机 / Overlay12 验证</h3>
      <Badge>READ-ONLY PROBE</Badge>
    </div></CardHeader>
    <CardContent>
      <div className="m2-safety-banner"><LockKeyhole size={19}/><div>
        <strong>只读取客体寄存器与固定源证据地址，不做任何 Guest 写入</strong>
        <p>需要带 ARM9 GDB Stub 的 DeSmuME 开发构建。普通 Windows 发布版未必提供该接口。手动在 ARM9 调试器暂停游戏后执行采样；采样结束由调试器继续运行。不可把 M5 进程读取视为 ARM9 已暂停。</p>
      </div></div>
      <div className="m2-actions">
        <label className="parameter-field"><span>ROM 版本（仅用于选择静态证据）</span>
          <select aria-label="M6 ROM Profile" value={selected} disabled={probe.isPending}
            onChange={e => {setSelected(e.target.value);setSamples([]);}}>
            {options.map(p => <option key={p.game_code + p.revision}
              value={p.game_code + ":" + p.revision}>
              {p.game} · {p.language} · {p.game_code} Rev{p.revision}
            </option>)}
          </select>
        </label>
        <label className="parameter-field"><span>本机 ARM9 GDB 端口</span>
          <input aria-label="M6 ARM9 GDB Port" value={port}
            onChange={e => setPort(e.target.value)}
            disabled={probe.isPending} inputMode="numeric"/>
        </label>
      </div>
      <div className="m2-actions">
        <Button disabled={!desktop || probe.isPending || !profile ||
          !Number.isInteger(Number(port)) || Number(port) < 1024 || Number(port) > 65535}
          onClick={() => probe.mutate()}><Search size={15}/>
          {probe.isPending ? "读取 ARM9 停机证据…" : "采样暂停的 ARM9 / Overlay12"}
        </Button>
        <Button variant="outline" disabled={samples.length === 0} onClick={() => void copy()}>
          <Copy size={14}/>{copied ? "证据已复制" : "复制 JSON 证据"}
        </Button>
      </div>
      {!desktop && <p className="muted-copy">浏览器 Mock 禁止真实探测；Windows Tauri 连接到用户已启用的 ARM9 GDB 服务后才会执行。</p>}
      {probe.isError && <p className="error-text" role="alert">
        <AlertTriangle size={14}/> {String(probe.error)}。请确认是 ARM9 GDB 端口、CPU 已停止、选中原版 ROM 且 DeSmuME Stub 真正启用。
      </p>}
      {copyError && <p role="alert" className="error-text">{copyError}</p>}
      {last && <div className="page-stack">
        <div className="connected-line"><ShieldCheck size={16}/>
          ARM9 GDB 停止响应与 PC 复读检查已通过；**不是**运行中 ROM 身份验证。
        </div>
        <div className="stats-grid">
          <div className="rng-metric"><span>ARM9 停机 PC</span><strong>{toHex(last.arm9_pc)}</strong></div>
          <div className="rng-metric"><span>Overlay12 状态</span><strong>{last.state}</strong></div>
          <div className="rng-metric"><span>Overlay 管理器</span><strong>{toHex(last.overlay_child)}</strong></div>
          <div className="rng-metric"><span>捕捉原指令</span><strong>{toHex(last.observed_halfword, 4)}</strong></div>
        </div>
        <p className="muted-copy">{desc[last.state]}。目标地址 {toHex(last.capture_addr)}；Overlay12 预期加载地址 {toHex(last.overlay_load_addr)}。{transition ?? "这是第一份停机快照；可在手动继续/重新暂停后再次采样，比较进出战斗变化。"}</p>
        <div className="m2-safety-banner"><AlertTriangle size={18}/><div>
          <strong>写入授权：未获得</strong>
          <p>尚无运行中完整 ROM SHA1、双 CPU 统一停机、JIT/缓存可见性与真实卸载证明。任何状态都不会开放捕捉、穿墙或战斗 Hook 开关。</p>
        </div></div>
      </div>}
      <p className="muted-copy">建议采集：野外暂停 → 手动继续 → 进入普通单打暂停 → 手动继续 → 战斗结束暂停 → 重进战斗暂停。每一次采样完成后按 DeSmuME 调试器的继续键，不能期待本界面自动恢复 CPU。最多保留本页面最近 8 条只读采样记录。</p>
    </CardContent>
  </Card>;
}
