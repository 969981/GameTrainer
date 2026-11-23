import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { AlertTriangle, LockKeyhole, Search } from "lucide-react";
import { isTauriDesktop } from "@/lib/backend";
import { hgssProfiles } from "@/lib/hgss";
import { blockerLabels, checkM7, cpuStable, portsValid, type M7Report } from "@/lib/hgss-m7";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";

const profiles=hgssProfiles.filter(p=>p.static_audit_1451);
const hex=(n:number|null)=>n===null?"—":"0x"+n.toString(16).toUpperCase().padStart(8,"0");
export function HgssM7SafetyPanel(){
  const desktop=isTauriDesktop();
  const [version,setVersion]=useState("IPKJ:0");
  const [arm9,setArm9]=useState("3333"),[arm7,setArm7]=useState("3334");
  const [result,setResult]=useState<M7Report|null>(null);
  const [copied,setCopied]=useState(false);
  const profile=profiles.find(p=>p.game_code+":"+p.revision===version);
  const probe=useMutation({
    mutationFn:()=>{if(!profile)throw Error("版本不存在");
      return checkM7(Number(arm9),Number(arm7),profile.game_code,profile.revision);},
    onSuccess:r=>{setResult(r);setCopied(false);}
  });
  async function copy(){
    if(result){await navigator.clipboard.writeText(JSON.stringify({
      type:"HGSS_M7_READONLY_PREFLIGHT",result,
      disclaimer:"Two independent GDB stop observations are NOT synchronized pause or write authority."
    },null,2));setCopied(true);}
  }
  return <Card><CardHeader><div className="section-row"><h3>M7 · ARM9/ARM7 + ROM 身份安全门禁</h3>
    <Badge>READ ONLY / BLOCKED</Badge></div></CardHeader><CardContent>
    <div className="m2-safety-banner"><LockKeyhole size={18}/><div><strong>后端不会开放任何 NDS 写入</strong>
      <p>需要支持 --arm9gdb 和 --arm7gdb 的 DeSmuME 开发构建。在两个调试器中手动暂停后采样。两个独立 Stop Reply 不能证明同一进程、同步停机或 JIT 缓存安全。</p>
    </div></div>
    <div className="m2-actions">
      <label className="parameter-field"><span>ROM Code / Revision（仅来源证据选择）</span>
        <select aria-label="M7 HGSS Version" value={version}
          onChange={e=>{setVersion(e.target.value);setResult(null);}}>
          {profiles.map(p=><option key={p.game_code+":"+p.revision}
            value={p.game_code+":"+p.revision}>{p.game} {p.language} {p.game_code} Rev{p.revision}</option>)}
        </select></label>
      <label className="parameter-field"><span>ARM9 本机端口</span>
        <input aria-label="M7 ARM9 Port" value={arm9} onChange={e=>setArm9(e.target.value)}/></label>
      <label className="parameter-field"><span>ARM7 本机端口</span>
        <input aria-label="M7 ARM7 Port" value={arm7} onChange={e=>setArm7(e.target.value)}/></label>
    </div>
    <div className="m2-actions"><Button disabled={!desktop||!profile||probe.isPending||
      !portsValid(Number(arm9),Number(arm7))}
      onClick={()=>{setResult(null);probe.mutate();}}><Search size={15}/>
      {probe.isPending?"正在读取停止状态…":"只读预检双 CPU"}</Button>
      <Button variant="outline" disabled={!result} onClick={()=>void copy()}>{copied?"JSON 已复制":"复制 JSON 证据"}</Button></div>
    {!desktop&&<p className="muted-copy">浏览器仅展示诊断入口，不会伪造 GDB 连接。</p>}
    {probe.isError&&<p className="error-text" role="alert">{String(probe.error)}</p>}
    {result&&<div className="page-stack">
      <div className="runtime-list">{[result.arm9,result.arm7].map(c=>
        <div className="process-row" key={c.cpu}><strong>{c.cpu} · :{c.port}</strong>
          <code>PC {hex(c.pc_second)}</code>
          <Badge>{cpuStable(c)?"仅独立停机证据":"未验证"}</Badge>
          {c.error&&<span className="error-text">{c.error}</span>}</div>)}</div>
      <div className="section-row"><h3><AlertTriangle size={16}/> 写入门禁未满足</h3>
        <Badge>{result.blockers.length} 项阻断</Badge></div>
      <div className="runtime-list">{result.blockers.map(b=><div className="process-row" key={b}>
        <LockKeyhole size={14}/><span>{blockerLabels[b]??b}</span></div>)}</div>
      <p className="muted-copy">write_authorized = {String(result.write_authorized)}。
        仍需运行中 ROM 完整指纹绑定、双 CPU 同步暂停、JIT/指令缓存、Overlay 代次和真实故障恢复的实机证据。</p>
    </div>}
    </CardContent></Card>;
}
