import { Activity, Cable, CircleOff, RefreshCw, ShieldCheck, Unplug } from "lucide-react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { getSessionSnapshot, isTauriDesktop } from "@/lib/backend";
import {
  connectEmeraldReadOnly, disconnectEmerald, getMgbaProcesses, readEmeraldRng,
} from "@/lib/mgba";

function hex(value: number, pad = 8) {
  return "0x" + value.toString(16).toUpperCase().padStart(pad, "0");
}

export function MgbaConnection() {
  const isDesktop = isTauriDesktop();
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
    mutationFn: connectEmeraldReadOnly,
    onSuccess: () => { void client.invalidateQueries({ queryKey: ["session"] }); },
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
    <CardHeader><div className="section-row"><h3>mGBA · Emerald 只读连接</h3><Badge>M1 / READ ONLY</Badge></div></CardHeader>
    <CardContent>
      {!isDesktop
        ? <div className="preview-placeholder"><CircleOff size={18} /><div><strong>浏览器 Mock</strong><p>请运行 Windows Tauri 桌面版来扫描本地 mGBA。这里不会模拟真实进程。</p></div></div>
        : <>
          {connected ? <div className="connected-line">
            <ShieldCheck size={18} /> 已连接：{session.data?.profile_id} · 部分锚点已验证 · 只读
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
                <Button size="sm" disabled={connect.isPending} onClick={() => connect.mutate(process.process_id)}>
                  <Cable size={14} /> 只读连接
                </Button>
              </div>)}
            </div>
          </>}
          {connect.isError && <p role="alert" className="error-text">连接失败：{String(connect.error)}</p>}
          {disconnect.isError && <p role="alert" className="error-text">断开失败：{String(disconnect.error)}</p>}
        </>
      }
      <p className="muted-copy">本阶段只验证 Emerald Rev0 六语言的部分 ROM 原版指令锚点；不能据此修改游戏。候选内存映射有歧义则拒绝附加。</p>
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
