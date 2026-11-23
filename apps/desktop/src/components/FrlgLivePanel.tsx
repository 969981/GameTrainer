import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, ArrowDownToLine, CheckCircle2, FileCode2, LockKeyhole, Plug, RotateCcw, ShieldAlert, Unplug } from "lucide-react";
import { isTauriDesktop } from "@/lib/backend";
import {
  frlgProfiles, frlgStatus, frlgConnect, frlgPreview, frlgApply, frlgArmWild,
  frlgRestore, frlgCancel, frlgDisconnect, frlgHistory,
  frlgPause, frlgResume, frlgMailbox, type FrlgDiff, type FrlgReceipt, type FrlgMailbox,
} from "@/lib/frlg";
import { Button } from "@/components/ui/button";
import { Card, CardHeader, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

type FeatureName = "capture" | "wall" | "wild";

export function FrlgLivePanel() {
  const desktop = isTauriDesktop();
  const client = useQueryClient();
  const [portText, setPortText] = useState("2345");
  const [feature, setFeature] = useState<FeatureName>("capture");
  const [templateFile, setTemplateFile] = useState<File | null>(null);
  const [seed, setSeed] = useState("00000000");
  const [preview, setPreview] = useState<FrlgDiff | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [armConfirmed, setArmConfirmed] = useState(false);
  const [receipt, setReceipt] = useState<FrlgReceipt | null>(null);
  const [mail, setMail] = useState<FrlgMailbox | null>(null);

  const refresh = () => { void client.invalidateQueries({ queryKey: ["m4"] }); };
  const status = useQuery({
    queryKey: ["m4", "status"], queryFn: frlgStatus, enabled: desktop, retry: false, refetchInterval: 5000,
  });
  const profiles = useQuery({ queryKey: ["m4", "profiles"], queryFn: frlgProfiles, enabled: desktop, retry: false });
  const history = useQuery({
    queryKey: ["m4", "history"], queryFn: frlgHistory, enabled: desktop, retry: false, refetchInterval: 10000,
  });

  const connect = useMutation({ mutationFn: frlgConnect, onSuccess: () => { setReceipt(null); setPreview(null); refresh(); } });
  const makePreview = useMutation({
    mutationFn: async () => {
      if (feature !== "wild") return frlgPreview(feature);
      if (!templateFile) throw new Error("必须从用户的 v2.0.4 源包选择 wild_payload.bin（116B）。");
      return frlgPreview(feature, Array.from(new Uint8Array(await templateFile.arrayBuffer())));
    },
    onSuccess: result => { setPreview(result); setConfirmed(false); refresh(); },
  });
  const apply = useMutation({
    mutationFn: frlgApply,
    onSuccess: result => { setReceipt(result); setPreview(null); setConfirmed(false); refresh(); },
    onError: () => { setPreview(null); refresh(); },
  });
  const cancel = useMutation({ mutationFn: frlgCancel, onSuccess: () => { setPreview(null); refresh(); } });
  const restore = useMutation({
    mutationFn: frlgRestore,
    onSuccess: result => { setReceipt(result); setMail(null); refresh(); },
    onError: refresh,
  });
  const disconnect = useMutation({
    mutationFn: frlgDisconnect,
    onSuccess: () => { setPreview(null); setReceipt(null); setMail(null); refresh(); },
  });
  const arm = useMutation({
    mutationFn: frlgArmWild,
    onSuccess: result => { setMail(result); setArmConfirmed(false); refresh(); },
    onError: refresh,
  });
  const pause = useMutation({ mutationFn: frlgPause, onSuccess: refresh });
  const resume = useMutation({ mutationFn: frlgResume, onSuccess: refresh });
  const fetchMailbox = useMutation({ mutationFn: frlgMailbox, onSuccess: setMail });

  const current = status.data;
  const busy = [connect, makePreview, apply, cancel, restore, disconnect, arm, pause, resume, fetchMailbox]
    .some(m => m.isPending);
  const issue = [connect, makePreview, apply, cancel, restore, disconnect, arm, pause, resume, fetchMailbox]
    .map(m => m.error).find(e => e != null);
  const selectedSeed = seed.trim().replace(/^0x/i, "");
  const validSeed = /^[0-9a-fA-F]{8}$/.test(selectedSeed);
  const port = Number(portText);
  const validPort = Number.isInteger(port) && port >= 1024 && port <= 65535;
  const hasOwner = !!current?.active_feature;
  const isWild = current?.active_feature === "features.frlg.wild.seed";

  return <div className="page-stack">
    <div className="page-heading">
      <div><div className="eyebrow">M4 / FIRERED · LEAFGREEN</div><h2>火红／叶绿 · 真实写入实验室</h2>
        <p>基于 v2.0.4 源码 16 个零售版 Profile，使用独立的 mGBA GDB Guest 内存通道。</p></div>
      <Badge>EXPERIMENTAL · OFFLINE</Badge>
    </div>
    <div className="m2-safety-banner"><ShieldAlert size={18}/><div>
      <strong>真实 ROM Guest 内存操作：仅用于离线、可丢弃存档</strong>
      <p>请先备份 .sav 与 SaveState。每次预览和执行前重新校验完整 16MiB ROM SHA1；当前不支持汉化、随机版或联网。GDB/ROM 实机安全性仍需测试，Hook 的停用与 SaveState 切换尤其有风险。</p>
    </div></div>
    {!desktop && <div className="m2-safety-banner"><LockKeyhole size={17}/><div>
      <strong>网页 Mock 禁止访问真实游戏</strong><p>必须启动 Windows Tauri 桌面版及 mGBA 的本机 GDB Server。</p>
    </div></div>}
    {issue && <p role="alert" className="error-text">{String(issue)}</p>}
    <div className="m2-layout">
      <Card><CardHeader><h3>01 · 连接 & 16 ROM Profile</h3></CardHeader><CardContent>
        <p className="muted-copy">仅允许本机 mGBA GDB；连接时 Guest CPU 必须停止。完整 ROM SHA1 与 16 个零售版本逐一对照，不会扫描进程内存或允许任意地址写入。</p>
        <div className="m3-identity"><div><span>源包 Profile</span><strong>{profiles.data?.length ?? 16} 个 · 英/日/德/法/意/西</strong></div>
          <div><span>GDB 本机端口</span><input aria-label="FRLG GDB 端口" type="number" min={1024} max={65535}
            value={portText} disabled={!!current || busy} onChange={e => setPortText(e.target.value)}/></div></div>
        <div className="m2-actions"><Button disabled={!desktop || !validPort || !!current || busy}
          onClick={() => connect.mutate(port)}><Plug size={15}/> {connect.isPending ? "完整 ROM SHA1 核对中…" : "连接火红／叶绿"}</Button>
          {current && <Button disabled={busy || hasOwner || !!preview} variant="outline"
            onClick={() => disconnect.mutate()}><Unplug size={15}/> 安全断开</Button>}</div>
        {current && <div className="m3-identity">
          <div><span>版本</span><strong>{current.profile_id} · {current.language}</strong></div>
          <div><span>完整 ROM SHA1</span><code>{current.rom_sha1}</code></div>
          <div><span>Game</span><code>{current.game_id}</code></div>
          <div><span>Guest CPU</span><Badge>{current.guest_paused ? "PAUSED" : "RUNNING"}</Badge></div>
          <div><span>已安装 Owner</span><code>{current.active_feature ?? "无"}</code></div>
        </div>}
      </CardContent></Card>
      <Card><CardHeader><h3>02 · 固定补丁 & 一次性 Seed Hook</h3></CardHeader><CardContent>
        <div className="m3-select"><label htmlFor="frlg-feature">功能</label>
          <select id="frlg-feature" value={feature} disabled={!current || busy || !!preview || hasOwner}
            onChange={e => setFeature(e.target.value as FeatureName)}>
            <option value="capture">100% 捕捉 · 单 halfword</option>
            <option value="wall">穿墙 · 单 halfword</option>
            <option value="wild">普通野生 Seed 注入 · v2.0.4 实验 Hook</option>
          </select>
        </div>
        {feature === "wild" && <div className="m3-payload">
          <label htmlFor="frlg-template">上传本地 wild_payload.bin（SHA256 严格核对，不通过网络）</label>
          <input id="frlg-template" type="file" accept=".bin" disabled={!current || busy || !!preview || hasOwner}
            onChange={e => setTemplateFile(e.target.files?.[0] ?? null)}/>
          <small>仅接收你提供的 v2.0.4 源包 116B Payload；沿用其 flags==1 普通野生判断和单次 flags→0 防 Repel 逻辑。甜甜香气是另一条 Hook，本版没有一并开放。</small>
        </div>}
        <Button className="w-full" disabled={!desktop || !current || !current.guest_paused || busy || !!preview || hasOwner || (feature === "wild" && !templateFile)}
          onClick={() => makePreview.mutate()}><FileCode2 size={15}/> 仅预览 Guest 指令差异</Button>
        {preview && <div className="m3-preview">
          <strong>差异区域：{preview.spans.length} · 30 秒一次性 Token</strong>
          {preview.spans.map(span => <div key={span.address} className="m3-span">
            <strong>{span.label}</strong><code>{span.address}</code>
            <small>原值：{span.before.length > 60 ? span.before.slice(0,60) + "…" : span.before}</small>
            <small>新值：{span.after.length > 60 ? span.after.slice(0,60) + "…" : span.after}</small>
          </div>)}
          <label className="m2-confirm"><input type="checkbox" checked={confirmed} onChange={e => setConfirmed(e.target.checked)}/>
            我已备份存档并确认仅供离线单机实验，接受游戏崩溃与旧 Hook 风险
          </label>
          <div className="m2-actions"><Button variant="outline" disabled={busy} onClick={() => cancel.mutate()}>放弃预览</Button>
            <Button disabled={busy || !confirmed} onClick={() => apply.mutate(preview.token)}>
              <AlertTriangle size={15}/> 确认真实写入</Button></div>
        </div>}
        {hasOwner && <div className="m3-active"><Badge>{current?.active_state?.toUpperCase() ?? "ACTIVE"}</Badge>
          <p>真正修改的 Guest 字节已由 SQLite 记录 Owner。运行中恢复会先暂停，确认仍是原版 ROM 和本工具的字节，再安全撤销。穿墙建议在正常地面关闭；野生 Hook 必须在非战斗状态卸载。</p>
          {isWild && <div className="m3-payload">
            <label htmlFor="frlg-seed">目标 Seed（8 位十六进制）</label>
            <input id="frlg-seed" value={seed} maxLength={10} disabled={busy}
              onChange={e => setSeed(e.target.value)} placeholder="00000000"/>
            <label className="m2-confirm"><input type="checkbox" checked={armConfirmed} onChange={e => setArmConfirmed(e.target.checked)}/>
              再次确认：仅对备份后的离线单机进度注入一次野生 Seed
            </label>
            <Button disabled={busy || !validSeed || !armConfirmed} onClick={() => arm.mutate(Number.parseInt(selectedSeed,16))}>
              <ArrowDownToLine size={15}/> 让下一次普通野生遭遇使用此 Seed
            </Button>
            <small>单次模式。Hook 只接受 flags==1 的普通步行/冲浪等路径；命中后会自动清除待命并绕过本次 Repel。需要重新待命才能再次触发。</small>
            <div className="m2-actions"><Button variant="outline" disabled={busy || !!current?.guest_paused} onClick={() => pause.mutate()}>暂停读取邮箱</Button>
              <Button variant="outline" disabled={busy || !current?.guest_paused} onClick={() => fetchMailbox.mutate()}>读取命中数</Button>
              <Button variant="outline" disabled={busy || !current?.guest_paused} onClick={() => resume.mutate()}>继续运行</Button></div>
            {mail && <div className="m3-identity">
              <div><span>命中次数</span><strong>{mail.hit_count}</strong></div>
              <div><span>上次触发</span><code>{mail.last_trigger ? "YES" : "NO"}</code></div>
              <div><span>是否仍待命</span><code>{mail.mode === 1 && mail.active_hook === 1 ? "ARMED" : "OFF"}</code></div>
              <div><span>上个 Seed</span><code>0x{mail.last_old_seed.toString(16).padStart(8,"0")}</code></div>
            </div>}
          </div>}
          <Button className="w-full" variant="outline" disabled={busy} onClick={() => restore.mutate()}>
            <RotateCcw size={15}/> 暂停并验证恢复原指令
          </Button>
        </div>}
        {receipt && <div className="m2-receipt" role="status"><CheckCircle2 size={16}/><div>
          <strong>Rust 真实操作回执：{receipt.status}</strong><p>{receipt.feature_id} · {receipt.owner_id}</p></div></div>}
      </CardContent></Card>
    </div>
    <Card><CardHeader><h3>03 · FRLG 真实 SQLite 操作历史</h3></CardHeader><CardContent>
      <p className="muted-copy">与红蓝宝石 M3 和纯内存 M2 Fixture 完全隔离。若 SaveState 携带旧跳板，请回到干净的备份，不要跨 Session 强制执行恢复。</p>
      <div className="m3-event-list">{(history.data ?? []).slice(0,8).map(row =>
        <div key={row.event_id}><strong>{row.action_kind}</strong><span>{row.feature_id} · {row.status}</span>
          <small>{new Date(row.occurred_at_ms).toLocaleString("zh-CN")}</small></div>)}</div>
    </CardContent></Card>
  </div>;
}
