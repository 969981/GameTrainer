# GameTrainer Studio

> 多游戏、跨运行时的 Windows 桌面研究与修改工具（M0–M2 已建立，M3/M4 受控 GDB Guest 写入实验）

**当前状态：M0 自动化测试通过、M1 Emerald/mGBA 只读原型已实现、M2 Fixture-only 安全事务与 SQLite 审计已通过 CI。** Windows Tauri 的 EXE、Rust 命令单测、React/Vite 和 Chromium 响应式测试均在 CI 通过；M1 新增 mGBA 进程发现、部分 ROM 指令锚点验证、Seed/Advances 只读快照及会话失效保护。**新增 M3 受限 mGBA GDB 真实 Guest 写入代码，尚无真实 Windows+mGBA+原版 ROM 实测，不能宣称捕捉/战斗 Hook 已通过。没有反作弊绕过。**

## M6 · HGSS ARM9 GDB + Overlay12 读证据与恢复模拟验证

新增 12 版本精确静态指令证据、DeSmuME ARM9 GDB 只读 Stop/PC/Overlay12 采样、React 诊断面板/JSON 证据复制，以及 Mock 所有权恢复测试。**没有真实 ARM9 写入通道、尚未 Windows DeSmuME 端到端验收，捕捉/穿墙/战斗 Hook 均不解锁。**

[M6 验证文档](docs/m6-hgss-arm9-overlay-proof.md) · [ADR-0008](docs/decisions/0008-m6-arm9-overlay-evidence.md) · [静态证据清单](fixtures/hgss-m6-overlay12.json)。

## M5 · HeartGold / SoulSilver（HGSS v1.4.5.1）

NDS Gen4 新增心金和魂银两款游戏，包含 14 个多语言 ROM Profile、独立 Rust trainer-hgss、Windows DeSmuME 外部只读内存适配、Tauri 2 IPC、React RNG/MT 面板及本地 NDS ROM SHA1 文件核验。12 个日/欧版 Profile 有 v1.4.5.1 静态审计，两份美版 Rev0 保留为旧版 Profile。

**目前只读，未经真实 DeSmuME 联调，不能当作稳定修改器。** 不提供 NDS 捕捉/穿墙/瞬移/战斗 Hook 的实际内存写入；上游必会心、必中、一击必杀等战斗 Hook 也未得到安全验证。

[M5 操作与安全说明](docs/m5-hgss-v1451-readonly.md) · [ADR-0007](docs/decisions/0007-m5-hgss-read-only.md) · [版本 Manifest](fixtures/hgss-v1451-profiles.json)。

## M4 · FireRed / LeafGreen v2.0.4（16 零售版 Profile）

**M4 构建验证 2026-11-23**：[CI #45121396498](https://github.com/969981/GameTrainer/actions/runs/45121396498) 三组任务全部通过：Rust 57 项单元测试、前端自动化和响应式 Chromium 检查、Windows Tauri 2 check + desktop Rust test + debug EXE 编译。[Windows x64 测试构建 Artifact](https://github.com/969981/GameTrainer/actions/runs/45121396498/artifacts/14794430558)（工作流保留历史 artifact 名 `gametrainer-m0-m1-win64-debug`，内容为 M4 当前代码编译的 `game-trainer-desktop.exe`）。**真实 FireRed/LeafGreen ROM 游戏效果及 Hook 卸载仍未实机验证。**

- [M4 架构、受控 Guest 写入与实验 Hook 使用说明](docs/m4-frlg-v204-live.md) · [ADR-0006](docs/decisions/0006-m4-frlg-experimental-live.md)。
- 已接入完整 ROM SHA1 白名单 16 版：FireRed/LeafGreen 美/日 Rev0/1、德/法/意/西 Rev0。新增 Rust `trainer-frlg`、Tauri M4 独立 Session、React 真实 GDB 面板，游戏库从 7 款增至 9 款。
- 已编写实际 GDB Guest 写入：固定 ROM 原指令核对与 100% 捕捉、穿墙 Halfword 事务；v2.0.4 116B 固定 SHA256 Payload 的一次性普通野生 Seed Hook（普通 `flags=1` / Repel 修复）。
- 完整 ROM SHA1 于 connect/preview/apply/restore 复核，后端先写独立 SQLite Intent，写后回读、按所有权恢复；M3/M4 连接互斥。**不支持汉化/随机版或联机游戏，不分发原 GPL-3.0 Hook 二进制。**
- **真实 Windows+mGBA + 原版火红/叶绿 ROM 的捕捉/穿墙/Hook 实机验收尚未完成**。其它钓鱼、地图传送和游走 Hook 不会因为此版本加入而自动启用。

## M3 · Ruby/Sapphire 实验性真实写入（未实机验收）

- [M3 GDB 写入、捕捉和 BattleCrit 的使用及安全边界](docs/m3-live-gdb-experimental.md) · [ADR-0005](docs/decisions/0005-m3-gdb-guarded-live.md)。
- 通过本机 mGBA GDB RSP，只有 Guest CPU 暂停、完整 ROM SHA1 命中 [28 个源报告白名单](fixtures/rs-known-rom-sha1.json) 和逐功能原指令匹配，Rust 才允许进入预览。
- 捕捉：双 ROM Halfword 读回/恢复。BattleCrit：四 ROM、用户提供且经 SHA256 校验的 124B Thumb 模板，code cave 先写、dispatch 最后激活，恢复顺序相反。
- 用户明确确认备份及离线单机测试，SQLite 先写 Intent。**钓鱼/Feebas、传送实际 Hook 及其他未实现战斗功能仍然不开放；本阶段无真实 ROM/mGBA 端到端验收。**

**M3 自动化结果（2026-11-23）**：最新 CI [#45118655477](https://github.com/969981/GameTrainer/actions/runs/45118655477) Rust 50 项单测、React/Vitest 15 项、三尺寸 Chromium E2E、Windows Tauri check/unit tests/debug EXE 全部通过。Windows ZIP 的 Artifact 名仍保留旧的 m0-m1 标记，但其内容为最新 M3 commit 构建的 exe：[下载 Actions Artifact](https://github.com/969981/GameTrainer/actions/runs/45118655477/artifacts/14792413807)。**没有实机游戏验证。**

## 产品目标

- 支持 Pokémon Gen3（GBA）、Gen4/Gen5（NDS）、Gen6（3DS），并扩展到 Windows 原生单机游戏。
- 使用 **Rust + Tauri 2 + React + TypeScript + Vite + Tailwind CSS + shadcn/ui** 构建现代 Windows 桌面应用。
- 通过 `RuntimeProvider`（进程和模拟器访问）、`GameAdapter`（游戏/版本语义）、`FeaturePack`（具体功能）、`UI Schema`（动态菜单与参数）扩展，不在全局页面里硬编码具体游戏。
- 将调试日志、真实游戏操作历史、Obsidian 开发日记严格区分。
- 默认只读；未知版本禁止写入；功能修改必须预览、校验、执行、记录并尽可能恢复。
- 不支持反作弊规避、隐蔽注入或联网多人作弊功能。

## UI V3 交互原型

- [完整离线 HTML 演示](preview/GameTrainer-Studio-V3.html) — 浏览器下载后打开；Gen3–Gen6、PC 游戏库、动态模块和日记页面。**仅 Mock UI，不连接真实进程。**
- [使用说明](preview/README.md)

## M0 工程与验证

```bash
corepack enable
pnpm install
pnpm dev                         # 浏览器 Mock
pnpm desktop                     # Windows Tauri 2 开发启动（需要 Rust/MSVC/WebView2）
pnpm lint && pnpm typecheck && pnpm test && pnpm build
cargo fmt --all -- --check
cargo clippy -p trainer-core -p trainer-mgba --all-targets -- -D warnings
cargo test -p trainer-core -p trainer-mgba
pnpm --filter @gametrainer/desktop test:e2e  # 首先安装 Playwright Chromium
```

- [React/Tauri 应用目录](apps/desktop/) · [Rust 核心注册中心](crates/trainer-core/src/lib.rs)
- [五类游戏平台共享 Fixture](fixtures/m0-registry.json) · [M0 实施与测试记录](docs/m0-implementation.md)
- [最新完整 CI（Rust 16 项、React 10 项、Chromium 3 个尺寸/9 张截图、Windows Shell/Test/EXE）](https://github.com/969981/GameTrainer/actions/runs/45107272587)
- [Windows 调试 EXE Artifact](https://github.com/969981/GameTrainer/actions/runs/45107272587/artifacts/14787309682)（CI 产物，非正式安装器）
- [Chromium 三尺寸截图 Artifact](https://github.com/969981/GameTrainer/actions/runs/45107272587/artifacts/14788062150)
- 注意：`pnpm dev` 只能浏览 Mock；`pnpm desktop` 才启用 Tauri Windows 只读 IPC；Windows GUI、真实 mGBA + Emerald ROM 必须另行实机验证。

## M1 · Emerald/mGBA 只读接入

- [源码/限制与本机验证说明](docs/m1-readonly.md) · [只读 ROM/Seed 解析核心](crates/trainer-mgba/src/lib.rs) · [Windows Runtime](crates/trainer-runtime-win/src/windows.rs) · [Tauri IPC](apps/desktop/src-tauri/src/lib.rs)
- 桌面版运行后：启动 mGBA → 进入「运行时」→ 刷新进程 → 选择 PID → **只读连接** → 在 Emerald 的「RNG / Seed」查看 Current Seed、Initial Seed 与 Advances。
- 已配置六种 Rev0 ROM GameCode（BPEJ/BPEE/BPED/BPEF/BPEI/BPES）。现在只有**部分源代码锚点验证**，没有全 ROM CRC 或全部锚点，所以绝不开放写入。
- 暂未经过真实 mGBA 版本、合法原版 ROM、实际存储映射/ROM Reload 测试；候选不确定时应失败关闭，而不是猜地址。

## M2 · 补丁事务和 SQLite（Fixture-only）

- [完整技术说明](docs/m2-safe-patch.md) · [ADR-0002](docs/decisions/0002-fixture-only-mutations.md)
- [Rust PatchManager](crates/trainer-patch/src/lib.rs) + [SQLite AuditStore](crates/trainer-storage/src/lib.rs) + [Tauri M2 IPC](apps/desktop/src-tauri/src/m2.rs)。
- 桌面「补丁实验室」可在隔离的 Rust 测试内存中完成 Seed 预览 → 明确确认 → 应用 → 安全恢复；「操作历史」区分真实与模拟实验。
- **真实 mGBA 和其他游戏进程仍只读**；M2 尚未实机验证，不能把 Fixture 的成功等同于真实游戏修改。

- [M2 CI 全部通过（Rust 32 项、前端单测与三尺寸浏览器检查、Windows Tauri EXE）](https://github.com/969981/GameTrainer/actions/runs/45109373457)
- [M2 Windows CI 调试版](https://github.com/969981/GameTrainer/actions/runs/45109373457/artifacts/14788604154) · [M2 三尺寸页面截图](https://github.com/969981/GameTrainer/actions/runs/45109373457/artifacts/14789250532)

## Ruby / Sapphire Gen3 适配（只读）

- [26 个零售版 Profile、内存布局、源代码证据与安全边界](docs/gen3-ruby-sapphire.md) · [ADR-0003](docs/decisions/0003-gen3-ruby-sapphire.md)。
- 新增独立 Pokémon Ruby / Pokémon Sapphire 游戏，Rust `trainer-rs` 实现 26 个版本的只读 Header/Anchor/Seed/Timer；Windows mGBA 有独立只读连接命令。
- M2 可选择 Emerald / Ruby / Sapphire 三种隔离 Fixture 做 Seed 事务与 SQLite 审计，**真实 mGBA 和所有原生游戏仍禁止写入**。
- v4.6.0 BattleCrit 源资料仅部分 ROM 静态验证，缺乏真实 mGBA 战斗测试，只作为研究模块。

**2026-11-23 Gen3 扩展 CI：三组全部通过**——Rust 44 项单测（Core 8、Emerald 9、Ruby/Sapphire 8、Patch 16、SQLite 3），React/Vitest 11 项、Chromium 三宽度多页面导航与截图，以及 Windows Tauri `cargo check`、Rust 命令测试、调试版 EXE 构建。

- [完整 CI](https://github.com/969981/GameTrainer/actions/runs/45111652638) · [Windows x64 测试 EXE（ZIP）](https://github.com/969981/GameTrainer/actions/runs/45111652638/artifacts/14789568790) · [18 张页面截图](https://github.com/969981/GameTrainer/actions/runs/45111652638/artifacts/14789319553)
- 这些测试不代表 Windows GUI/WebView2 手动启动或真实 mGBA ROM 的 Seed/Timer 实机读取成功；真实游戏写入仍保持关闭。

## Ruby / Sapphire v4.6 · 安全迁移评审（2026-11-23）

- [安全迁移评估：捕捉 / 钓鱼 / 传送 / 战斗](docs/gen3-rs-migration-security-review.md)：逐项列出上游源码支持程度、功能风险、M1/M2 缺口、迁移优先级和未来实机门禁。
- [ADR-0004 多资源补丁安全边界](docs/decisions/0004-rs-migration-write-gates.md)；[7 项功能的禁写 Manifest](fixtures/rs-migration-review.json)。
- 结论：先做捕捉两处指令的**只读**比对、钓鱼 RNG/状态观察和 306 条传送目录。多资源事务、真实地图切换以及 BattleCrit Hook 均不能通过解除 M2 的 Fixture 锁直接启用。
- 源码专项 pytest：**47 passed / 2 skipped**；完整上传包测试存在因缺失历史 `dist/*.CT` 文件而失败的用例，不等同实机通过。

## 文档入口

| 文档 | 用途 |
| --- | --- |
| [AGENTS.md](AGENTS.md) | AI 编程代理统一执行规范 |
| [docs/m0-implementation.md](docs/m0-implementation.md) | M0 已实现工程、测试证据和待办 |
| [docs/m1-readonly.md](docs/m1-readonly.md) | Emerald/mGBA 只读连接原型与实机验收计划 |
| [docs/architecture.md](docs/architecture.md) | 分层架构与依赖方向 |
| [docs/extensions.md](docs/extensions.md) | Gen3–Gen6 / PC Native 扩展与模块清单 |
| [docs/ui/README.md](docs/ui/README.md) | V3 导航、布局、页面与设计令牌 |
| [docs/module-contract.md](docs/module-contract.md) | 插件描述符、能力、版本与安全约束 |
| [docs/logging-and-diary.md](docs/logging-and-diary.md) | SQLite 审计、系统日志和 Obsidian |
| [docs/roadmap.md](docs/roadmap.md) | M0–M6 实施里程碑和验收 |
| [docs/decisions/0001-modular-architecture.md](docs/decisions/0001-modular-architecture.md) | 架构决策记录 |
| [docs/2026-11-23-planning.md](docs/2026-11-23-planning.md) | 本次规划和开发日记 |
| [examples/game-manifests.json](examples/game-manifests.json) | 仅演示用途的游戏描述符 |

## 预计项目结构（非已实现）

```text
apps/desktop/                 # React/Vite UI + Tauri Shell（M0/M1 已有）
crates/trainer-core/          # Registry、只读 Session DTO（已实现）
crates/trainer-runtime-win/   # Win32 mGBA 只读外部进程访问（代码已提交，未实机验证）
crates/trainer-mgba/          # Emerald ROM/Seed 只读解码及会话（已有测试）
crates/trainer-patch/         # preview/apply/restore
crates/trainer-storage/       # SQLite、日志
games/pokemon/gen3/          # 世代共享 codec / adapter
games/pokemon/gen4/
games/pokemon/gen5/
games/pokemon/gen6/
games/native/                # Windows 原生单机游戏适配
features/                    # 可复用的 Feature Packs
profiles/                    # 每游戏每版本隔离配置
docs/                         # Obsidian 友好 Markdown 文档
```

**路线：** M0 UI + Mock Registry（CI 验收完成，Windows GUI 实操待验证）→ M1 Emerald/mGBA 只读（实现/编译测试通过，真实 ROM 待验）→ M2 Fixture-only 安全事务与审计 → M3 更多 Gen3 功能 → M4 Gen4/5/6 与 PC Adapter 验证 → M5 日记与插件管理 → M6 可选 Native Agent/MinHook。

## 许可证提醒

已研究过的 Emerald v5.3.0 Lua/CT 资料带有 GPL-3.0 许可背景。任何代码移植、复用或发行必须单独完成许可证与来源审查。计划文档不等于已完成移植。
