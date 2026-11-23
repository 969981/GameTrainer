import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Activity, ClipboardCopy, LockKeyhole, Scan, ShieldCheck, AlertTriangle } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { isTauriDesktop } from "@/lib/backend";
import { canProbe, checkLocalDpRom, dpProfiles, hex8, probeDpArm9,
  type DpOfflineEvidence, type DpSnapshot } from "@/lib/dp";

export function DiamondPearlPanel(){
  const desktop=isTauriDesktop();
  const [game,setGame]=useState<"Diamond"|"Pearl">("Diamond");
  const [version,setVersion]=useState("ADAE:13");
  const [port,setPort]=useState("3333");
  const [result,setResult]=useState<DpSnapshot|null>(null);
  const [fileResult,setFileResult]=useState<DpOfflineEvidence|null>(null);
  const [fileError,setFileError]=useState<string|null>(null);
  const [hashing,setHashing]=useState(false);
  const [copied,setCopied]=useState(false);
  const [copyError,setCopyError]=useState<string|null>(null);
  const list=dpProfiles.filter(p=>p.game===game);
  const profile=list.find(p=>p.game_code+":"+p.revision===version);
  const probe=useMutation({
    mutationFn:()=> {
      if(!profile)throw Error("ROM Code/Revision 不支持");
      return probeDpArm9(Number(port),profile.game_code,profile.revision);
    },
    onSuccess:r=>{setResult(r);setCopied(false);}
  });
  function selectGame(g:"Diamond"|"Pearl"){
    setGame(g);setVersion(g==="Diamond"?"ADAE:13":"APAE:13");
    setResult(null);setFileResult(null);setFileError(null);
  }
  async function verifyFile(file:File|undefined){
    setFileResult(null);setFileError(null);if(!file)return;
    setHashing(true);
    try{setFileResult(await checkLocalDpRom(file));}
    catch(e){setFileError(String(e));}
    finally{setHashing(false);}
  }
  async function copy(){
    if(!result)return;
    try {
      await navigator.clipboard.writeText(JSON.stringify({
        type:"DP_V0814_ARM9_GDB_READONLY",
        snapshot:result,
        warning:"No core-loaded ROM SHA256 binding, CPU synchronization, JIT/cave lifecycle proof or write authority."
      },null,2));
      setCopied(true);setCopyError(null);
    }catch(e){setCopyError(String(e));}
  }
  return <Card>
    <CardHeader><div className="section-row">
      <h3>DeSmuME · 钻石／珍珠 M9（v0.8.14）</h3><Badge>READ ONLY</Badge>
    </div></CardHeader>
    <CardContent>
      <div className="m2-safety-banner"><LockKeyhole size={18}/><div>
        <strong>从源 CT 迁移 10 个 ROM Profile；不开放任何 NDS 写入</strong>
        <p>需要支持 ARM9 GDB Stub 的 DeSmuME 开发版。先通过外部调试器手动暂停 ARM9，再执行只读采样。
        此功能不自动暂停或继续 CPU，也不会安装捕捉、传送、穿墙或 Overlay11 Thumb Hook。</p>
      </div></div>
      <div className="m2-actions">
        <Button variant={game==="Diamond"?"default":"outline"} onClick={()=>selectGame("Diamond")} disabled={probe.isPending}>钻石</Button>
        <Button variant={game==="Pearl"?"default":"outline"} onClick={()=>selectGame("Pearl")} disabled={probe.isPending}>珍珠</Button>
        <Badge>{list.length} ROM PROFILES</Badge>
      </div>
      <div className="m2-actions">
        <label className="parameter-field"><span>ROM Code / Revision / Header6C</span>
          <select aria-label="DP ROM Profile" value={version}
            onChange={e=>{setVersion(e.target.value);setResult(null);}} disabled={probe.isPending}>
            {list.map(p=><option key={p.game_code} value={p.game_code+":"+p.revision}>
              {p.language} · {p.game_code} Rev{p.revision} · 6C:{p.header6c.toString(16).toUpperCase()}
            </option>)}
          </select>
        </label>
        <label className="parameter-field"><span>ARM9 GDB 本机端口</span>
          <input aria-label="DP ARM9 GDB Port" inputMode="numeric" value={port}
            disabled={probe.isPending} onChange={e=>setPort(e.target.value)}/>
        </label>
      </div>
      <div className="m2-actions">
        <Button disabled={!desktop||probe.isPending||!profile||
          !canProbe(Number(port),profile?.game_code??"",profile?.revision??-1)}
          onClick={()=>{setResult(null);probe.mutate();}}>
          <Scan size={15}/>{probe.isPending?"读取 ARM9 Guest…":"只读采样 ARM9 / RNG"}
        </Button>
        <Button variant="outline" disabled={!result} onClick={()=>void copy()}>
          <ClipboardCopy size={14}/>{copied?"已复制 JSON":"复制 JSON 证据"}
        </Button>
      </div>
      {!desktop&&<p className="muted-copy">浏览器不提供虚假的 DeSmuME 模拟器连接结果。</p>}
      {probe.isError&&<p role="alert" className="error-text">{String(probe.error)}</p>}
      {copyError&&<p role="alert" className="error-text">{copyError}</p>}
      {result&&<div className="page-stack">
        <div className="connected-line"><ShieldCheck size={16}/>
          {result.game_code} Rev{result.revision} · GDB 停止响应、Header 及 ARM9 指令锚点一致（只读）
        </div>
        <div className="stats-grid">
          <div className="rng-metric"><span>ARM9 PC</span><strong>{hex8(result.arm9_pc)}</strong></div>
          <div className="rng-metric"><span>Current LCRNG Seed</span><strong>{hex8(result.current_seed)}</strong></div>
          <div className="rng-metric"><span>MT State[0]</span><strong>{hex8(result.mt_state0)}</strong></div>
          <div className="rng-metric"><span>MT Index</span><strong>{result.mt_index}</strong></div>
          <div className="rng-metric"><span>Delay</span><strong>{result.delay}</strong></div>
          <div className="rng-metric"><span>Trainer Pointer</span><strong>{hex8(result.trainer_ptr)}</strong></div>
        </div>
        {result.battle&&<div className="page-stack">
          <div className="section-row"><h3><Activity size={16}/> 珍珠 Overlay11 只读诊断</h3><Badge>STATIC</Badge></div>
          <p className="muted-copy">Callsite {hex8(result.battle.overlay11_callsite)}；
            Thumb BL 结构候选：{result.battle.thumb_bl_candidate?"匹配":"不匹配"}。
            72B Thumb Cave {hex8(result.battle.source_cave_addr)}：
            {result.battle.cave_all_zero?"当前全零":"非零，拒绝视为可用"}。
            这只是当前内存观察，不是原指令归属或 Guest 真执行证明。</p>
        </div>}
        <div className="m2-safety-banner"><AlertTriangle size={18}/><div>
          <strong>write_authorized = false</strong>
          <p>尚无 DeSmuME 内核完整 ROM 指纹绑定、双 CPU/JIT 一致性、Overlay11 安全重载或 0x5A/0xA5 真实执行结果。</p>
        </div></div>
      </div>}
      <div className="section-row"><h3>本地 .nds 文件 SHA256</h3><Badge>OFFLINE ONLY</Badge></div>
      <label className="parameter-field"><span>选择自己持有的 ROM，仅在本机计算，不上传</span>
        <input type="file" accept=".nds" aria-label="DP NDS ROM 文件"
          disabled={hashing} onChange={e=>void verifyFile(e.target.files?.[0])}/>
      </label>
      {hashing&&<p className="muted-copy">计算完整 ROM SHA256…</p>}
      {fileError&&<p role="alert" className="error-text">{fileError}</p>}
      {fileResult&&<div className="feature-metadata">
        <span>ROM Header</span><code>{fileResult.gameCode} Rev{fileResult.revision} · 6C:{fileResult.header6c.toString(16)}</code>
        <span>SHA256</span><code>{fileResult.sha256}</code>
        <span>来源对比</span><strong>{fileResult.note}</strong>
      </div>}
      <div className="section-row"><h3>来源 v0.8.14 各语言状态</h3></div>
      <div className="runtime-list">{list.map(p=><div className="process-row" key={p.game_code}>
        <strong>{p.language}</strong><code>{p.game_code} Rev{p.revision}</code>
        <Badge>{p.battle_static_audit?"Overlay11 STATIC":"ARM9 / RNG ADDRESSES"}</Badge>
      </div>)}</div>
      <p className="muted-copy">仅珍珠 APAE / APAI 有 Overlay11 完整静态调查。原 CT 的 72B Thumb 动态执行和
        两阶段卸载仅有静态与模拟测试，不是实机验证通过。日版、韩版不在 10 个严格源审计 ROM Profile 内。</p>
    </CardContent>
  </Card>;
}
