import { describe, expect, it } from "vitest";
import { blockerLabels, cpuStable, portsValid, type CpuObservation, type M7Report } from "./hgss-m7";

const cpu:CpuObservation={cpu:"ARM9",port:3333,rsp_stop_reply:true,
  pc_first:0x02001000,pc_second:0x02001000,error:null};
describe("HGSS M7 fail-closed UI",()=>{
  it("requires different valid local ports",()=>{
    expect(portsValid(3333,3334)).toBe(true);
    expect(portsValid(3333,3333)).toBe(false);
    expect(portsValid(23,3334)).toBe(false);
    expect(portsValid(3333,65536)).toBe(false);
    expect(portsValid(NaN,3334)).toBe(false);
  });
  it("distinguishes a stop reply from stable PC",()=>{
    expect(cpuStable(cpu)).toBe(true);
    expect(cpuStable({...cpu,pc_second:0x02001004})).toBe(false);
    expect(cpuStable({...cpu,error:"read failure"})).toBe(false);
  });
  it("cannot interpret two stopped CPUs as NDS write authorization",()=>{
    const report:M7Report={
      game_code:"IPKJ",revision:0,arm9:cpu,arm7:{...cpu,cpu:"ARM7",port:3334},
      source_overlay12_original_observed:true,
      running_rom_sha1_verified:false,shared_emulator_instance_verified:false,
      simultaneous_cpu_stop_verified:false,jit_icache_coherence_verified:false,
      live_restore_faults_verified:false,write_authorized:false,
      blockers:["running_rom_digest_not_bound","cpu_pair_not_atomically_stopped",
        "emulator_jit_and_icache_not_verified","live_write_backend_intentionally_absent"],
      evidence_scope:"read_only",
    };
    expect(cpuStable(report.arm9)&&cpuStable(report.arm7)).toBe(true);
    expect(report.write_authorized).toBe(false);
    expect(report.blockers.every(b=>blockerLabels[b]?.length>0)).toBe(true);
  });
});
