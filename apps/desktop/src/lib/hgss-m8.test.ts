import {describe,expect,it} from "vitest";
import {m8Blockers,validateM8,type M8Report} from "./hgss-m8";
describe("M8 loaded-ROM binding UI contract",()=>{
 it("requires a PID and two distinct GDB ports",()=>{
  expect(validateM8(4567,3333,3334)).toBe(true);
  expect(validateM8(0,3333,3334)).toBe(false);
  expect(validateM8(4567,3333,3333)).toBe(false);
  expect(validateM8(4567,23,3334)).toBe(false);
  expect(validateM8(4567,3333,65536)).toBe(false);
 });
 it("never promotes a matching ROM into Guest writes",()=>{
  const r:M8Report={
   process_id:123,process_created_filetime:7,rom_load_epoch:2,
   code:"IPKJ",revision:0,rom_size:1048576,rom_crc32:"ABCD1234",
   rom_sha1:"a".repeat(40),catalog_sha1:"a".repeat(40),arm9_port:3333,arm7_port:3334,
   jit_enabled_when_loaded:false,core_rom_stream_complete:true,
   pipe_process_identity_verified:true,gdb_socket_pid_binding_verified:true,
   running_rom_digest_matches_catalog:true,diagnostic_identity_verified:true,
   dual_cpu_atomic_stop_verified:false,jit_cache_coherence_verified:false,
   overlay_restore_verified:false,write_authorized:false,
   blockers:["atomic_arm9_arm7_stop_not_proven","jit_instruction_cache_not_proven",
    "overlay_epoch_restore_not_proven","nds_writes_not_implemented"],
   evidence_scope:"m8_read_only",
  };
  expect(r.diagnostic_identity_verified).toBe(true);
  expect(r.write_authorized).toBe(false);
  expect(r.blockers.every(s=>m8Blockers[s]?.length>0)).toBe(true);
 });
});
