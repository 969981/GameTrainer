# GameTrainer Studio

> 多游戏、跨运行时的 Windows 桌面研究与修改工具（M0 已实现，M1 只读接入验证阶段）

**当前状态：M0 自动化测试通过，M1 Emerald/mGBA 只读原型代码已实现。** Windows Tauri 的 EXE、Rust 命令单测、React/Vite 和 Chromium 响应式测试均在 CI 通过；M1 新增 mGBA 进程发现、部分 ROM 指令锚点验证、Seed/Advances 只读快照及会话失效保护。**没有在真实 Windows 桌面上亲自启动 WebView2，也没有用真实 mGBA + ROM 完成内存读取验证；没有写入、Hook、反作弊绕过。**

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

**路线：** M0 UI + Mock Registry（CI 验收完成，Windows GUI 实操待验证）→ M1 Emerald/mGBA 只读（实现/编译测试通过，真实 ROM 待验）→ M2 安全修改与审计 → M3 更多 Gen3 功能 → M4 Gen4/5/6 与 PC Adapter 验证 → M5 日记与插件管理 → M6 可选 Native Agent/MinHook。

## 许可证提醒

已研究过的 Emerald v5.3.0 Lua/CT 资料带有 GPL-3.0 许可背景。任何代码移植、复用或发行必须单独完成许可证与来源审查。计划文档不等于已完成移植。
