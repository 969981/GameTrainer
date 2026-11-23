import { Activity, ArrowRight, BookOpenText, Check, Cpu, FileWarning, Gamepad2, Layers3, Library, MonitorCog, SearchX, ShieldAlert, Sparkles } from "lucide-react";
import { useState } from "react";
import { M3LivePanel } from "@/components/M3LivePanel";
import { FrlgLivePanel } from "@/components/FrlgLivePanel";
import { HgssPanel } from "@/components/HgssPanel";
import { DiamondPearlPanel } from "@/components/DiamondPearlPanel";
import { DiamondPearlM10IdentityPanel } from "@/components/DiamondPearlM10IdentityPanel";
import { HgssM6ValidationPanel } from "@/components/HgssM6ValidationPanel";
import { HgssM7SafetyPanel } from "@/components/HgssM7SafetyPanel";
import { HgssM8RomIdentityPanel } from "@/components/HgssM8RomIdentityPanel";
import { EmeraldRngLivePanel, MgbaConnection, RubySapphireLivePanel, RubySapphireProfileCatalog } from "@/components/MgbaConnection";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { GameIcon } from "@/components/icons";
import { filterGames, uniqueModules } from "@/lib/catalog";
import { useWorkspace } from "@/store/workspace";
import { PLATFORM_LABEL, PLATFORMS, type FeatureDescriptor, type GameCatalog, type GameDescriptor } from "@/types/catalog";

export function StatusNotice({ title, message }: { title: string; message: string }) {
  return <div className="status-notice" role="status"><ShieldAlert size={18} /><div><strong>{title}</strong><p>{message}</p></div></div>;
}
function StatTile({ caption, value, icon: Icon }: { caption: string; value: string | number; icon: typeof Activity }) {
  return <Card className="stat-tile"><div className="stat-tile-label"><span>{caption}</span><Icon size={18} /></div><strong>{value}</strong></Card>;
}
function GameCard({ game }: { game: GameDescriptor }) {
  const selectGame = useWorkspace((state) => state.selectGame);
  return <button type="button" className="game-card" aria-label={"打开 " + game.name} onClick={() => selectGame(game.game_id)}>
    <div className="game-card-top"><span className="game-art"><GameIcon name={game.icon} size={30} /></span><Badge>MOCK</Badge></div>
    <div className="game-card-meta">{PLATFORM_LABEL[game.platform_family]}</div><h3>{game.name}</h3><p>{game.summary}</p>
    <div className="game-card-footer"><span>{game.features.length} 个功能模块</span><ArrowRight size={16} /></div>
  </button>;
}
export function HomePage({ catalog }: { catalog: GameCatalog }) {
  const setView = useWorkspace((state) => state.selectView);
  return <div className="page-stack">
    <div className="page-heading"><div><div className="eyebrow">OVERVIEW / M0</div><h1>你的游戏工作台</h1><p>不同游戏、运行时与功能模块，通过统一 Registry 管理。</p></div><Button onClick={() => setView("library")}>浏览游戏库 <ArrowRight size={15} /></Button></div>
    <StatusNotice title="M0 · Mock 模式" message="当前仅展示可扩展的游戏目录和动态 UI，没有连接任何真实游戏。" />
    <div className="stats-grid"><StatTile caption="游戏适配器" value={catalog.games.length} icon={Gamepad2} /><StatTile caption="Runtime 候选" value={catalog.runtimes.length} icon={Cpu} /><StatTile caption="功能描述符" value={catalog.games.reduce((sum, game) => sum + game.features.length, 0)} icon={Layers3} /></div>
    <div className="section-row"><h2>最近项目</h2><span>共享 Mock Registry</span></div><div className="game-grid">{catalog.games.slice(0, 3).map((game) => <GameCard key={game.game_id} game={game} />)}</div>
  </div>;
}
export function GameLibraryPage({ catalog }: { catalog: GameCatalog }) {
  const platform = useWorkspace((state) => state.platform);
  const search = useWorkspace((state) => state.search);
  const setPlatform = useWorkspace((state) => state.filterPlatform);
  const games = filterGames(catalog, platform, search);
  return <div className="page-stack"><div className="page-heading"><div><div className="eyebrow">GAME LIBRARY</div><h1>游戏库</h1><p>按世代、平台或游戏名浏览。全部内容为设计用 Mock。</p></div><Badge><Library size={14} /> {catalog.games.length} GAMES</Badge></div>
    <div className="filter-row" role="group" aria-label="世代筛选">{PLATFORMS.map((item) => <button key={item.id} type="button" className={"filter-chip" + (platform === item.id ? " active" : "")} aria-pressed={platform === item.id} onClick={() => setPlatform(item.id)}>{item.label}</button>)}</div>
    <div className="section-row"><h2>{platform === "all" ? "全部游戏" : PLATFORMS.find((item) => item.id === platform)?.label}</h2><span>显示 {games.length} 个结果</span></div>
    {games.length ? <div className="game-grid">{games.map((game) => <GameCard key={game.game_id} game={game} />)}</div>
      : <div className="empty-state"><SearchX size={34} /><h3>没有符合条件的游戏</h3><p>尝试其他平台或搜索词。</p></div>}
    <div className="library-bottom"><Sparkles size={16} /> 未来增加游戏不需要修改主导航：注册 GameDescriptor 即可。</div>
  </div>;
}
export function GameWorkspacePage({ game }: { game: GameDescriptor }) {
  const featureId = useWorkspace((state) => state.selectedFeatureId);
  const selectFeature = useWorkspace((state) => state.selectFeature);
  const selectGame = useWorkspace((state) => state.selectGame);
  const current = game.features.find((feature) => feature.feature_id === featureId);
  return <div className="page-stack">
    <div className="game-hero"><div className="hero-icon"><GameIcon name={game.icon} size={38} /></div><div className="hero-copy"><div className="eyebrow">GAME WORKSPACE · {PLATFORM_LABEL[game.platform_family]}</div><h1>{game.name}</h1><p>{game.summary}</p></div><Badge>MOCK / READ ONLY</Badge></div>
    <StatusNotice title="未连接 · 只读演示" message="尚无已验证的游戏会话和 Profile，所有修改操作均不可执行。" />
    {current ? <FeatureDetail game={game} feature={current} onBack={() => selectGame(game.game_id)} /> : <>
      <div className="stats-grid"><StatTile caption="注册功能" value={game.features.length} icon={Layers3} /><StatTile caption="Runtime 候选" value={game.runtime_candidates.length} icon={Cpu} /><StatTile caption="可执行修改" value={0} icon={ShieldAlert} /></div>
      <div className="section-row"><h2>功能模块</h2><span>来自 {game.game_id}</span></div>
      <div className="feature-list">{game.features.map((feature) => <button type="button" key={feature.feature_id} className="feature-card" onClick={() => selectFeature(feature.feature_id)} aria-label={"查看模块 " + feature.name}><span className="feature-icon"><GameIcon name={feature.icon} /></span><span className="feature-body"><strong>{feature.name}</strong><small>{feature.description}</small></span><Badge>规划中</Badge><ArrowRight size={16} /></button>)}</div>
      <Card><CardHeader><h3>候选 Runtime</h3></CardHeader><CardContent><div className="runtime-chips">{game.runtime_candidates.map((runtime) => <code key={runtime}>{runtime}</code>)}</div><p className="muted-copy">候选不代表已安装、已连接或已通过版本验证。</p></CardContent></Card>
    </>}
  </div>;
}
function FeatureDetail({ game, feature, onBack }: { game: GameDescriptor; feature: FeatureDescriptor; onBack: () => void }) {
  return <><div className="feature-detail-heading"><Button variant="ghost" size="sm" onClick={onBack}>← 返回游戏总览</Button><Badge>FEATURE · {feature.status.toUpperCase()}</Badge></div>
    <Card><CardHeader><div className="feature-detail-title"><span className="feature-icon"><GameIcon name={feature.icon} size={25} /></span><div><h2>{feature.name}</h2><p>{feature.description}</p></div></div></CardHeader><CardContent><div className="feature-metadata"><span>Game ID</span><code>{game.game_id}</code><span>Feature ID</span><code>{feature.feature_id}</code><span>Capabilities</span><code>{feature.required_capabilities.join(", ") || "无"}</code></div></CardContent></Card>
    {feature.feature_id === "features.gen3.rng" && game.game_id === "games.pokemon.emerald" && <EmeraldRngLivePanel />}
    {feature.feature_id === "features.gen3.rng" &&
      (game.game_id === "games.pokemon.ruby" || game.game_id === "games.pokemon.sapphire") &&
      <RubySapphireLivePanel gameId={game.game_id} />}
    {feature.feature_id === "features.hgss.rng" && <HgssPanel />}
    <Card><CardHeader><h3>参数与预览</h3></CardHeader><CardContent>
      {feature.parameter_schema.length ? <div className="parameter-fields">{feature.parameter_schema.map((field) => <label className="parameter-field" key={field.id}><span>{field.label}</span><input disabled aria-label={field.label} placeholder={field.placeholder} /><small>仅示意，不能修改游戏</small></label>)}</div> : <p className="muted-copy">尚无可执行参数，等待正式 FeaturePack 实现。</p>}
      <div className="preview-placeholder"><FileWarning size={20} /><div><strong>修改预览不可用</strong><p>M2 才会实现后端 preview → apply → restore 与审计。</p></div></div>
      <Button disabled>预览修改（M2）</Button>
    </CardContent></Card>
  </>;
}
export function ModulesPage({ catalog }: { catalog: GameCatalog }) {
  const modules = uniqueModules(catalog);
  return <div className="page-stack"><div className="page-heading"><div><div className="eyebrow">EXTENSION CENTER</div><h1>模块中心</h1><p>通过 FeatureDescriptor 自动汇总，不加载任何外部 DLL。</p></div><Badge>设计阶段</Badge></div>
    <div className="stats-grid"><StatTile caption="功能 ID" value={modules.length} icon={Layers3} /><StatTile caption="覆盖游戏" value={catalog.games.length} icon={Gamepad2} /><StatTile caption="可执行" value={0} icon={ShieldAlert} /></div>
    <StatusNotice title="第三方插件尚未启用" message="当前是编译期 Mock 模块描述；没有可执行插件、Hook 或注入。" />
    <div className="feature-list">{modules.map((item) => <div className="module-row" key={item.id}><span className="feature-icon"><Layers3 size={20} /></span><div><strong>{item.name}</strong><small>{item.id}</small></div><Badge>{item.count} 游戏</Badge></div>)}</div>
  </div>;
}
export function RuntimePage({ catalog }: { catalog: GameCatalog }) {
  return <div className="page-stack"><div className="page-heading"><div><div className="eyebrow">RUNTIME MANAGER</div><h1>运行时管理</h1><p>未来每类模拟器或原生进程分别实现独立 Provider。</p></div><Badge>全部未连接</Badge></div>
    <StatusNotice title="Runtime 状态说明" message="Windows 桌面版支持 mGBA 外部只读连接；M3 使用独立 GDB 调试会话的完整 ROM SHA1 和显式安全确认。" />
    <MgbaConnection />
    <RubySapphireProfileCatalog />
    <M3LivePanel />
    <FrlgLivePanel />
    <HgssPanel />
    <DiamondPearlPanel />
    <DiamondPearlM10IdentityPanel />
    <HgssM6ValidationPanel />
    <HgssM7SafetyPanel />
    <HgssM8RomIdentityPanel />
    <div className="runtime-list">{catalog.runtimes.map((runtime) => <Card key={runtime.runtime_id}><CardContent className="runtime-row"><span className="feature-icon"><MonitorCog size={22} /></span><div><strong>{runtime.name}</strong><small>{runtime.description}</small><code>{runtime.runtime_id}</code></div><Badge>PLANNED</Badge></CardContent></Card>)}</div>
  </div>;
}
const STARTER_JOURNAL = ["# GameTrainer 开发草稿", "", "## M0 任务", "- [ ] Rust Registry 与五游戏平台", "- [ ] React 动态模块导航", "- [ ] 第六款游戏扩展测试", "", "## 技术结论", "Runtime、Game、Feature、UI Schema 分离。", "当前仅为 Mock，未连接任何真实游戏。", "", "> 仅临时草稿，尚未保存到 Obsidian。"].join("\n");
export function JournalPage() {
  const [draft, setDraft] = useState(STARTER_JOURNAL);
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState(false);
  async function copy() {
    try { await navigator.clipboard.writeText(draft); setCopied(true); setError(false); }
    catch { setError(true); }
  }
  return <div className="page-stack"><div className="page-heading"><div><div className="eyebrow">DEVELOPER JOURNAL</div><h1>开发日记</h1><p>临时草稿与真实游戏操作历史严格分离。</p></div><Button variant="outline" onClick={copy}>{copied ? <Check size={16} /> : <BookOpenText size={16} />}{copied ? "已复制" : "复制 Markdown"}</Button></div>
    <StatusNotice title="临时草稿" message="离开页面可能丢失修改；M0 不会自动保存或覆盖个人 Obsidian Vault。" />
    {error && <p className="error-text" role="alert">剪贴板不可用，请手动复制。</p>}
    <label htmlFor="journal-editor" className="journal-label">Markdown 内容</label>
    <textarea id="journal-editor" className="journal-editor" spellCheck={false} value={draft} onChange={(event) => { setDraft(event.target.value); setCopied(false); }} />
  </div>;
}
export function SettingsPage() {
  const theme = useWorkspace((state) => state.theme);
  const setTheme = useWorkspace((state) => state.setTheme);
  return <div className="page-stack"><div className="page-heading"><div><div className="eyebrow">PREFERENCES</div><h1>设置</h1><p>界面和工作台偏好。</p></div></div>
    <Card><CardHeader><h3>外观</h3></CardHeader><CardContent><div className="settings-row"><div><strong>颜色主题</strong><p>统一 Dark Control / Light 配色</p></div><div className="theme-buttons"><Button variant={theme === "dark" ? "default" : "outline"} onClick={() => setTheme("dark")}>深色</Button><Button variant={theme === "light" ? "default" : "outline"} onClick={() => setTheme("light")}>浅色</Button></div></div></CardContent></Card>
    <Card><CardHeader><h3>默认权限</h3></CardHeader><CardContent><p className="muted-copy">M0 不提供任何修改权限。未来写入必须由可信 Rust Host 独立校验。</p><Badge>READ ONLY</Badge></CardContent></Card>
  </div>;
}
