import { describe, expect, it } from "vitest";
import fixture from "../../../../fixtures/m0-registry.json";
import { hgssProfiles } from "./hgss";

describe("HGSS M5 read-only profiles", () => {
  it("registers 14 profiles with exactly 12 version-specific static audits", () => {
    expect(hgssProfiles).toHaveLength(14);
    expect(hgssProfiles.filter(p => p.static_audit_1451)).toHaveLength(12);
    expect(new Set(hgssProfiles.map(p => p.game_code + ":" + p.revision)).size).toBe(14);
    expect(hgssProfiles.every(p => p.write_allowed === false)).toBe(true);
  });
  it("does not confuse USA Rev0 and EU-English Rev10", () => {
    for (const code of ["IPKE", "IPGE"]) {
      const us = hgssProfiles.find(p => p.game_code === code && p.revision === 0);
      const eu = hgssProfiles.find(p => p.game_code === code && p.revision === 10);
      expect(us?.sha1).not.toBe(eu?.sha1);
      expect(us?.static_audit_1451).toBe(false);
      expect(eu?.static_audit_1451).toBe(true);
    }
  });
  it("adds two Gen4 adapters without executable write capability", () => {
    const games = fixture.games.filter(g => g.game_id.endsWith("heartgold") || g.game_id.endsWith("soulsilver"));
    expect(games).toHaveLength(2);
    expect(games.every(g => g.platform_family === "gen4")).toBe(true);
    expect(games.every(g => g.features.some(f => f.feature_id === "features.hgss.rng"))).toBe(true);
    expect(games.every(g => g.features.every(f => !f.required_capabilities.includes("patch.apply"))).toBe(true);
  });
});
