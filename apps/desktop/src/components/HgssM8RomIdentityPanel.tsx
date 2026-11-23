import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { AlertTriangle, ClipboardCopy, Fingerprint, LockKeyhole } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { isTauriDesktop } from "@/lib/backend";
import { hgssProfiles } from "@/lib/hgss";
import { m8Blockers, probeM8, validateM8, type M8Report } from "@/lib/hgss-m8";

const profiles=hgssProfiles.filter(p=>p.static_audit_1451);
export function HgssM8RomIdentityPanel(){
  const desktop=isTauriDesktop();
  const [version,setVersion]=useState("IPKJ:0");
  const [pid,setPid]=useState("");
  const [arm9,setArm9]=useState("3333");
  const [arm7,setArm7]=useState("3334");
  const [report,setReport]=useState<M8Report|null>(null);
  const [copyError,setCopyError]=useState<string|null>(null);
  const [copied,setCopied]=useState(false);
  const profile=profiles.find(p=>p.game_code+":"+p.revision===version);
  const valid=validateM8(Number(pid),Number(arm9),Number(arm7)) && pid.trim()!=="";
  const probe=useMutation({
    mutationFn:()=>{if(!profile)throw Error("没有此 ROM 静态档案");
      return probeM8(Number(pid),profile.game_code,profile.revision,Number(arm9),Number(arm7));},
    onSuccess:r=>{setReport(r);setCopied(false);}
  });
  async function copy(){
    if(!report)return;
    try{
      await navigator.clipboard.writeText(JSON.stringify({
        kind:"GAME_TRAINER_HGSS_M8_CORE_ROM_IDENTITY_V1",
        report,notice:"diagnostic identity only; no live memory patch authorization"
      },null,2));
      setCopied(true);setCopyError(null);
    }catch(e){setCopyError(String(e));}
  }
  return <Card>
    <CardHeader><div className="section-row">
      <h3>M8 · DeSmuME 内核 ROM 身份桥</h3><Badge>READ ONLY / NO PATCH</Badge>
    </div></CardHeader>
    <CardContent>
      <div className="m2-safety-banner"><Fingerprint size={18}/><div>
        <strong>从模拟器正在读取的 ROM 字节生成指纹，而非选择文件路径</strong>
        <p>仅针对启用 GT_HGSS_M8_BRIDGE + GDB_STUB 的 DeSmuME 源码构建。
          本机命名管道返回 ROM 加载代次、SHA1 和实际 GDB 端口；Windows 再校验服务 PID、进程创建时间及两个 TCP 监听端口的拥有者。</p>
      </div></div>
      <div className="m2-actions">
        <label className="parameter-field"><span>HGSS 原版静态 Profile</span>
          <select aria-label="M8 ROM Profile" value={version} disabled={probe.isPending}
            onChange={e=>{setVersion(e.target.value);setReport(null);}}>
            {profiles.map(p=><option key={p.game_code+":"+p.revision}
              value={p.game_code+":"+p.revision}>{p.game} {p.language} {p.game_code} Rev{p.revision}</option>)}
          </select>
        </label>
        <label className="parameter-field"><span>DeSmuME Windows PID</span>
          <input aria-label="M8 DeSmuME PID" inputMode="numeric" value={pid}
            onChange={e=>setPid(e.target.value)} placeholder="例如 12345"/>
        </label>
        <label className="parameter-field"><span>ARM9 GDB Port</span>
          <input aria-label="M8 ARM9 Port" inputMode="numeric" value={arm9}
            onChange={e=>setArm9(e.target.value)}/>
        </label>
        <label className="parameter-field"><span>ARM7 GDB Port</span>
          <input aria-label="M8 ARM7 Port" inputMode="numeric" value={arm7}
            onChange={e=>setArm7(e.target.value)}/>
        </label>
      </div>
      <div className="m2-actions">
        <Button disabled={!desktop||!valid||!profile||probe.isPending}
          onClick={()=>{setReport(null);probe.mutate();}}><Fingerprint size={15}/>
          {probe.isPending?"读取 DeSmuME 进程证据…":"从内核验证 ROM 身份"}
        </Button>
        <Button variant="outline" disabled={!report} onClick={()=>void copy()}>
          <ClipboardCopy size={14}/>{copied?"已复制 JSON":"复制诊断证据"}
        </Button>
      </div>
      {!desktop&&<p className="muted-copy">浏览器预览不会连接本机模拟器或伪造成功结果。</p>}
      {probe.isError&&<p className="error-text" role="alert">{String(probe.error)}</p>}
      {copyError&&<p className="error-text" role="alert">{copyError}</p>}
      {report&&<div className="page-stack">
        <div className="section-row"><h3>运行态身份快照</h3>
          <Badge>{report.diagnostic_identity_verified?"ROM / OS 绑定匹配（只读）":"身份尚未通过"}</Badge></div>
        <div className="stats-grid">
          <div className="rng-metric"><span>GameCode / Rev</span><strong>{report.code} / {report.revision}</strong></div>
          <div className="rng-metric"><span>Windows PID</span><strong>{report.process_id}</strong></div>
          <div className="rng-metric"><span>ROM Load Epoch</span><strong>{report.rom_load_epoch}</strong></div>
          <div className="rng-metric"><span>ROM 字节数</span><strong>{report.rom_size.toLocaleString()}</strong></div>
          <div className="rng-metric"><span>OS GDB TCP PID 绑定</span><strong>{report.gdb_socket_pid_binding_verified?"匹配":"不匹配"}</strong></div>
          <div className="rng-metric"><span>ROM 内核 SHA1</span><strong>{report.running_rom_digest_matches_catalog?"原版匹配":"未匹配"}</strong></div>
        </div>
        <div className="feature-metadata">
          <span>实际加载 ROM 的 SHA1</span><code>{report.rom_sha1}</code>
          <span>原版登记 SHA1</span><code>{report.catalog_sha1??"无"}</code>
          <span>JIT 加载时状态</span><strong>{report.jit_enabled_when_loaded?"开启":"关闭（仅加载时观察）"}</strong>
        </div>
        <div className="section-row"><h3><LockKeyhole size={16}/> 写入继续禁止</h3>
          <Badge>{report.blockers.length} 项未满足</Badge></div>
        <div className="runtime-list">{report.blockers.map(b=><div className="process-row" key={b}>
          <AlertTriangle size={14}/><span>{m8Blockers[b]??b}</span></div>)}</div>
        <p className="muted-copy">diagnostic_identity_verified = {String(report.diagnostic_identity_verified)}；
          write_authorized = {String(report.write_authorized)}。身份通过不证明两颗 CPU 原子停机，
          不证明 JIT 缓存或 Overlay12 可以安全写回。</p>
      </div>}
    </CardContent>
  </Card>;
}
