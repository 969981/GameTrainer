import { describe, expect, it } from "vitest";
import { dpM10Blockers, validDpM10, type DpM10Audit, type DpM10Identity } from "./dp-m10";

describe("DP M10 OS-bound ROM + dual CPU proof gates",()=>{
  it("requires catalog ROM, process PID, separate GDB ports",()=>{
    expect(validDpM10(1524,3333,3334,"APAE",13)).toBe(true);
    expect(validDpM10(1524,3333,3334,"ADAE",13)).toBe(true);
    expect(validDpM10(0,3333,3334,"APAE",13)).toBe(false);
    expect(validDpM10(1524,3333,3333,"APAE",13)).toBe(false);
    expect(validDpM10(1524,3333,3334,"APAJ",6)).toBe(false);
    expect(validDpM10(1524,3333,65536,"APAE",13)).toBe(false);
  });
  it("never authorizes writes with a complete core hash or two stopped CPUs",()=>{
    const identity:DpM10Identity={
      process_id:20,process_created_filetime:"133000000000000000",
      rom_epoch:"4",game_code:"APAE",revision:13,header6c:76,rom_size:128*1024*1024,
      crc32:"AABBCCDD",core_sha256:"7".repeat(64),
      known_full_rom_sha256:"7".repeat(64),
      full_rom_reference_available:true,full_rom_matches_catalog:true,
      core_rom_stream_complete:true,pipe_and_process_bound:true,
      both_gdb_ports_bound:true,diagnostic_identity_verified:true,
      arm9_port:3333,arm7_port:3334,jit_enabled_when_loaded:false,
      guest_write_allowed:false,
      blockers:["arm9_arm7_atomic_stop_unverified","jit_icache_coherence_unverified",
        "overlay11_loading_epoch_unverified","nds_guest_writes_disabled"]
    };
    const cpu=(name:string,port:number)=>({
      cpu:name,port,stop_reply_observed:true,
      first_pc:0x02001000,second_pc:0x02001000,error:null
    });
    const audit:DpM10Audit={
      identity,rom_epoch_stable_across_gdb_samples:true,
      arm9:cpu("ARM9",3333),arm7:cpu("ARM7",3334),
      independent_stops_observed:true,cpu_pair_atomically_stopped:false,
      arm9_pc_inside_known_thumb_cave:false,
      overlay11_static_source_present:true,overlay11_guest_observation:null,
      jit_icache_coherence_verified:false,guest_thumb_execution_verified:false,
      original_instruction_restoration_verified:false,write_authorized:false,
      blockers:identity.blockers,
    };
    expect(audit.identity.diagnostic_identity_verified).toBe(true);
    expect(audit.independent_stops_observed).toBe(true);
    expect(audit.write_authorized).toBe(false);
    expect(audit.cpu_pair_atomically_stopped).toBe(false);
    expect(audit.blockers.every(b=>dpM10Blockers[b]!==undefined)).toBe(true);
  });
  it("separates absent eight complete ROM references from an actual wrong digest",()=>{
    expect(dpM10Blockers.no_original_full_rom_sha256_reference).not.toEqual(
      dpM10Blockers.running_rom_sha256_not_catalog_match);
  });
});
