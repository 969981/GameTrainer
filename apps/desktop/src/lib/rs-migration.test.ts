import { describe, expect, it } from "vitest";
import review from "../../../../fixtures/rs-migration-review.json";

describe("RS 4.6 migration evidence safety gates", () => {
  it("keeps every migration action read-only or fixture-only", () => {
    expect(review.status).toBe("review_only");
    expect(review.live_write_enabled).toBe(false);
    expect(review.features).toHaveLength(7);
    expect(review.features.every((feature) => feature.live_write_enabled === false)).toBe(true);
    expect(review.features.every((feature) => feature.source_paths.length > 0)).toBe(true);
  });

  it("requires a multi-resource manager for each potential mutation", () => {
    for (const feature of review.features) {
      if (feature.resource_kinds.length > 0) {
        expect(feature.requires_multiresource_transaction).toBe(true);
      }
    }
    const capture = review.features.find((item) => item.area === "capture");
    expect(capture?.resource_kinds).toEqual(["rom_halfword", "rom_halfword"]);
    expect(capture?.required_gates).toContain("two_span_transaction");
  });

  it("treats the teleport catalog as metadata, but not teleport execution", () => {
    const list = review.features.find((item) => item.area === "teleport_catalog");
    const execute = review.features.find((item) => item.area === "teleport_execution");
    expect(list?.phase).toBe("read_only_catalog");
    expect(list?.resource_kinds).toHaveLength(0);
    expect(execute?.live_write_enabled).toBe(false);
    expect(execute?.required_gates).toContain("trusted_backend_guard");
  });

  it("never promotes the source-unverified battle features to executable", () => {
    expect(review.features.find((item) => item.area === "battlecrit")?.phase).toBe("blocked_experimental");
    expect(review.features.find((item) => item.area === "battle_hit_miss_ohko")?.phase).toBe("source_not_implemented");
    expect(review.features.find((item) => item.area === "battlecrit")?.required_gates).toContain("no_link_battles");
  });
});
