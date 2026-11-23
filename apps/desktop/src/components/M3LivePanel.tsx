import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, CheckCircle2, FileCode, Lock, Plug, RotateCcw, ShieldAlert, Unplug } from "lucide-react";
import { isTauriDesktop } from "@/lib/backend";
import {
  connectLive, liveStatus, previewCapture, previewCrit, applyLive, restoreLive,
  cancelLive, disconnectLive, liveHistory, type LivePreview, type LiveReceipt,
} from "@/lib/live";
import { Card, CardHeader, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";

export function M3LivePanel(){
  const desktop=isTauriDesktop();
  const query=useQueryClient();
  const [port,setPort]=useState("2345");
  const [feature,setFeature]=useState<"capture"|"crit">("capture");
  const [template,setTemplate]=useState<File|null>(null);
  const [preview,setPreview]=useState<LivePreview|null>(null);
  const [confirmed,setConfirmed]=useState(false);
  const [receipt,setReceipt]=useState<LiveReceipt|null>(null);
  const [localError,setLocalError]=useState<string|null>(null);
  const status=useQuery({queryKey:["m3","status"],queryFn:liveStatus,enabled:desktop,refetchInterval:5000,retry:false});
  const history=useQuery({queryKey:["m3","history"],queryFn:liveHistory,enabled:desktop,refetchInterval:10000,retry:false});
  const refresh=()=>{void query.invalidateQueries({queryKey:["m3"]});};
  const connect=useMutation({mutationFn:connectLive,onSuccess:()=>{setPreview(null);setReceipt(null);refresh();}});
  const create=useMutation({mutationFn:async()=>{
    if(feature==="capture")return previewCapture();
    if(!template)throw Error("需要先选择原 v4.6.0 源包内的 cheat_crit_dispatch_thumb.bin。");
    const bytes=Array.from(new Uint8Array(await template.arrayBuffer()));
    return previewCrit(bytes);
  },onSuccess:result=>{setPreview(result);setConfirmed(false);setLocalError(null);refresh();}});
  const apply=useMutation({mutationFn:applyLive,onSuccess:result=>{setReceipt(result);setPreview(null);setConfirmed(false);refresh();},onError:()=>{setPreview(null);setConfirmed(false);refresh();}});
  const restore=useMutation({mutationFn:restoreLive,onSuccess:result=>{setReceipt(result);refresh();},onError:refresh});
  const cancel=useMutation({mutationFn:cancelLive,onSuccess:()=>{setPreview(null);setConfirmed(false);refresh();}});
  const disconnect=useMutation({mutationFn:disconnectLive,onSuccess:()=>{setPreview(null);setReceipt(null);refresh();}});
  const state=status.data;
  const busy=connect.isPending||create.isPending||apply.isPending||restore.isPending||cancel.isPending||disconnect.isPending;
  const hasPatch=!!state?.active_feature;
  const issue=[connect.error,create.error,apply.error,restore.error,cancel.error,disconnect.error]
    .find(e=>e!==null);
  const parsedPort=Number(port);
  const validPort=Number.isInteger(parsedPort)&&parsedPort>=1024&&parsedPort<=65535;
  return <div className="page-stack">
    <div className="page-heading"><div><div className="eyebrow">M3 / LIVE GBA DEBUGGER</div><h2>真实写入 · mGBA GDB 调试模式</h2>
      <p>仅允许本机暂停的 mGBA GDB 服务、已知整 ROM SHA1 和固定的 Ruby/Sapphire 补丁。</p></div>
      <Badge>EXPERIMENTAL / OFFLINE</Badge>
    </div>
    <div className="m2-safety-banner"><ShieldAlert size={19}/><div>
      <strong>真实游戏内存将被修改，可能导致崩溃或存档异常</strong>
      <p>先备份 .sav 和 SaveState，关闭联机功能，确认 ROM 原版及游戏处于安全状态。未知 ROM、德国 Debug、汉化/随机版以及未验证 BattleCrit 版本会被拒绝。不能同时使用其他金手指工具。</p>
    </div></div>
    {!desktop && <div className="m2-safety-banner"><Lock size={19}/><div><strong>浏览器模式不可用</strong><p>只有 Windows Tauri 实际连接 mGBA GDB 后才能执行；这里不伪造连接或写入回执。</p></div></div>}
    {(issue||localError||status.error) && <p role="alert" className="error-text">{localError??String(issue??status.error)}</p>}
    <div className="m2-layout">
      <Card><CardHeader><h3>01 · 可信 GDB 会话</h3></CardHeader><CardContent>
        <p className="muted-copy">在 mGBA 内启动 GDB Server（默认 2345），让模拟器暂停，再从此处连接。首次连接会通过 GDB 读取 ROM 并计算完整 SHA1，只有匹配 28 份已登记原版镜像才继续。</p>
        <label className="parameter-field m2-seed-entry"><span>GDB 本机端口</span>
          <input value={port} aria-label="mGBA GDB 端口" onChange={e=>setPort(e.target.value)}
            disabled={!!state||busy} placeholder="2345"/>
        </label>
        <div className="m2-actions"><Button disabled={!desktop||busy||!!state||!validPort}
          onClick={()=>connect.mutate(parsedPort)}><Plug size={15}/> {connect.isPending?"正在验证整 ROM…":"连接并验证完整 ROM"}</Button>
          {state && <Button variant="outline" disabled={busy||hasPatch||!!preview}
            onClick={()=>disconnect.mutate()}><Unplug size={15}/> 安全断开</Button>}
        </div>
        {state && <div className="m3-identity">
          <div><span>Game / Build</span><strong>{state.game_code} Rev{state.revision}</strong></div>
          <div><span>ROM SHA1（GDB Guest）</span><code>{state.sha1}</code></div>
          <div><span>ROM 大小</span><code>{(state.rom_bytes/1024/1024).toFixed(0)} MiB</code></div>
          <div><span>CPU</span><Badge>{state.gdb_paused?"PAUSED":"RUNNING"}</Badge></div>
          <div><span>活跃补丁</span><strong>{state.active_feature??"无"}</strong></div>
        </div>}
        <p className="muted-copy">当前不依赖 Win32 WriteProcessMemory；RSP 命令在 Guest CPU 暂停时由 mGBA 调试器执行，并在写后读回。</p>
      </CardContent></Card>
      <Card><CardHeader><h3>02 · 补丁与 Hook</h3></CardHeader><CardContent>
        <div className="m3-select"><label htmlFor="m3-feature">操作功能</label>
          <select id="m3-feature" value={feature} disabled={!state||busy||!!preview||hasPatch}
            onChange={e=>setFeature(e.target.value as "capture"|"crit")}>
            <option value="capture">100% 捕捉 · 双 ROM 指令</option>
            <option value="crit">我方必会心 · 实验性 Thumb Hook</option>
          </select>
        </div>
        {feature==="crit" && <div className="m3-payload">
          <label htmlFor="m3-payload">原 v4.6.0 GPL-3.0 Payload（124 字节）</label>
          <input id="m3-payload" type="file" accept=".bin" disabled={!state||busy||!!preview||hasPatch}
            onChange={e=>setTemplate(e.target.files?.[0]??null)}/>
          <small>仅接受官方研究源包中的 cheat_crit_dispatch_thumb.bin，并由 Rust 核验固定 SHA256。不会上传源文件到服务器。</small>
          {state&&!state.experimental_hook_supported&&<p className="error-text">此 ROM 没有独立的实验战斗 Hook 证明，禁止启用。</p>}
        </div>}
        <Button className="w-full" disabled={!desktop||!state||!state.gdb_paused||busy||!!preview||hasPatch||(feature==="crit"&&(!template||!state.experimental_hook_supported))}
          onClick={()=>create.mutate()}><FileCode size={15}/> 生成只读差异预览</Button>
        {preview && <div className="m3-preview">
          <h4>{preview.feature_id} · {preview.spans.length} 个内存区域</h4>
          <p className="error-text">{preview.risk}</p>
          {preview.spans.map(span=><div key={span.guest_address} className="m3-span">
            <strong>{span.label}</strong><code>{span.guest_address}</code>
            <div><small>原值 {span.before.length>48?span.before.slice(0,48)+"…":span.before}</small>
              <small>修改 {span.after.length>48?span.after.slice(0,48)+"…":span.after}</small></div>
          </div>)}
          <p className="muted-copy">一次性 Token，{preview.expires_in_seconds} 秒有效。确认后 Rust 再次核对原值和 ROM 会话，并先写 SQLite 审计意图。</p>
          <label className="m2-confirm"><input type="checkbox" checked={confirmed}
            onChange={e=>setConfirmed(e.target.checked)}/>
            我已备份存档，仅在离线单机测试，并接受实验性功能可能导致游戏异常
          </label>
          <div className="m2-actions"><Button variant="outline" disabled={busy} onClick={()=>cancel.mutate()}>取消预览</Button>
            <Button disabled={!confirmed||busy} onClick={()=>apply.mutate(preview.token)}>
              <AlertTriangle size={15}/> 确认实际写入</Button>
          </div>
        </div>}
        {state?.active_feature && <div className="m3-active">
          <Badge>{state.active_status?.toUpperCase()??"ACTIVE"}</Badge>
          <p>GDB 已允许游戏继续运行。卸载时自动发送暂停命令，核对当前仍属于本工具的字节，再按相反顺序恢复。</p>
          <Button className="w-full" variant="outline" disabled={busy}
            onClick={()=>restore.mutate()}><RotateCcw size={15}/> 暂停 Guest CPU 并验证恢复</Button>
        </div>}
        {receipt && <div role="status" className="m2-receipt">
          <CheckCircle2 size={16}/><div><strong>Rust 回执：{receipt.state}</strong>
            <p><code>{receipt.owner_id}</code></p><small>审计确认：{String(receipt.audit_committed)}</small></div>
        </div>}
      </CardContent></Card>
    </div>
    <Card><CardHeader><h3>03 · Live SQLite 审计</h3></CardHeader><CardContent>
      <p className="muted-copy">与 M2 Fixture SQLite 完全分离。先记录 apply_intent，再改写 Guest；失去连接或恢复失败不自动碰旧地址。</p>
      <div className="m3-event-list">{(history.data??[]).slice(0,8).map(event=>
        <div key={event.event_id}><strong>{event.action_kind}</strong><span>{event.feature_id} · {event.status}</span>
          <small>{new Date(event.occurred_at_ms).toLocaleString("zh-CN")}</small></div>)}
      </div>
    </CardContent></Card>
  </div>;
}
