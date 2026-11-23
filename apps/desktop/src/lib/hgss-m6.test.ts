import { describe, expect, it } from "vitest";
import { m6Transition, type M6OverlayReport } from "./hgss-m6";

const base: M6OverlayReport = {
  game_code:"IPKJ", revision:0, arm9_pc:0x0223D3A6,
  gdb_stop_verified:true, field_system:0x02004000,
  process_manager:0x02005000, overlay_child:0x02006000,
  overlay_id:12, execution_state:1,
  overlay_load_addr:0x02236E00, capture_addr:0x02246B5A,
  observed_halfword:0x2801, neighbor_signature_valid:true,
  state:"original_verified", running_rom_sha1_verified:false,
  write_authorized:false,
  evidence_scope:"arm9_gdb_stopped_guest_read_only_not_full_rom_validation",
};
describe("M6 HGSS debugger evidence UI contracts", () => {
  it("only detects observed overlay entry and exit, not a patch approval", () => {
    const empty: M6OverlayReport = {...base,state:"not_loaded",overlay_id:5,observed_halfword:null};
    expect(m6Transition(empty,base)).toContain("进入");
    expect(m6Transition(base,empty)).toContain("退出");
    expect(base.write_authorized).toBe(false);
    expect(base.running_rom_sha1_verified).toBe(false);
  });
  it("detects foreign changes and does not trust stale overlay ownership", () => {
    expect(m6Transition(base,{...base,observed_halfword:0x4280})).toContain("变化");
    expect(m6Transition(base,{...base,overlay_child:0x02007000})).toContain("所有权");
  });
  it("will not compare evidence from different versions", () => {
    expect(m6Transition(base,{...base,game_code:"IPGE",revision:10})).toContain("不可复用");
  });
});
