import {
  Activity, BookOpenText, ChevronDown, ClipboardList, Command, Cpu,
  Gamepad2, LayoutDashboard, Layers3, Library, Moon, PanelRightClose,
  PanelRightOpen, Puzzle, Search, Settings2, ShieldAlert, Sun,
} from "lucide-react";
import { useEffect, useRef } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { GameIcon } from "@/components/icons";
import { groupedFeatures } from "@/lib/catalog";
import { useWorkspace } from "@/store/workspace";
import { PLATFORM_LABEL, PLATFORMS, type GameCatalog, type GameDescriptor, type SessionSnapshot, type ViewId } from "@/types/catalog";

const PRIMARY_ITEMS: { id: ViewId; label: string; icon: typeof Library }[] = [
  { id: "home", label: "总览", icon: LayoutDashboard },
  { id: "library", label: "游戏库", icon: Library },
  { id: "modules", label: "模块中心", icon: Puzzle },
  { id: "runtime", label: "运行时", icon: Cpu },
  { id: "history", label: "操作历史", icon: ClipboardList },
  { id: "journal", label: "开发日记", icon: BookOpenText },
  { id: "settings", label: "设置", icon: Settings2 },
];

export function PrimaryRail() {
  const view = useWorkspace((state) => state.view);
  const setView = useWorkspace((state) => state.selectView);
  return (
    <nav className="primary-rail" aria-label="主导航">
      <div className="brand-mark" aria-label="GameTrainer Studio"><Gamepad2 size={24} /></div>
      <div className="rail-separator" />
      {PRIMARY_ITEMS.map(({ id, label, icon: Icon }) => (
        <button key={id} type="button" title={label} aria-label={label}
          className={"rail-action" + (view === id ? " active" : "")}
          aria-current={view === id ? "page" : undefined}
          onClick={() => setView(id)}>
          <Icon size={20} strokeWidth={1.8} />
          <span className="rail-text">{label}</span>
        </button>
      ))}
      <div className="rail-spacer" />
      <div className="rail-footer" title="M0 · Mock 预览">M0</div>
    </nav>
  );
}

export function ContextSidebar({ game, catalog }: { game?: GameDescriptor; catalog: GameCatalog }) {
  const view = useWorkspace((state) => state.view);
  const platform = useWorkspace((state) => state.platform);
  const setPlatform = useWorkspace((state) => state.filterPlatform);
  const selectedFeatureId = useWorkspace((state) => state.selectedFeatureId);
  const selectFeature = useWorkspace((state) => state.selectFeature);
  const selectView = useWorkspace((state) => state.selectView);
  const selectGame = useWorkspace((state) => state.selectGame);
  const groups = game && view === "workspace" ? groupedFeatures(game) : undefined;
  return (
    <aside className="context-sidebar" aria-label="上下文导航">
      <div className="context-heading">
        <span className="eyebrow">WORKSPACE</span>
        <h2>游戏工作台</h2>
        <p>Modular game platform</p>
      </div>
      {groups && game ? (
        <>
          <button className="game-switcher" type="button" onClick={() => selectView("library")}>
            <span className="game-switcher-icon"><GameIcon name={game.icon} size={20} /></span>
            <span className="game-switcher-text"><strong>{game.name}</strong><small>{PLATFORM_LABEL[game.platform_family]}</small></span>
            <ChevronDown size={15} />
          </button>
          <div className="sidebar-heading">当前游戏</div>
          <button type="button" className={"sidebar-link" + (!selectedFeatureId ? " active" : "")}
            onClick={() => selectGame(game.game_id)}>
            <LayoutDashboard size={16} /> 游戏总览
          </button>
          {[...groups.entries()].map(([group, features]) => (
            <section key={group} className="sidebar-group">
              <div className="sidebar-heading">{group}</div>
              {features.map((feature) => (
                <button key={feature.feature_id} type="button"
                  className={"sidebar-link" + (selectedFeatureId === feature.feature_id ? " active" : "")}
                  aria-current={selectedFeatureId === feature.feature_id ? "page" : undefined}
                  onClick={() => selectFeature(feature.feature_id)}>
                  <GameIcon name={feature.icon} size={16} />
                  <span>{feature.name}</span>
                </button>
              ))}
            </section>
          ))}
          <div className="sidebar-bottom">
            <ShieldAlert size={14} /> 当前游戏功能全部不可执行
          </div>
        </>
      ) : (
        <>
          <div className="sidebar-heading">游戏平台</div>
          {PLATFORMS.map(({ id, label, subtitle }) => {
            const count = id === "all" ? catalog.games.length :
              catalog.games.filter((item) => item.platform_family === id).length;
            return (
              <button key={id} type="button" className={"sidebar-link" + (platform === id && view === "library" ? " active" : "")}
                onClick={() => setPlatform(id)} aria-pressed={platform === id}>
                <Layers3 size={15} />
                <span className="sidebar-link-text">{label}<small>{subtitle}</small></span>
                <span className="sidebar-count">{count}</span>
              </button>
            );
          })}
          <div className="sidebar-heading">最近项目</div>
          {catalog.games.slice(0, 3).map((entry) => (
            <button key={entry.game_id} type="button" className="sidebar-link"
              onClick={() => selectGame(entry.game_id)}>
              <GameIcon name={entry.icon} size={16} />
              <span className="sidebar-truncate">{entry.name}</span>
            </button>
          ))}
          <div className="sidebar-bottom">
            <ShieldAlert size={14} /> 所有游戏均为 Mock / Planned
          </div>
        </>
      )}
    </aside>
  );
}

export function AppHeader({ game }: { game?: GameDescriptor }) {
  const search = useWorkspace((state) => state.search);
  const setSearch = useWorkspace((state) => state.setSearch);
  const view = useWorkspace((state) => state.view);
  const theme = useWorkspace((state) => state.theme);
  const setTheme = useWorkspace((state) => state.setTheme);
  const toggleInspector = useWorkspace((state) => state.toggleInspector);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        searchRef.current?.focus();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <header className="app-header">
      <div className="breadcrumb"><span>GameTrainer Studio</span><span className="crumb-separator">/</span><strong>{game && view === "workspace" ? game.name : PRIMARY_ITEMS.find((item) => item.id === view)?.label}</strong></div>
      <div className="header-actions">
        <label className="search-control">
          <Search size={16} aria-hidden="true" />
          <input ref={searchRef} aria-label="搜索游戏" placeholder="搜索游戏 / 模块…" value={search} onChange={(event) => setSearch(event.target.value)} />
          <span className="keyboard-hint"><Command size={12} /> K</span>
        </label>
        <Badge className="demo-badge"><Activity size={11} /> M0 MOCK</Badge>
        <Button variant="ghost" size="icon" title="切换主题" aria-label="切换主题"
          onClick={() => setTheme(theme === "dark" ? "light" : "dark")}>{theme === "dark" ? <Sun size={18} /> : <Moon size={18} />}</Button>
        <Button variant="outline" size="icon" title="会话检查器" aria-label="切换会话检查器" onClick={toggleInspector}>
          <PanelRightOpen className="when-closed" size={18} /><PanelRightClose className="when-open" size={18} />
        </Button>
      </div>
    </header>
  );
}

export function SessionInspector({ session, game }: { session?: SessionSnapshot; game?: GameDescriptor }) {
  const inspectorOpen = useWorkspace((state) => state.inspectorOpen);
  const toggleInspector = useWorkspace((state) => state.toggleInspector);
  const attached = session?.state === "attached_read_only";
  return (
    <aside className={"inspector " + (inspectorOpen ? "inspector-open" : "")} aria-label="会话检查器">
      <div className="inspector-title"><div><span className="eyebrow">SESSION INSPECTOR</span><h3>连接与诊断</h3></div>
        <button type="button" className="inspector-close" onClick={toggleInspector} aria-label="关闭会话检查器">×</button>
      </div>
      <div className="inspector-status"><span className="status-dot" /><span>{attached ? "已连接 · 只读" : "未连接"}</span><Badge>只读</Badge></div>
      <dl className="detail-list">
        <div><dt>目标游戏</dt><dd>{game?.name ?? "未选择"}</dd></div>
        <div><dt>Session ID</dt><dd>{session?.session_id ?? "—"}</dd></div>
        <div><dt>Runtime</dt><dd>{session?.runtime_id ?? "未启动"}</dd></div>
        <div><dt>ROM / EXE</dt><dd>未验证</dd></div>
        <div><dt>Profile</dt><dd>{session?.profile_id ?? "无"}</dd></div>
        <div><dt>写入能力</dt><dd className="text-warning">禁用</dd></div>
      </dl>
      <div className="inspector-message"><ShieldAlert size={18} />
        <span>{session?.message ?? "未连接后端。当前仅为 Mock 目录。"}</span>
      </div>
      <div className="sidebar-heading">只读连接约束</div>
      <div className="inspector-note">M1 只支持 mGBA Emerald 只读探测和 Seed 观察；无写入、Patch 或 Hook 命令。所有未知 ROM 与内存映射均拒绝连接。</div>
    </aside>
  );
}
