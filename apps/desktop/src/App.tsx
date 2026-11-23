import { useEffect, useRef } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { AlertTriangle, LoaderCircle } from "lucide-react";
import { AppHeader, ContextSidebar, PrimaryRail, SessionInspector } from "@/components/Shell";
import {
  GameLibraryPage, GameWorkspacePage, HistoryPage, HomePage, JournalPage,
  ModulesPage, RuntimePage, SettingsPage,
} from "@/components/Pages";
import { getCatalog, getSessionSnapshot, isTauriDesktop } from "@/lib/backend";
import { disconnectEmerald } from "@/lib/mgba";
import { findGame } from "@/lib/catalog";
import { useWorkspace } from "@/store/workspace";
import type { GameCatalog } from "@/types/catalog";

export default function App({ catalogOverride }: { catalogOverride?: GameCatalog }) {
  const view = useWorkspace((state) => state.view);
  const queryClient = useQueryClient();
  const detaching = useRef(false);
  const selectedGameId = useWorkspace((state) => state.selectedGameId);
  const theme = useWorkspace((state) => state.theme);
  const { data, isPending, error } = useQuery({
    queryKey: ["catalog", "m0"],
    queryFn: getCatalog,
    enabled: !catalogOverride,
    retry: false,
  });
  const sessionQuery = useQuery({
    queryKey: ["session", "m0"],
    queryFn: getSessionSnapshot,
    retry: false,
    refetchInterval: 2000,
  });
  const catalog = catalogOverride ?? data;
  const game = catalog ? findGame(catalog, selectedGameId) : undefined;

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    // A runtime belongs to ONE game session. Selecting another game detaches
    // the Emerald runtime rather than allowing an old process mapping to linger.
    if (!isTauriDesktop()) return;
    const session = sessionQuery.data;
    const mismatched = selectedGameId &&
      session?.state === "attached_read_only" &&
      session.game_id !== selectedGameId;
    if (!mismatched) {
      if (session?.state !== "attached_read_only") detaching.current = false;
      return;
    }
    if (detaching.current) return;
    detaching.current = true;
    void disconnectEmerald()
      .then(() => queryClient.invalidateQueries({ queryKey: ["session"] }))
      .finally(() => { detaching.current = false; });
  }, [selectedGameId, sessionQuery.data, queryClient]);


  if (!catalog && isPending) {
    return <div className="startup-state" role="status"><LoaderCircle className="spin" size={24} /> 正在读取 Mock 游戏目录…</div>;
  }
  if (!catalog || error) {
    return <div className="startup-state error" role="alert"><AlertTriangle size={24} />
      <div><strong>无法加载游戏注册中心</strong><p>{String(error ?? "Registry 未提供有效游戏清单")}</p>
        <small>若在 Tauri 中出错，不会自动伪造浏览器数据。</small></div>
    </div>;
  }
  const content = (() => {
    switch (view) {
      case "home": return <HomePage catalog={catalog} />;
      case "library": return <GameLibraryPage catalog={catalog} />;
      case "workspace": return game
        ? <GameWorkspacePage game={game} />
        : <GameLibraryPage catalog={catalog} />;
      case "modules": return <ModulesPage catalog={catalog} />;
      case "runtime": return <RuntimePage catalog={catalog} />;
      case "history": return <HistoryPage />;
      case "journal": return <JournalPage />;
      case "settings": return <SettingsPage />;
    }
  })();

  return <div className="app-layout">
    <PrimaryRail />
    <ContextSidebar catalog={catalog} game={game} />
    <div className="main-column">
      <AppHeader game={game} />
      <main id="main-content" className="page-content">{content}</main>
      <footer className="app-footer"><span>GameTrainer Studio · M0 Mock Registry</span><span>未连接 · 禁止写入 · {catalog.games.length} Game Adapters</span></footer>
    </div>
    <SessionInspector game={game} session={sessionQuery.data} />
  </div>;
}
