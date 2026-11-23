import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Activity, Cable, CircleOff, RefreshCw, ShieldCheck, Unplug } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { isTauriDesktop } from "@/lib/backend";
import {
  attachHgss, detachHgss, getDesmumeProcesses, hgssProfiles,
  readHgss, verifyLocalHgssRom,
  type HgssSnapshot, type LocalRomEvidence,
} from "@/lib/hgss";

const hex = (value: number) => "0x" + value.toString(16).toUpperCase().padStart(8, "0");
export function HgssPanel() {
  const desktop = isTauriDesktop();
  const [gameId, setGameId] = useState("games.pokemon.heartgold");
  const [session, setSession] = useState<HgssSnapshot | null>(null);
  const [evidence, setEvidence] = useState<LocalRomEvidence | null>(null);
  const [hashError, setHashError] = useState<string | null>(null);
  const [hashing, setHashing] = useState(false);
  const processes = useQuery({
    queryKey: ["m5", "desmume"], queryFn: getDesmumeProcesses,
    enabled: desktop, refetchInterval: 8000, retry: false,
  });
  const connect = useMutation({ mutationFn: (pid: number) => attachHgss(pid, gameId),
    onSuccess: result => setSession(result) });
  const disconnect = useMutation({ mutationFn: detachHgss,
    onSuccess: () => setSession(null) });
  const snapshot = useQuery({
    queryKey: ["m5", "hgss-rng", session?.session_id], queryFn: readHgss,
    enabled: desktop && !!session, refetchInterval: 1500, retry: false,
  });
  const current = snapshot.data ?? session;
  const busy = connect.isPending || disconnect.isPending;
  async function verifyFile(file: File | undefined) {
    setEvidence(null); setHashError(null);
    if (!file) return;
    setHashing(true);
    try { setEvidence(await verifyLocalHgssRom(file)); }
    catch (e) { setHashError(String(e)); }
    finally { setHashing(false); }
  }
  return <div className="page-stack">
    <Card>
      <CardHeader><div className="section-row"><h3>DeSmuME · HGSS 心金／魂银（M5）</h3>
        <Badge>READ ONLY / EXPERIMENTAL</Badge></div></CardHeader>
      <CardContent>
        <p className="muted-copy">这是真实 Windows 进程只读扫描实验，不是 mGBA GDB。不会申请写入权限，不提供捕捉、穿墙、传送、战斗 Hook 开关。尚未进行 DeSmuME 实机验收。</p>
        {!desktop && <div className="preview-placeholder"><CircleOff size={18}/><p>浏览器只显示静态版本库；实际连接需要 Windows Tauri 桌面应用。</p></div>}
        <div className="m2-actions">
          <Button variant={gameId === "games.pokemon.heartgold" ? "default" : "outline"}
            onClick={() => setGameId("games.pokemon.heartgold")} disabled={busy || !!session}>心金</Button>
          <Button variant={gameId === "games.pokemon.soulsilver" ? "default" : "outline"}
            onClick={() => setGameId("games.pokemon.soulsilver")} disabled={busy || !!session}>魂银</Button>
          {desktop && <Button variant="outline" disabled={processes.isFetching}
            onClick={() => void processes.refetch()}><RefreshCw size={14}/> 刷新进程</Button>}
        </div>
        {processes.isError && <p role="alert" className="error-text">{String(processes.error)}</p>}
        {!session && desktop && (processes.data?.length === 0) &&
          <p className="muted-copy">没有找到 DeSmuME 进程。先在 DeSmuME 启动游戏。</p>}
        {!session && processes.data?.map(p => <div key={p.process_id} className="process-row">
          <span>{p.executable_name}</span><code>PID {p.process_id}</code>
          <Button size="sm" disabled={busy}
            onClick={() => connect.mutate(p.process_id)}><Cable size={14}/> 只读连接</Button>
        </div>)}
        {connect.isError && <p role="alert" className="error-text">内存身份或映射校验失败：{String(connect.error)}</p>}
        {session && <div className="page-stack">
          <div className="connected-line"><ShieldCheck size={18}/>
            {session.game_code} Rev{session.revision} · {session.language} · 只读
            <Button variant="outline" size="sm" onClick={() => disconnect.mutate()}
              disabled={busy}><Unplug size={14}/>断开</Button>
          </div>
          {snapshot.isError && <p role="alert" className="error-text">
            会话可能失效：{String(snapshot.error)}。请断开并重新校验。</p>}
          {current && !snapshot.isError && <div className="stats-grid">
            <div className="rng-metric"><span>Current LCRNG Seed</span><strong>{hex(current.current_seed)}</strong></div>
            <div className="rng-metric"><span>MT State[0]（不是 Initial Seed）</span><strong>{hex(current.mt_state0)}</strong></div>
            <div className="rng-metric"><span>MT Index</span><strong>{current.mt_index}</strong></div>
            <div className="rng-metric"><span>Delay</span><strong>{current.delay}</strong></div>
            <div className="rng-metric"><span>Trainer ID / SID</span><strong>{current.trainer_id ?? "?"} / {current.secret_id ?? "?"}</strong></div>
          </div>}
          <p className="muted-copy">进程扫描仅核对 Code/Rev、MT Index、关键指针；没有计算模拟器内完整 ROM SHA1。也没有验证 DTCM 和 Battle Advances，因此不展示虚假的准确推进数。</p>
        </div>}
        <div className="section-row"><h3>本地 .nds 文件指纹</h3><Badge>OFFLINE SHA1</Badge></div>
        <label className="parameter-field"><span>选择自己持有的原版 ROM（仅本地计算，不上传）</span>
          <input type="file" accept=".nds" aria-label="选择 NDS 文件" disabled={hashing}
            onChange={e => void verifyFile(e.target.files?.[0])}/>
        </label>
        {hashing && <p className="muted-copy">正在本地计算整个 .nds 文件 SHA1…</p>}
        {hashError && <p role="alert" className="error-text">{hashError}</p>}
        {evidence && <div className="feature-metadata">
          <span>Game Code / Rev</span><code>{evidence.gameCode} Rev{evidence.revision}</code>
          <span>文件 SHA1</span><code>{evidence.sha1}</code>
          <span>校验结果</span><strong>{evidence.matched ? "与登记原版匹配" : evidence.error}</strong>
        </div>}
        <p className="muted-copy">本地文件校验与模拟器中的 ROM 身份是两回事；SHA1 匹配也不会解除 M5 的只读限制。</p>
      </CardContent>
    </Card>
    <Card><CardHeader><h3>v1.4.5.1 · 版本及实验来源</h3></CardHeader><CardContent>
      <p className="muted-copy">14 个 Profile：12 个日/欧版有本版 ROM 静态证据；美版 Rev0 是旧版资料，未列入本次战斗调用链审计。韩版和其它 Revision 不支持。</p>
      <div className="runtime-list">{hgssProfiles.filter(p => p.game_id === gameId).map(p =>
        <div className="process-row" key={p.game_code + ":" + p.revision}>
          <span>{p.language}</span><code>{p.game_code} Rev{p.revision}</code>
          <Badge>{p.static_audit_1451 ? "12-ROM STATIC" : "LEGACY PROFILE"}</Badge>
        </div>)}</div>
      <div className="preview-placeholder"><Activity size={18}/><div><strong>未开放的战斗与剧情修改</strong>
        <p>源包中的战斗 Callflow 是静态断点证据 + CE 定时采样，不是已验证的条件式 Hook；M5 未接入任何写入。</p>
      </div></div>
    </CardContent></Card>
  </div>;
}
