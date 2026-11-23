import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Activity, AlertTriangle, ArrowDown, CheckCircle2, ClipboardList, Database,
  Lock, RotateCcw, ShieldAlert, ShieldCheck, TestTube2, X,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { isTauriDesktop } from "@/lib/backend";
import {
  applyFixturePreview, cancelFixturePreview, getFixtureStatus, hexSeed, listActionHistory,
  parseHexSeed, previewFixtureSeed, restoreFixturePatch, startFixture,
  type ActionEvent, type AuditEnvironment, type FixtureGameId, type PatchPreview, type PatchReceipt,
} from "@/lib/patch";

const FIXTURE_GAMES: { id: FixtureGameId; label: string }[] = [
  { id: "games.pokemon.emerald", label: "Pokémon Emerald（绿宝石）" },
  { id: "games.pokemon.ruby", label: "Pokémon Ruby（红宝石）" },
  { id: "games.pokemon.sapphire", label: "Pokémon Sapphire（蓝宝石）" },
];
const STEPS = ["选择隔离会话", "预览修改", "明确确认", "检查与恢复"];
const STATUS_NAME: Record<string, string> = {
  preview: "预览", apply_intent: "写入意图", applied: "应用成功", apply_rejected: "写入拒绝",
  restore_intent: "恢复意图", restored: "恢复成功", restore_failed: "恢复失败",
  restore_conflict: "恢复冲突", rollback_confirmed: "回滚确认",
  preview_cancelled: "取消预览",
};

function formatTime(ms: number) {
  return new Date(ms).toLocaleString("zh-CN", { hour12: false });
}

export function PatchLabPage() {
  const desktop = isTauriDesktop();
  const queryClient = useQueryClient();
  const [input, setInput] = useState("0xF00DBABE");
  const [gameId, setGameId] = useState<FixtureGameId>("games.pokemon.emerald");
  const [preview, setPreview] = useState<PatchPreview | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [lastReceipt, setLastReceipt] = useState<PatchReceipt | null>(null);
  const seed = parseHexSeed(input);
  const status = useQuery({
    queryKey: ["m2", "status"],
    queryFn: getFixtureStatus,
    enabled: desktop,
    refetchInterval: 2000,
    retry: false,
  });
  function refresh() {
    void queryClient.invalidateQueries({ queryKey: ["m2"] });
  }
  const start = useMutation({
    mutationFn: startFixture,
    onSuccess: () => { setPreview(null); setConfirmed(false); setLastReceipt(null); refresh(); },
  });
  const makePreview = useMutation({
    mutationFn: previewFixtureSeed,
    onSuccess: (data) => { setPreview(data); setConfirmed(false); refresh(); },
  });
  const cancel = useMutation({
    mutationFn: cancelFixturePreview,
    onSuccess: () => { setPreview(null); setConfirmed(false); refresh(); },
  });
  const apply = useMutation({
    mutationFn: applyFixturePreview,
    onSuccess: (receipt) => { setLastReceipt(receipt); setPreview(null); setConfirmed(false); refresh(); },
    onError: () => { setPreview(null); setConfirmed(false); refresh(); },
  });
  const restore = useMutation({
    mutationFn: restoreFixturePatch,
    onSuccess: (receipt) => { setLastReceipt(receipt); refresh(); },
    onError: refresh,
  });
  const active = status.data?.fixture?.active_patches ?? [];
  const blocked = !!active.length;
  const busy = start.isPending || makePreview.isPending || cancel.isPending || apply.isPending || restore.isPending;
  const mutationError = [start.error, makePreview.error, cancel.error, apply.error, restore.error]
    .find((error) => error !== null);
  const stage = !status.data?.connected ? 0 : preview ? 2 : blocked ? 3 : 1;
  return <div className="page-stack">
    <div className="page-heading"><div><div className="eyebrow">M2 / VERIFIED PATCH PIPELINE</div><h1>安全补丁实验室</h1>
      <p>先在 Rust 内存测试目标完成完整的预览 → 应用 → 恢复流程。</p>
    </div><Badge><TestTube2 size={13} /> FIXTURE ONLY</Badge></div>
    <div className="m2-safety-banner"><Lock size={19} /><div><strong>不会写入真实游戏</strong>
      <p>本页面只能修改桌面应用内部的一枚合成 Seed。mGBA 仍然只读；未知 ROM、未验证的游戏版本和任意地址写入均保持禁止。</p>
    </div></div>
    <div className="m2-steps" aria-label="补丁流程">
      {STEPS.map((label, index) => <div className={"m2-step" + (index === stage ? " current" : index < stage ? " done" : "")}
        key={label}><span>{index < stage ? "✓" : index + 1}</span><small>{label}</small></div>)}
    </div>
    {!desktop && <Card><CardContent className="m2-desktop-notice"><ShieldAlert size={20} /><div>
      <strong>浏览器仅能预览 M2 设计</strong><p>真实 SQLite 和事务由 Tauri Rust 命令执行。请使用 Windows 桌面版进行测试，浏览器不会伪造应用成功、恢复成功或审计记录。</p>
    </div></CardContent></Card>}
    {status.isError && <p className="error-text" role="alert">SQLite 审计初始化失败，写入操作已关闭：{String(status.error)}</p>}
    {mutationError && <p className="error-text" role="alert"><AlertTriangle size={15} className="inline-block" /> {String(mutationError)}</p>}
    <div className="m2-layout">
      <Card>
        <CardHeader><h3>01 · 隔离会话与输入</h3></CardHeader>
        <CardContent>
          <div className="m2-session">
            <span className="feature-icon"><TestTube2 size={22} /></span>
            <div><strong>{FIXTURE_GAMES.find((game) => game.id === status.data?.fixture?.game_id)?.label ?? "Gen3"} · Rust Fixture</strong>
              <small>{status.data?.connected ? "已连接隔离内存" : "未启动 · 无真实游戏进程"} · 固定 4 字节种子</small>
            </div>
            <Badge>{status.data?.connected ? "ISOLATED" : "DETACHED"}</Badge>
          </div>
          <label className="parameter-field m2-seed-entry"><span>选择隔离测试游戏</span>
            <select aria-label="选择隔离测试游戏" className="m2-fixture-select" value={gameId}
              disabled={busy || blocked || !!preview}
              onChange={(event) => setGameId(event.target.value as FixtureGameId)}>
              {FIXTURE_GAMES.map((game) => <option key={game.id} value={game.id}>{game.label}</option>)}
            </select>
            <small>每个游戏单独生成 Session ID、Profile、审计事件；红蓝宝石使用美版 RS Seed 布局模拟值。</small>
          </label>
          <Button variant="outline" className="w-full" disabled={!desktop || busy || blocked || !!preview}
            onClick={() => start.mutate(gameId)}><Activity size={15} />
            {status.data?.connected ? "重置隔离会话" : "开始隔离测试"}
          </Button>
          <label className="parameter-field m2-seed-entry"><span>目标 Seed（十六进制 u32）</span>
            <input aria-label="M2 目标 Seed" value={input} disabled={!desktop || !status.data?.connected || !!preview || busy || blocked}
              placeholder="0xF00DBABE" onChange={(event) => setInput(event.target.value)} />
            <small>仅允许 1–8 位十六进制字符，可使用 0x 前缀。地址由可信 Rust 模块固定指定。</small>
          </label>
          {seed === null && <p className="error-text" role="alert">请输入合法的 32 位十六进制 Seed。</p>}
          <div className="m2-actions"><Button className="w-full" disabled={!desktop || !status.data?.connected || seed === null || !!preview || blocked || busy}
            onClick={() => { if (seed !== null) makePreview.mutate(seed); }}><ShieldCheck size={15} /> 生成后端预览</Button></div>
          {status.data?.connected && <div className="m2-current">
            <span>当前测试内存值 · {status.data.fixture?.seed_address_display}</span><code>{hexSeed(status.data.fixture?.seed ?? 0)}</code>
          </div>}
          {(status.data?.unresolved_fixture_intents ?? 0) > 0 && <div className="m2-recovery-note">
            <AlertTriangle size={16} /><span>SQLite 发现 {status.data?.unresolved_fixture_intents} 条未闭合的测试写入意图。重启后只展示证据，不自动恢复旧内存。</span>
          </div>}
        </CardContent>
      </Card>
      <Card>
        <CardHeader><h3>02 · 预览 / 所有权 / 恢复</h3></CardHeader>
        <CardContent>
          {preview ? <>
            <div className="m2-diff" role="region" aria-label="Seed 修改差异">
              <div><span>原始数据</span><code>{preview.before_display}</code></div>
              <ArrowDown size={18} />
              <div><span>拟写入值</span><code>{preview.after_display}</code></div>
            </div>
            <div className="m2-metadata"><span>Guest 地址</span><code>{preview.address_display}</code>
              <span>会话 Epoch</span><code>{preview.epoch}</code>
              <span>有效期</span><code>{preview.expires_in_seconds}s（一次性）</code>
              <span>Profile</span><code>{preview.profile_id}</code>
            </div>
            <label className="m2-confirm"><input type="checkbox" aria-label="确认模拟写入" checked={confirmed}
              onChange={(event) => setConfirmed(event.target.checked)} />
              我已核对原值与目标值，确认这次只修改隔离测试内存
            </label>
            <div className="m2-actions">
              <Button variant="outline" disabled={busy} onClick={() => cancel.mutate(preview.preview_id)}><X size={15} /> 放弃预览</Button>
              <Button disabled={!confirmed || busy} onClick={() => apply.mutate(preview.preview_id)}><CheckCircle2 size={15} /> 确认应用</Button>
            </div>
          </> : active.length > 0 ? active.map((patch) => <div key={patch.owner_id} className="m2-active">
            <div className="m2-active-heading"><Badge>{patch.status.toUpperCase()}</Badge><strong>受控补丁已登记</strong></div>
            <code title={patch.owner_id}>{patch.owner_id}</code>
            <p>只有当前数据仍与该补丁写入值相同，且会话身份没有变化时才允许恢复。</p>
            <Button variant="outline" className="w-full" disabled={busy} onClick={() => restore.mutate(patch.owner_id)}>
              <RotateCcw size={15} /> 验证并恢复原始值
            </Button>
          </div>) : <div className="m2-empty">
            <ShieldCheck size={29}/><strong>尚无待处理的修改</strong>
            <p>提交新的 Seed 预览后，系统会在此展示具体字节变化、过期时间和需要确认的操作。</p>
          </div>}
          {lastReceipt && <div className="m2-receipt" role="status">
            <CheckCircle2 size={16} /><div><strong>后端回执：{lastReceipt.status}</strong>
              <p>Owner: <code>{lastReceipt.owner_id}</code></p><small>SQLite 审计：{lastReceipt.audited ? "已确认" : "未确认"}</small>
            </div>
          </div>}
        </CardContent>
      </Card>
    </div>
    <Card><CardHeader><div className="section-row"><h3><Database size={16} className="inline-block" /> SQLite 审计说明</h3><Badge>APP LOCAL DATA</Badge></div></CardHeader>
      <CardContent><p className="muted-copy">预览、写入意图、应用回执、冲突、恢复意图与恢复结果由 Rust 写入本地 SQLite，并与真实游戏操作历史分离。日志只保留白名单摘要，不存储整段内存或游戏 ROM。意外退出后不会尝试恢复先前会话的补丁。</p></CardContent>
    </Card>
  </div>;
}
export function AuditHistoryPage() {
  const desktop = isTauriDesktop();
  const [environment, setEnvironment] = useState<AuditEnvironment>("live");
  const history = useQuery({
    queryKey: ["m2", "history", environment],
    queryFn: () => listActionHistory(environment),
    enabled: desktop,
    refetchInterval: 3000,
    retry: false,
  });
  return <div className="page-stack">
    <div className="page-heading"><div><div className="eyebrow">AUDIT / SQLITE</div><h1>操作历史</h1>
      <p>本地持久化审计，模拟实验与真实游戏动作必须分开查看。</p>
    </div><Badge><ClipboardList size={14} /> READ ONLY</Badge></div>
    <div className="m2-history-tabs" role="group" aria-label="历史记录范围">
      <button className={environment === "live" ? "active" : ""} aria-pressed={environment === "live"}
        onClick={() => setEnvironment("live")} type="button">真实操作</button>
      <button className={environment === "fixture" ? "active" : ""} aria-pressed={environment === "fixture"}
        onClick={() => setEnvironment("fixture")} type="button">隔离实验</button>
    </div>
    {!desktop && <div className="m2-safety-banner"><ShieldAlert size={17} /><div><strong>浏览器无本地 SQLite 数据</strong><p>请运行桌面版以查看审计事件。</p></div></div>}
    {history.isError && <p className="error-text" role="alert">无法读取 SQLite 审计：{String(history.error)}</p>}
    {history.isPending && desktop && <p className="muted-copy">正在读取本地审计记录…</p>}
    {history.data?.length ? <div className="m2-history-list">
      {history.data.map((event: ActionEvent) => <Card key={event.event_id}><CardContent className="m2-history-entry">
        <span className="feature-icon"><Database size={17} /></span>
        <div><div className="m2-event-title"><strong>{STATUS_NAME[event.action_kind] ?? event.action_kind}</strong>
          <Badge>{event.status}</Badge></div>
          <p>{event.summary}</p><small>{event.feature_id} · {event.session_id.slice(0, 16)}…</small>
          {event.error_code && <small className="error-text">错误码：{event.error_code}</small>}
        </div><time>{formatTime(event.occurred_at_ms)}</time>
      </CardContent></Card>)}
    </div> : !history.isPending && <div className="empty-state"><Database size={30}/><h3>暂无{environment === "fixture" ? "实验" : "真实"}操作记录</h3>
      <p>{environment === "fixture" ? "只有桌面端真实运行 Rust Fixture 事务后才会产生审计事件。" : "M1 真实游戏仍为只读，不会记录伪造的修改历史。"}</p></div>}
    <p className="muted-copy"><Lock size={13} className="inline-block" /> 显示最近 100 条当前范围事件。SQLite WAL + FULL 同步；崩溃后只保留证据，不自动重播外部写入。</p>
  </div>;
}
