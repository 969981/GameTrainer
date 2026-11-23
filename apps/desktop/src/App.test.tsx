import { beforeEach, describe, expect, it } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import fixture from "../../../fixtures/m0-registry.json";
import App from "./App";
import { useWorkspace } from "./store/workspace";
import type { GameCatalog, GameDescriptor } from "./types/catalog";

const baseCatalog = fixture as unknown as GameCatalog;

function mount(catalog: GameCatalog = baseCatalog) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(<QueryClientProvider client={client}><App catalogOverride={catalog} /></QueryClientProvider>);
}

beforeEach(() => {
  useWorkspace.setState({
    view: "library", platform: "all", selectedGameId: null, selectedFeatureId: null,
    search: "", inspectorOpen: false, theme: "dark",
  });
});

describe("M0 dynamic AppShell", () => {
  it("lists the five Mock games and never claims connected status", () => {
    mount();
    expect(screen.getByRole("button", { name: "打开 Pokémon Emerald" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "打开 Pokémon Platinum" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "打开 Pokémon Black 2" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "打开 Pokémon X / Y" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "打开 Windows Native · Template" })).toBeInTheDocument();
    expect(screen.getByText("未连接")).toBeInTheDocument();
  });

  it("switches a Gen6 game without leaking Emerald-only features", () => {
    mount();
    fireEvent.click(screen.getByRole("button", { name: "打开 Pokémon Emerald" }));
    expect(screen.getByRole("button", { name: "RNG / Seed" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "RNG / Seed" }));
    expect(useWorkspace.getState().selectedFeatureId).toBe("features.gen3.rng");
    fireEvent.click(screen.getByRole("button", { name: "游戏库" }));
    fireEvent.click(screen.getByRole("button", { name: "打开 Pokémon X / Y" }));
    expect(screen.getByRole("button", { name: "队伍与盒子" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "RNG / Seed" })).not.toBeInTheDocument();
    expect(useWorkspace.getState().selectedFeatureId).toBeNull();
  });

  it("filters platform and shows a clear empty state", () => {
    mount();
    fireEvent.click(screen.getByRole("button", { name: "Gen 6" }));
    expect(screen.getByRole("button", { name: "打开 Pokémon X / Y" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "打开 Pokémon Emerald" })).not.toBeInTheDocument();
    fireEvent.change(screen.getByRole("textbox", { name: "搜索游戏" }), { target: { value: "not-an-existing-game" } });
    expect(screen.getByText("没有符合条件的游戏")).toBeInTheDocument();
  });

  it("renders sixth game and its feature without AppShell changes", () => {
    const sixth: GameDescriptor = {
      ...baseCatalog.games[0], game_id: "games.pokemon.firered", name: "Pokémon FireRed",
      features: [{ ...baseCatalog.games[0].features[0], feature_id: "features.firered.seed", name: "FireRed Seed" }],
    };
    mount({ ...baseCatalog, games: [...baseCatalog.games, sixth] });
    fireEvent.click(screen.getByRole("button", { name: "打开 Pokémon FireRed" }));
    expect(screen.getByRole("button", { name: "FireRed Seed" })).toBeInTheDocument();
    expect(screen.getByText(/6 Game Adapters/)).toBeInTheDocument();
  });

  it("never produces live RNG values in browser Mock", () => {
    mount();
    fireEvent.click(screen.getByRole("button", { name: "打开 Pokémon Emerald" }));
    fireEvent.click(screen.getByRole("button", { name: "RNG / Seed" }));
    expect(screen.getByText(/尚未连接经过身份验证的 Emerald 只读会话/)).toBeInTheDocument();
    expect(screen.queryByText(/0x1234ABCD/)).not.toBeInTheDocument();
  });

  it("keeps feature mutation disabled even after selecting a Mock feature", () => {
    mount();
    fireEvent.click(screen.getByRole("button", { name: "打开 Pokémon Emerald" }));
    fireEvent.click(screen.getByRole("button", { name: "RNG / Seed" }));
    expect(screen.getByRole("button", { name: "预览修改（M2）" })).toBeDisabled();
    expect(screen.getByRole("textbox", { name: "目标 Seed" })).toBeDisabled();
  });
});
