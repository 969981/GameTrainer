import { describe, expect, it } from "vitest";
import review from "../../../../fixtures/rs-migration-review.json";
import known from "../../../../fixtures/rs-known-rom-sha1.json";

describe("M3 guarded live capture and four-ROM BattleCrit contracts", () => {
  it("defaults to deny and only grants two exact experimental features", () => {
    expect(review.schema_version).toBe(2);
    expect(review.default_live_write_enabled).toBe(false);
    expect(review.features).toHaveLength(7);
    const enabled=review.features.filter((feature)=>feature.live_write_enabled);
    expect(enabled.map(f=>f.area).sort()).toEqual(["battlecrit","capture"]);
    expect(enabled.every(f=>f.genuine_mgba_emulator_result==="not_verified")).toBe(true);
  });
  it("requires complete ROM identity and manual consent for each enabled feature", () => {
    for(const feature of review.features.filter(f=>f.live_write_enabled)){
      expect(feature.runtime_gate).toContain("full_gdb_guest_rom_sha1");
      expect(feature.runtime_gate).toContain("confirmed_paused_guest_cpu");
      expect(feature.runtime_gate).toContain("sqlite_intent_before_write");
      expect(feature.approval).toBe("manual_backup_and_offline_single_player_confirmation");
    }
    expect(known.roms).toHaveLength(28);
    expect(known.roms.some(r=>r.sha1==="CA5E3D415C4B47353A73A616878BA833F3648B7A")).toBe(false);
  });
  it("only unlocks BattleCrit for the four source-proven builds", () => {
    const item=review.features.find(f=>f.area==="battlecrit");
    expect(item?.allowed_builds).toEqual(["AXPE_R1","AXPE_R2","AXVJ_R0","AXPJ_R0"]);
    expect(item?.runtime_gate).toContain("user_supplied_v46_template_sha256");
    const noHook=review.features.find(f=>f.area==="battle_hit_miss_ohko");
    expect(noHook?.live_write_enabled).toBe(false);
    expect(noHook?.phase).toBe("source_not_implemented");
  });
  it("keeps fishing and irreversible teleport execution disabled", () => {
    for(const area of ["fishing_assist","fishing_feebas","teleport_execution"]){
      expect(review.features.find(f=>f.area===area)?.live_write_enabled).toBe(false);
    }
    expect(review.features.find(f=>f.area==="capture")?.resource_kinds).toEqual(["rom_halfword","rom_halfword"]);
    expect(review.features.find(f=>f.area==="teleport_catalog")?.phase).toBe("read_only_catalog");
  });
});
