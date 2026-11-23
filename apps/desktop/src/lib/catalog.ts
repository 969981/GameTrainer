import type { GameCatalog, GameDescriptor, PlatformFamily } from "@/types/catalog";

export function filterGames(
  catalog: GameCatalog,
  platform: "all" | PlatformFamily,
  search: string,
): GameDescriptor[] {
  const term = search.trim().toLocaleLowerCase();
  return catalog.games.filter((game) => {
    const matchesPlatform = platform === "all" || game.platform_family === platform;
    const matchesSearch = !term
      || [game.name, game.game_id, game.target_platform, game.summary]
        .some((value) => value.toLocaleLowerCase().includes(term));
    return matchesPlatform && matchesSearch;
  });
}

export function findGame(catalog: GameCatalog, gameId: string | null): GameDescriptor | undefined {
  return catalog.games.find((game) => game.game_id === gameId);
}

export function groupedFeatures(game: GameDescriptor): Map<string, GameDescriptor["features"]> {
  const groups = new Map<string, GameDescriptor["features"]>();
  for (const feature of game.features) {
    groups.set(feature.group, [...(groups.get(feature.group) ?? []), feature]);
  }
  return groups;
}

export function uniqueModules(catalog: GameCatalog) {
  const result = new Map<string, { id: string; name: string; count: number; group: string }>();
  for (const game of catalog.games) {
    for (const feature of game.features) {
      const previous = result.get(feature.feature_id);
      result.set(feature.feature_id, {
        id: feature.feature_id,
        name: feature.name,
        group: feature.group,
        count: (previous?.count ?? 0) + 1,
      });
    }
  }
  return [...result.values()];
}
