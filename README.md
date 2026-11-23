# GameTrainer Studio

> 多游戏、跨运行时的 Windows 桌面修改与研究工具（规划阶段）

**当前状态：M0 已实现（Mock）。** 仓库已包含 Rust GameRegistry、Tauri 2 只读 IPC、React V3 动态工作台、五类游戏 Fixture 和自动化测试；**仍未实现真实进程连接、游戏内存读写、Hook 或 Windows GUI 实机验证**。所有示例游戏与配置均为 Mock，不能作为已验证的游戏 Profile。

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
cargo clippy -p trainer-core --all-targets -- -D warnings
cargo test -p trainer-core
```

- [React/Tauri 应用目录](apps/desktop/) · [Rust 核心注册中心](crates/trainer-core/src/lib.rs)
- [五类游戏平台共享 Fixture](fixtures/m0-registry.json) · [M0 实施与测试记录](docs/m0-implementation.md)
- [通过的 CI（Rust 7 项、前端 9 项）](https://github.com/969981/GameTrainer/actions/runs/45104751458)
- 注意：`pnpm dev` 是浏览器 Mock；`pnpm desktop` 尚需 Windows GUI 实测，不能视为已经验证。

## 文档入口

| 文档 | 用途 |
| --- | --- |
| [AGENTS.md](AGENTS.md) | AI 编程代理统一执行规范 |
| [docs/m0-implementation.md](docs/m0-implementation.md) | M0 已实现工程、测试证据和待办 |
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
apps/desktop/                 # React/Vite UI + Tauri Shell
crates/trainer-core/          # Registry、session、capability
crates/trainer-runtime-win/   # Win32 外部进程
crates/trainer-runtime-mgba/  # 模拟器适配
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

**路线：** M0 UI + Mock Registry（核心 CI 已通过，Windows GUI 待验收）→ M1 Emerald/mGBA 只读 → M2 安全修改与审计 → M3 更多 Gen3 功能 → M4 Gen4/5/6 与 PC Adapter 验证 → M5 日记与插件管理 → M6 可选 Native Agent/MinHook。

## 许可证提醒

已研究过的 Emerald v5.3.0 Lua/CT 资料带有 GPL-3.0 许可背景。任何代码移植、复用或发行必须单独完成许可证与来源审查。计划文档不等于已完成移植。
