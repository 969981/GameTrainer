import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { AlertTriangle, CheckCircle2, ClipboardCopy, LockKeyhole, Scan } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { isTauriDesktop } from "@/lib/backend";
import { dpProfiles, hex8 } from "@/lib/dp";
import { getDesmumeProcesses } from "@/lib/hgss";
import {
  M11_ACK, hexBytes, m11Apply, m11Preview, m11Restore, m11Valid,
  type M11Feature, type M11Preview, type M11Record
} from "@/lib/dp-m11";

const verified=dpProfiles.filter(x=>x.rom_sha256!==null);
export function DiamondPearlM11LivePatchLab(){
  const desktop=isTauriDesktop();
  const [version,setVersion]=useState("APAE:13");
  const [feature,setFeature]=useState<M11Feature>("wall_collision");
  const [pid,setPid]=useState("");
  const [port9,setPort9]=useState("3333");
  const [port7,setPort7]=useState("3334");
  const [consent,setConsent]=useState(false);
  const [preview,setPreview]=useState<M11Preview|null>(null);
  const [receipt,setReceipt]=useState<M11Record|null>(null);
  const [copyError,setCopyError]=useState<string|null>(null);
  const [copied,setCopied]=useState(false);
  const [warningDismissed,setWarningDismissed]=useState(false);
  const p=verified.find(x=>x.game_code+":"+x.revision===version);
  const valid=!!p&&pid.trim()!==""&&m11Valid(Number(pid),Number(port9),Number(port7),p.game_code,p.revision,feature);
  const targets=useQuery({
    queryKey:["m11","desmume"],queryFn:getDesmumeProcesses,
    enabled:desktop,refetchInterval:12000,retry:false
  });
  function args(){
    if(!p)throw Error("必须选择原版珍珠 ROM");
    return [Number(pid),p.game_code,p.revision,Number(port9),Number(port7),feature] as const;
  }
  const check=useMutation({
    mutationFn:()=>m11Preview(...args()),
    onSuccess:r=>{setPreview(r);setReceipt(null);setCopied(false);}
  });
  const apply=useMutation({
    mutationFn:()=>m11Apply(...args(),consent?M11_ACK:""),
    onSuccess:r=>{setReceipt(r);setPreview(null);setCopied(false);}
  });
  const restore=useMutation({
    mutationFn:()=>m11Restore(...args()),
    onSuccess:r=>{setReceipt(r);setPreview(null);setCopied(false);}
  });
  const busy=check.isPending||apply.isPending||restore.isPending;
  const error=check.error??apply.error??restore.error;
  function invalidate(){setPreview(null);setReceipt(null);setCopied(false);}
  async function copy(){
    try{
      await navigator.clipboard.writeText(JSON.stringify({
        kind:"DP_M11_EXPERIMENTAL_LIVE_GUEST_SHORT_OPCODE",
        preview,receipt,
        caution:"Both GDB CPUs are independently stopped, not an atomic emulator safe point."
      },null,2));
      setCopyError(null);setCopied(true);
    }catch(e){setCopyError(String(e));}
  }
  return <Card><CardHeader><div className="section-row">
    <h3>M11 · 珍珠真实 Guest 16-bit 补丁实验台</h3>
    <Badge>MANUAL / EXPERIMENTAL</Badge>
  </div></CardHeader><CardContent>
    <div className="m2-safety-banner"><AlertTriangle size={18}/><div>
      <strong>这会真实修改 DeSmuME 的 ARM9 Guest 指令，不是模拟功能</strong>
      <p>目前只接入源码有完整 ROM 指纹的 APAE/APAI 穿墙和 APAE 英文捕捉。需要双 GDB 手动暂停、
        进程与 ROM M10 SHA256 绑定、ROM 加载时 JIT 关闭、原指令与周围指令签名匹配、
        本地持久化所有权日志。即使如此也没有两个 CPU 原子安全点或 JIT 运行期缓存保证：
        **使用前备份 SAV 和 DeSmuME 即时存档，可能崩溃或无法自动恢复**。</p>
    </div></div>
    <div className="m2-actions">
      <label className="parameter-field"><span>原版珍珠 ROM（严格）</span>
        <select aria-label="M11 ROM" value={version} disabled={busy}
          onChange={e=>{setVersion(e.target.value);invalidate();}}>
          {verified.map(p=><option key={p.game_code} value={p.game_code+":"+p.revision}>
            {p.game} {p.language} {p.game_code} Rev{p.revision}
          </option>)}
        </select>
      </label>
      <label className="parameter-field"><span>实验补丁</span>
        <select aria-label="M11 Feature" value={feature} disabled={busy}
          onChange={e=>{setFeature(e.target.value as M11Feature);invalidate();}}>
          <option value="wall_collision">穿墙 · PlayerAvatar Collision</option>
          {version==="APAE:13"&&<option value="capture_english">100% 捕捉 · 英文 Battle Overlay</option>}
        </select>
      </label>
      <label className="parameter-field"><span>Windows DeSmuME PID</span>
        <input aria-label="M11 Process ID" value={pid} disabled={busy}
          inputMode="numeric" placeholder="选择本机模拟器进程" onChange={e=>{setPid(e.target.value);invalidate();}}/>
      </label>
      <label className="parameter-field"><span>ARM9 GDB Port</span>
        <input aria-label="M11 ARM9 Port" inputMode="numeric" value={port9} disabled={busy}
          onChange={e=>{setPort9(e.target.value);invalidate();}}/>
      </label>
      <label className="parameter-field"><span>ARM7 GDB Port</span>
        <input aria-label="M11 ARM7 Port" inputMode="numeric" value={port7} disabled={busy}
          onChange={e=>{setPort7(e.target.value);invalidate();}}/>
      </label>
    </div>
    {desktop&&<div className="runtime-list">
      {targets.data?.map(t=><div className="process-row" key={t.process_id}>
        <strong>{t.executable_name}</strong><code>PID {t.process_id}</code>
        <Button size="sm" variant="outline" disabled={busy}
          onClick={()=>{setPid(String(t.process_id));invalidate();}}>选择</Button>
      </div>)}
    </div>}
    <div className="m2-actions">
      <Button disabled={!desktop||!valid||busy} onClick={()=>{setPreview(null);check.mutate();}}>
        <Scan size={15}/>{check.isPending?"验证 Guest 原指令…":"预览本机原指令（不写入）"}
      </Button>
      <Button variant="outline" disabled={!desktop||!valid||busy}
        onClick={()=>{setPreview(null);restore.mutate();}}>从持久化日志恢复原指令</Button>
    </div>
    {preview&&<div className="feature-metadata">
      <span>功能与来源</span><strong>{preview.plan.feature} · {preview.plan.source}</strong>
      <span>Patch 地址</span><code>{hex8(preview.plan.address)}</code>
      <span>原指令 → 实验指令</span><code>{hexBytes(preview.plan.original)} → {hexBytes(preview.plan.replacement)}</code>
      <span>ARM9 / ARM7 当前 PC</span><code>{hex8(preview.arm9_pc)} / {hex8(preview.arm7_pc)}</code>
      <span>ROM Epoch</span><code>{preview.rom_epoch}</code>
      <span>危险提醒</span><strong>{preview.warning}</strong>
    </div>}
    <label className="parameter-field">
      <span>风险确认</span>
      <span><input type="checkbox" checked={consent} disabled={busy}
        onChange={e=>setConsent(e.target.checked)}/>
        我已备份游戏存档和即时存档，并明确同意在真实 DeSmuME Guest 中进行有风险的实验指令写入</span>
    </label>
    <div className="m2-actions">
      <Button disabled={!desktop||!preview||!consent||busy||!valid}
        onClick={()=>apply.mutate()}>
        <LockKeyhole size={15}/>{apply.isPending?"正在写入并回读…":"确认并执行真实 Guest 写入"}
      </Button>
      <Button variant="outline" disabled={!receipt&&!preview} onClick={()=>void copy()}>
        <ClipboardCopy size={14}/>{copied?"已复制":"复制事务证据"}
      </Button>
    </div>
    {error&&<p className="error-text" role="alert">{String(error)}</p>}
    {copyError&&<p className="error-text" role="alert">{copyError}</p>}
    {receipt&&<div className="page-stack">
      <div className="section-row"><h3>事务结果</h3>
        <Badge>{receipt.phase.toUpperCase()}</Badge></div>
      <div className="connected-line"><CheckCircle2 size={15}/>
        {receipt.owner_id} · {receipt.phase} · {hex8(receipt.plan.address)}
      </div>
      <p className="muted-copy">本地持久化日志存放于 %LOCALAPPDATA%\GameTrainer\DP-M11-journal。
        若写入 ACK 丢失、状态变为 uncertain 或 ROM/Overlay 已切换，不能盲目恢复；
        请保留日志并检查 RAM/即时存档。</p>
    </div>}
    <div className="m2-safety-banner"><LockKeyhole size={16}/><div>
      <strong>传送、会心／命中／OHKO、Guest 72B Cave 仍未开放真实热清理</strong>
      <p>这些功能涉及完整 Thumb 跳板、JIT 指令缓存和 Overlay11 生命周期，
        仅靠双 GDB 暂停以及 PC 不在 Cave 不能证明没有待返回的调用栈；
        尚需额外的模拟器内核安全点和恢复故障注入。</p>
    </div></div>
    {desktop&&!warningDismissed&&<div className="m2-actions">
      <Button size="sm" variant="outline" onClick={()=>setWarningDismissed(true)}>
        已阅读实验风险提醒
      </Button>
    </div>}
  </CardContent></Card>;
}
