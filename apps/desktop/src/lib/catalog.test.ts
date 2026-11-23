import { describe, expect, it } from "vitest";
import fixture from "../../../../fixtures/m0-registry.json";
import { filterGames, findGame, groupedFeatures, uniqueModules } from "./catalog";
import type { GameCatalog, GameDescriptor } from "@/types/catalog";

const catalog = fixture as unknown as GameCatalog;

describe("M0 shared catalog selectors", () => {
  it("exposes all five platform families", () => {
    expect(catalog.games.map((game) => game.platform_family).sort())
      .toEqual(["gen3", "gen4", "gen5", "gen6", "pc"]);
    expect(catalog.read_only).toBe(true);
    expect(catalog.source).toBe("mock");
  });

  it("filters by platform and searches by game or ID", () => {
    expect(filterGames(catalog, "gen6", "")).toHaveLength(1);
    expect(filterGames(catalog, "pc", "")[0].game_id).toBe("games.native.template");
    expect(filterGames(catalog, "all", "platinum")).toHaveLength(1);
    expect(filterGames(catalog, "gen5", "platinum")).toHaveLength(0);
  });

  it("derives feature groups from descriptors, not hardcoded game names", () => {
    const game = catalog.games[0];
    expect([...groupedFeatures(game).values()].flat()).toHaveLength(game.features.length);
    expect(findGame(catalog, game.game_id)?.features[0].feature_id).toBe("features.gen3.rng");
    expect(uniqueModules(catalog).length).toBeGreaterThanOrEqual(10);
  });

  it("handles a sixth game without changing navigation selectors", () => {
    const sixth: GameDescriptor = {
      ...catalog.games[0],
      game_id: "games.pokemon.firered",
      name: "Pokémon FireRed",
      features: [{
        ...catalog.games[0].features[0],
        feature_id: "features.firered.seed",
        name: "FireRed Seed",
      }],
    };
    const extended = { ...catalog, games: [...catalog.games, sixth] };
    expect(filterGames(extended, "gen3", "")).toHaveLength(2);
    expect(findGame(extended, sixth.game_id)?.features[0].name).toBe("FireRed Seed");
    expect(uniqueModules(extended).some((module) => module.id === "features.firered.seed")).toBe(true);
  });
});
