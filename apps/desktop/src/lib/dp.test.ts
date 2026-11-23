import { describe, expect, it } from "vitest";
import fixture from "../../../../fixtures/m0-registry.json";
import { canProbe, dpProfiles } from "./dp";

describe("Diamond and Pearl source-backed v0.8.14 profiles",()=>{
  it("includes exactly ten Western Code/Rev/Header6C triples",()=>{
    expect(dpProfiles).toHaveLength(10);
    expect(new Set(dpProfiles.map(p=>p.game_code+":"+p.revision+":"+p.header6c)).size).toBe(10);
    expect(dpProfiles.filter(p=>p.game==="Diamond")).toHaveLength(5);
    expect(dpProfiles.filter(p=>p.game==="Pearl")).toHaveLength(5);
    expect(dpProfiles.every(p=>p.arm9_sha256.length===64&&p.overlay06_sha256.length===64)).toBe(true);
    expect(dpProfiles.every(p=>!p.write_allowed&&!p.guest_exec_runtime_verified)).toBe(true);
  });
  it("does not pass ARM9/Overlay06 component fingerprints off as full ROM hashes",()=>{
    expect(dpProfiles.filter(p=>p.rom_sha256!==null).map(p=>p.game_code).sort()).toEqual(["APAE","APAI"]);
    expect(dpProfiles.filter(p=>p.battle_static_audit).map(p=>p.game_code).sort()).toEqual(["APAE","APAI"]);
    expect(dpProfiles.find(p=>p.game_code==="ADAE")?.rom_sha256).toBeNull();
  });
  it("registers two distinct game adapters with eight research modules and no patch API",()=>{
    const games=fixture.games.filter(p=>p.game_id==="games.pokemon.diamond"||p.game_id==="games.pokemon.pearl");
    expect(games).toHaveLength(2);
    expect(games.every(g=>g.platform_family==="gen4"&&g.target_platform==="nds")).toBe(true);
    expect(games.every(g=>g.features.length===8&&g.features.some(f=>f.feature_id==="features.dp.guestexec"))).toBe(true);
    expect(games.every(g=>g.features.every(f=>!f.required_capabilities.includes("patch.apply")))).toBe(true);
  });
  it("rejects unsupported Japanese/Korean and wrong ports",()=>{
    expect(canProbe(3333,"APAE",13)).toBe(true);
    expect(canProbe(3333,"ADAE",13)).toBe(true);
    expect(canProbe(3333,"APAJ",6)).toBe(false);
    expect(canProbe(3333,"APAK",0)).toBe(false);
    expect(canProbe(0,"APAE",13)).toBe(false);
    expect(canProbe(3333.5,"APAE",13)).toBe(false);
  });
});
