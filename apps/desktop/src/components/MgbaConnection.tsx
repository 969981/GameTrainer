import { Activity, Cable, CircleOff, RefreshCw, ShieldCheck, Unplug } from "lucide-react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { getSessionSnapshot, isTauriDesktop } from "@/lib/backend";
import { useWorkspace } from "@/store/workspace";
import {
  connectEmeraldReadOnly, connectRsReadOnly, disconnectEmerald, getMgbaProcesses, readEmeraldRng, readRsRng, listRsProfiles,
} from "@/lib/mgba";

function hex(value: number, pad = 8) {
  return "0x" + value.toString(16).toUpperCase().padStart(pad, "0");
}

export function MgbaConnection() {
  const isDesktop = isTauriDesktop();
  const selectGame = useWorkspace((state) => state.selectGame);
  const client = useQueryClient();
  const session = useQuery({
    queryKey: ["session", "m0"],
    queryFn: getSessionSnapshot,
    enabled: isDesktop,
    refetchInterval: 2000,
  });
  const processes = useQuery({
    queryKey: ["m1", "mgba-processes"],
    queryFn: getMgbaProcesses,
    enabled: isDesktop,
    refetchInterval: 4000,
    retry: false,
  });
  const connect = useMutation({
    mutationFn: ({ processId, gameId }: { processId: number; gameId: string }) =>
      gameId === "games.pokemon.emerald"
        ? connectEmeraldReadOnly(processId)
        : connectRsReadOnly(processId, gameId as "games.pokemon.ruby" | "games.pokemon.sapphire"),
    onSuccess: (receipt) => {
      // Select the same game before session polling, so the cross-game session
      // isolation guard cannot mistake this for a stale previous game.
      selectGame(receipt.game_id);
      void client.invalidateQueries({ queryKey: ["session"] });
    },
  });
  const disconnect = useMutation({
    mutationFn: disconnectEmerald,
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ["session"] });
      client.removeQueries({ queryKey: ["m1", "rng"] });
    },
  });
  const connected = session.data?.state === "attached_read_only";

  return <Card>
    <CardHeader><div className="section-row"><h3>mGBA · Gen3 只读连接</h3><Badge>RUBY / SAPPHIRE / EMERALD</Badge></div></CardHeader>
    <CardContent>
      {!isDesktop
        ? <div className="preview-placeholder"><CircleOff size={18} /><div><strong>浏览器 Mock</strong><p>请运行 Windows Tauri 桌面版来扫描本地 mGBA。这里不会模拟真实进程。</p></div></div>
        : <>
          {connected ? <div className="connected-line">
            <ShieldCheck size={18} /> 已连接：{session.data?.profile_id} · 部分 ROM 锚点 · 只读
            <Button variant="outline" size="sm" disabled={disconnect.isPending} onClick={() => disconnect.mutate()}><Unplug size={14} />断开</Button>
          </div> : <>
            <p className="muted-copy">仅扫描 mGBA 可读进程；不会申请写内存权限。只能由用户明确选择目标连接。</p>
            <Button variant="outline" size="sm" onClick={() => void processes.refetch()} disabled={processes.isFetching}>
              <RefreshCw size={14} /> 刷新进程
            </Button>
            {processes.isPending && <p className="muted-copy">正在检测…</p>}
            {processes.isError && <p role="alert" className="error-text">进程检测失败：{String(processes.error)}</p>}
            {processes.isSuccess && processes.data.length === 0 && <p className="muted-copy">没有检测到可读取的 mGBA 进程。启动模拟器后再刷新。</p>}
            <div className="runtime-list">
              {(processes.data ?? []).map((process) => <div className="process-row" key={process.process_id}>
                <span>{process.executable_name}</span><code>PID {process.process_id}</code>
                  {[
                  ["games.pokemon.emerald", "绿宝石"],
                  ["games.pokemon.ruby", "红宝石"],
                  ["games.pokemon.sapphire", "蓝宝石"],
                ].map(([gameId, label]) => <Button key={gameId} size="sm" disabled={connect.isPending}
                  onClick={() => connect.mutate({ processId: process.process_id, gameId })}>
                  <Cable size={14} /> 连接{label}
                </Button>)}
              </div>)}
            </div>
          </>}
          {connect.isError && <p role="alert" className="error-text">连接失败：{String(connect.error)}</p>}
          {disconnect.isError && <p role="alert" className="error-text">断开失败：{String(disconnect.error)}</p>}
        </>
      }
      <p className="muted-copy">Emerald 仍仅 Rev0/部分指令锚点。Ruby/Sapphire 可识别 26 组零售 Header/Revision 和对应的 16 字节 Anchor；并非全 ROM 校验。所有游戏均禁止写入，有歧义时拒绝连接。</p>
    </CardContent>
  </Card>;
}

export function EmeraldRngLivePanel() {
  const desktop = isTauriDesktop();
  const client = useQueryClient();
  const session = useQuery({
    queryKey: ["session", "m0"],
    queryFn: getSessionSnapshot,
    enabled: desktop,
    refetchInterval: 1500,
  });
  const connected = desktop &&
    session.data?.state === "attached_read_only" &&
    session.data?.game_id === "games.pokemon.emerald";
  const rng = useQuery({
    queryKey: ["m1", "rng", session.data?.session_id],
    queryFn: readEmeraldRng,
    enabled: connected,
    refetchInterval: connected ? 1000 : false,
    retry: false,
  });
  if (!connected) return <Card><CardHeader><h3>Current Seed / RNG</h3></CardHeader><CardContent>
    <p className="muted-copy">尚未连接经过身份验证的 Emerald 只读会话。请到“运行时”页面选择本地 mGBA 进程。</p>
  </CardContent></Card>;
  return <Card><CardHeader><div className="section-row"><h3>RNG 实时只读观察</h3><Badge><Activity size={12}/> SESSION READ ONLY</Badge></div></CardHeader>
    <CardContent>
      {rng.isPending && <p className="muted-copy">读取当前 Seed…</p>}
      {rng.isError && <div role="alert" className="error-text">
        读取失败或会话已失效：{String(rng.error)}
        <Button variant="outline" size="sm" onClick={() => void client.invalidateQueries({ queryKey: ["session"] })}>刷新会话</Button>
      </div>}
      {rng.data && !rng.isError && <>
        <div className="stats-grid">
          <div className="rng-metric"><span>Current Seed</span><strong>{hex(rng.data.current_seed)}</strong></div>
          <div className="rng-metric"><span>Initial Seed</span><strong>{hex(rng.data.initial_seed,4)}</strong></div>
          <div className="rng-metric"><span>Advances</span><strong>{rng.data.advances.toLocaleString()}</strong></div>
        </div>
        <p className="muted-copy">{rng.data.game_code} · {rng.data.language} · {rng.data.verification} · Epoch {rng.data.epoch}</p>
      </>}
    </CardContent></Card>;
}


export function RubySapphireLivePanel({ gameId }: { gameId: "games.pokemon.ruby" | "games.pokemon.sapphire" }) {
  const desktop = isTauriDesktop();
  const session = useQuery({
    queryKey: ["session", "m0"], queryFn: getSessionSnapshot,
    enabled: desktop, refetchInterval: 1600, retry: false,
  });
  const connected = desktop && session.data?.state === "attached_read_only" &&
    session.data?.game_id === gameId;
  const rng = useQuery({
    queryKey: ["rs", "seed", session.data?.session_id],
    queryFn: readRsRng,
    enabled: connected,
    refetchInterval: connected ? 1000 : false,
    retry: false,
  });
  if (!connected) return <Card><CardHeader><h3>Ruby/Sapphire · Seed / Timer</h3></CardHeader><CardContent>
    <p className="muted-copy">当前未连接对应的 Ruby/Sapphire 只读会话。到「运行时」选择 mGBA 进程及正确的游戏版本。</p>
    <p className="muted-copy">美版 Seed 0x03004818，日版 0x03004748，欧版 0x03004828；数值为来源代码的 Profile，不代表已经连接或完成实机校验。</p>
  </CardContent></Card>;
  return <Card><CardHeader><div className="section-row"><h3>Ruby/Sapphire RNG · 只读观察</h3><Badge>HEADER + ANCHOR</Badge></div></CardHeader>
    <CardContent>
      {rng.isPending && <p>正在读取 Seed/Timer…</p>}
      {rng.isError && <p role="alert" className="error-text">读取失败或会话失效：{String(rng.error)}</p>}
      {rng.data && !rng.isError && <><div className="stats-grid">
        <div className="rng-metric"><span>Current Seed</span><strong>{hex(rng.data.current_seed)}</strong></div>
        <div className="rng-metric"><span>Timer（16-bit）</span><strong>{hex(rng.data.timer, 4)}</strong></div>
        <div className="rng-metric"><span>ROM 版本</span><strong>{rng.data.game_code} R{rng.data.revision}</strong></div>
      </div><p className="muted-copy">{rng.data.language} · {rng.data.verification} · Epoch {rng.data.epoch} · 只读，不提供 ROM Patch</p></>}
    </CardContent>
  </Card>;
}

export function RubySapphireProfileCatalog() {
  const desktop = isTauriDesktop();
  const profiles = useQuery({
    queryKey: ["rs", "profiles"], queryFn: listRsProfiles,
    enabled: desktop, retry: false, staleTime: Infinity,
  });
  return <Card><CardHeader><div className="section-row"><h3>Ruby / Sapphire · 零售版本矩阵</h3><Badge>26 PROFILES</Badge></div></CardHeader>
    <CardContent>
      <p className="muted-copy">英文 Ruby/Sapphire Rev0–2；日、德、法、意、西文 Rev0–1。另有两个日版 Pokémon Box 兼容变体，但它们不增加独立 Header Revision。德版 Ruby Debug 及未知 ROM 拒绝零售 Profile。</p>
      {!desktop && <p className="muted-copy">浏览器不读取本地运行时。桌面版可从 Rust Registry 查询完整 26 项版本清单。</p>}
      {profiles.isError && <p className="error-text" role="alert">{String(profiles.error)}</p>}
      {profiles.data && <details><summary>查看 Rust Profile 详情（{profiles.data.length}）</summary>
        <div className="rs-profile-table-wrap"><table className="rs-profile-table">
          <thead><tr><th>GameCode</th><th>Rev</th><th>Language</th><th>Seed Guest Addr</th><th>权限</th></tr></thead>
          <tbody>{profiles.data.map((p) => <tr key={p.build_id}>
            <td><code>{p.game_code}</code></td><td>{p.revision}</td><td>{p.language}</td>
            <td><code>{hex(p.seed_address)}</code></td><td>只读</td>
          </tr>)}</tbody>
        </table></div>
      </details>}
    </CardContent>
  </Card>;
}
