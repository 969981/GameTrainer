import { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { AlertTriangle, LoaderCircle } from "lucide-react";
import { AppHeader, ContextSidebar, PrimaryRail, SessionInspector } from "@/components/Shell";
import {
  GameLibraryPage, GameWorkspacePage, HistoryPage, HomePage, JournalPage,
  ModulesPage, RuntimePage, SettingsPage,
} from "@/components/Pages";
import { getCatalog, getSessionSnapshot } from "@/lib/backend";
import { findGame } from "@/lib/catalog";
import { useWorkspace } from "@/store/workspace";
import type { GameCatalog } from "@/types/catalog";

export default function App({ catalogOverride }: { catalogOverride?: GameCatalog }) {
  const view = useWorkspace((state) => state.view);
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
  });
  const catalog = catalogOverride ?? data;
  const game = catalog ? findGame(catalog, selectedGameId) : undefined;

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

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
