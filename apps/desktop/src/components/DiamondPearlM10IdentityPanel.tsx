import { useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Cpu, ClipboardCopy, Fingerprint, LockKeyhole, ShieldAlert } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { isTauriDesktop } from "@/lib/backend";
import { dpProfiles, hex8 } from "@/lib/dp";
import { getDesmumeProcesses } from "@/lib/hgss";
import {
  dpM10Blockers, dpM10Identity, dpM10RuntimeAudit, validDpM10,
  type DpM10Identity, type DpM10Audit,
} from "@/lib/dp-m10";

function ProofField({ label, ok }: { label:string;ok:boolean }) {
  return <div className="process-row"><strong>{label}</strong>
    <Badge>{ok?"诊断通过":"未满足"}</Badge></div>;
}
function CpuProof({ cpu, port, first, second, error }: {
  cpu:string;port:number;first:number|null;second:number|null;error:string|null;
}) {
  return <div className="process-row"><strong>{cpu} :{port}</strong>
    <code>{second===null?"无 PC":hex8(second)}</code>
    <Badge>{first!==null&&first===second&&!error?"独立停机观察":"未验证"}</Badge>
    {error&&<span className="error-text">{error}</span>}
  </div>;
}
export function DiamondPearlM10IdentityPanel() {
  const desktop=isTauriDesktop();
  const [version,setVersion]=useState("APAE:13");
  const [pid,setPid]=useState("");
  const [arm9,setArm9]=useState("3333");
  const [arm7,setArm7]=useState("3334");
  const [identity,setIdentity]=useState<DpM10Identity|null>(null);
  const [audit,setAudit]=useState<DpM10Audit|null>(null);
  const [clipboard,setClipboard]=useState(false);
  const [copyError,setCopyError]=useState<string|null>(null);
  const profile=dpProfiles.find(p=>p.game_code+":"+p.revision===version);
  const valid=!!profile&&validDpM10(Number(pid),Number(arm9),Number(arm7),
    profile.game_code,profile.revision)&&pid.trim().length>0;
  const processes=useQuery({
    queryKey:["m10","desmume-processes"],queryFn:getDesmumeProcesses,
    enabled:desktop,refetchInterval:12000,retry:false,
  });
  const rom=useMutation({
    mutationFn:()=>{
      if(!profile)throw Error("未知 D/P ROM Profile");
      return dpM10Identity(Number(pid),Number(arm9),Number(arm7),profile.game_code,profile.revision);
    },
    onSuccess:r=>{setIdentity(r);setAudit(null);setClipboard(false);},
  });
  const runtime=useMutation({
    mutationFn:()=>{
      if(!profile)throw Error("未知 D/P ROM Profile");
      return dpM10RuntimeAudit(Number(pid),Number(arm9),Number(arm7),profile.game_code,profile.revision);
    },
    onSuccess:r=>{setIdentity(r.identity);setAudit(r);setClipboard(false);},
  });
  const busy=rom.isPending||runtime.isPending;
  const error=rom.error??runtime.error;
  async function copy(){
    if(!identity)return;
    try{
      await navigator.clipboard.writeText(JSON.stringify({
        evidence_kind:"DP_M10_CORE_ROM_SOURCE_SHA256_OS_GDB_READONLY",
        identity,independent_cpu_overlay_audit:audit,
        warning:"Never proves atomic CPU stop, JIT coherency, safe Overlay11 injection or patch restoration."
      },null,2));
      setClipboard(true);setCopyError(null);
    }catch(e){setCopyError(String(e));}
  }
  return <Card><CardHeader><div className="section-row">
    <h3>钻石／珍珠 M10 · 运行中 ROM SHA-256 与双 CPU 诊断</h3>
    <Badge>NO GUEST WRITES</Badge>
  </div></CardHeader><CardContent>
    <div className="m2-safety-banner"><LockKeyhole size={18}/><div>
      <strong>ROM 来源必须由 DeSmuME 内核产生，不能采用外部文件的哈希冒充</strong>
      <p>需要同时启用 M8/M10 源码诊断桥、ARM9 和 ARM7 两个 GDB Stub。
        Windows 后端检验 Named Pipe 服务 PID、进程创建时间及双 GDB TCP 归属；
        ROM 加载过程中计算完整 SHA-256。只有珍珠 APAE 和 APAI 有可比对的整包原版哈希。</p>
    </div></div>
    <div className="m2-actions">
      <label className="parameter-field"><span>钻石／珍珠原版 Code / Revision</span>
        <select aria-label="M10 DP Version" value={version}
          disabled={busy} onChange={e=>{setVersion(e.target.value);setIdentity(null);setAudit(null);}}>
          {dpProfiles.map(p=><option key={p.game_code} value={p.game_code+":"+p.revision}>
            {p.game==="Diamond"?"钻石":"珍珠"} {p.language} · {p.game_code} Rev{p.revision}
            {p.rom_sha256?" · FULL SHA256":" · COMPONENT ONLY"}
          </option>)}
        </select>
      </label>
      <label className="parameter-field"><span>DeSmuME Windows PID</span>
        <input aria-label="M10 DeSmuME PID" value={pid} disabled={busy}
          inputMode="numeric" placeholder="从下方进程列表选择或输入" onChange={e=>setPid(e.target.value)}/>
      </label>
      <label className="parameter-field"><span>ARM9 GDB 端口</span>
        <input aria-label="M10 ARM9 Port" value={arm9} disabled={busy}
          inputMode="numeric" onChange={e=>setArm9(e.target.value)}/>
      </label>
      <label className="parameter-field"><span>ARM7 GDB 端口</span>
        <input aria-label="M10 ARM7 Port" value={arm7} disabled={busy}
          inputMode="numeric" onChange={e=>setArm7(e.target.value)}/>
      </label>
    </div>
    {desktop&&<div className="runtime-list">
      {processes.data?.map(p=><div className="process-row" key={p.process_id}>
        <span>{p.executable_name}</span><code>PID {p.process_id}</code>
        <Button size="sm" variant="outline" disabled={busy}
          onClick={()=>{setPid(String(p.process_id));setIdentity(null);setAudit(null);}}>
          选择进程
        </Button></div>)}
    </div>}
    <div className="m2-actions">
      <Button disabled={!desktop||!valid||busy} onClick={()=>{setIdentity(null);setAudit(null);rom.mutate();}}>
        <Fingerprint size={16}/>{rom.isPending?"读取内核身份…":"验证加载 ROM SHA-256"}
      </Button>
      <Button variant="outline" disabled={!desktop||!valid||busy}
        onClick={()=>{setIdentity(null);setAudit(null);runtime.mutate();}}>
        <Cpu size={16}/>{runtime.isPending?"双 CPU / Overlay11 只读采样…":"验证双 CPU / Overlay11 状态"}
      </Button>
      <Button variant="outline" disabled={!identity} onClick={()=>void copy()}>
        <ClipboardCopy size={14}/>{clipboard?"已复制证据":"复制全部 JSON"}
      </Button>
    </div>
    {!desktop&&<p className="muted-copy">浏览器模式不提供本机 GDB/ROM 假数据。须使用 Windows Tauri。</p>}
    {error&&<p className="error-text" role="alert">{String(error)}</p>}
    {copyError&&<p className="error-text" role="alert">{copyError}</p>}
    {identity&&<div className="page-stack">
      <div className="section-row"><h3>运行态 ROM 身份</h3>
        <Badge>{identity.diagnostic_identity_verified?"OS / ROM 匹配（只读）":"未完成身份确认"}</Badge></div>
      <div className="stats-grid">
        <div className="rng-metric"><span>ROM</span><strong>{identity.game_code} Rev{identity.revision}</strong></div>
        <div className="rng-metric"><span>ROM Epoch</span><strong>{identity.rom_epoch}</strong></div>
        <div className="rng-metric"><span>Windows PID</span><strong>{identity.process_id}</strong></div>
        <div className="rng-metric"><span>ROM 长度</span><strong>{identity.rom_size.toLocaleString()}</strong></div>
        <div className="rng-metric"><span>原版完整 SHA256 参照</span><strong>{identity.full_rom_reference_available?"已登记":"无，不能对账"}</strong></div>
        <div className="rng-metric"><span>JIT 载入时开关</span><strong>{identity.jit_enabled_when_loaded?"开启":"关闭（非缓存证明）"}</strong></div>
      </div>
      <div className="feature-metadata">
        <span>内核加载 SHA-256</span><code>{identity.core_sha256}</code>
        <span>原版参考 SHA-256</span><code>{identity.known_full_rom_sha256??"未提供完整 ROM SHA256"}</code>
        <span>进程创建 FILETIME</span><code>{identity.process_created_filetime}</code>
      </div>
      <ProofField label="ROM Reader 字节流完整" ok={identity.core_rom_stream_complete}/>
      <ProofField label="Named Pipe / Windows 进程身份" ok={identity.pipe_and_process_bound}/>
      <ProofField label="ARM9/ARM7 两端口的 OS PID 归属" ok={identity.both_gdb_ports_bound}/>
      <ProofField label="运行中 ROM 与已审计完整 SHA256" ok={identity.full_rom_matches_catalog}/>
      {audit&&<div className="page-stack">
        <div className="section-row"><h3>ARM9 / ARM7 独立停机与 Overlay11</h3>
          <Badge>仅局部观察</Badge></div>
        <CpuProof cpu="ARM9" port={audit.arm9.port} first={audit.arm9.first_pc}
          second={audit.arm9.second_pc} error={audit.arm9.error}/>
        <CpuProof cpu="ARM7" port={audit.arm7.port} first={audit.arm7.first_pc}
          second={audit.arm7.second_pc} error={audit.arm7.error}/>
        <ProofField label="两次 ROM 身份快照的加载代次一致" ok={audit.rom_epoch_stable_across_gdb_samples}/>
        <ProofField label="ARM9/ARM7 各自 Stop Reply 稳定" ok={audit.independent_stops_observed}/>
        <ProofField label="两个 CPU 同一原子停机边界" ok={audit.cpu_pair_atomically_stopped}/>
        <ProofField label="JIT / ICache 代码可见性验收" ok={audit.jit_icache_coherence_verified}/>
        <ProofField label="Overlay11 静态 Callsite Guest 只读观察" ok={audit.overlay11_static_source_present}/>
        {audit.arm9_pc_inside_known_thumb_cave===true&&
          <p className="error-text">ARM9 PC 位于源文件标记的 72B Cave 范围：严禁擦除指令。</p>}
        {audit.overlay11_guest_observation?.battle&&<p className="muted-copy">
          当前观察 Callsite {hex8(audit.overlay11_guest_observation.battle.overlay11_callsite)}；
          Cave 全零：{String(audit.overlay11_guest_observation.battle.cave_all_zero)}。
          不能凭当前 PC 不在 Cave、连续轮询或全零判定卸载安全。
        </p>}
      </div>}
      <div className="section-row"><h3><ShieldAlert size={16}/> 仍阻断全部 NDS 修改</h3>
        <Badge>{(audit?.blockers??identity.blockers).length} 项门禁</Badge></div>
      <div className="runtime-list">{(audit?.blockers??identity.blockers).map(b=>
        <div className="process-row" key={b}><LockKeyhole size={14}/>
          <span>{dpM10Blockers[b]??b}</span></div>)}</div>
      <p className="muted-copy">身份诊断不授权写入：guest_write_allowed = {String(identity.guest_write_allowed)}，
        runtime.write_authorized = {String(audit?.write_authorized??false)}。
        真实 ARM9/ARM7 原子停机、同地址 JIT/ICache 以及 Overlay11 安全恢复仍需实机证据。</p>
    </div>}
  </CardContent></Card>;
}
