# 开发路线 M0–M6

状态：**M0 自动化全部通过**（2026-11-23，Rust 16 测试、React 10 测试、Chromium 1100/1366/1500px、Windows Tauri Shell/命令测试/EXE 构建）；**Windows WebView2 实际启动尚未人工验收**。M1 只读连接代码/CI 已完成第一迭代，真实 mGBA/ROM 待验。M2 Fixture-only Rust/SQLite/UI 已实现并通过 [CI #45109373457](https://github.com/969981/GameTrainer/actions/runs/45109373457)；真实外部进程写入保持禁止；M3–M6 仍为规划。

| Milestone | 交付 | 完成标准 |
| --- | --- | --- |
| M0 平台与 UI 基座 | **核心实现已提交：** Cargo/pnpm/Tauri 2/React；typed Registry；Gen3/4/5/6/PC Mock | **CI 已验收：** 六款扩展、跨游戏隔离、Rust 16 + React 10、Windows EXE、浏览器 1100/1366/1500px 截图；**待验：** Windows WebView2 GUI 实机 |
| M1 Emerald 只读 | **第一迭代代码已实现：** Win32/mGBA Provider、六语言 Rev0 / 部分 ROM Anchor、Seed Snapshot、会话失效检查、Tauri IPC/React UI | **CI 已验证编译和合成单测；待验：** Windows+mGBA 真实读数、完整 ROM CRC、GUI 连通性与重启失效 |
| M2 安全修改 | **Fixture-only 第一阶段已提交**：30s 令牌、预览/应用/恢复、原字节与 owner 校验、SQLite append-only Ledger、UI 实验室 | CI 已通过 Rust Patch/Storage 共 16 项测试及 Windows Tauri EXE；真实 mGBA 写入关闭；崩溃后不自动恢复 |
| M3 Gen3 功能迁移 | 捕捉/传送/钓鱼/培育屋等按独立 FeaturePack 迁移 | 每个功能有 Profile 证据、语言差异测试、真实恢复测试 |
| M4 游戏拓展 | 已提前集成 Ruby/Sapphire 的独立 Gen3 只读核心与 Fixture，后续 FireRed/LeafGreen、NDS/3DS 和 PC Native | 真实 mGBA 读取仍需实机验证；任何写入保持禁止 |
| M5 插件、预设与日记 | 模块列表、版本/依赖管理、导出审计、Obsidian 明确确认式导出 | 不覆盖原笔记、不丢审计、模块升级/卸载行为可测 |
| M6 高级研究 | WASM 纯逻辑插件 PoC、可选可信 Native Agent/MinHook | 跨进程协议经版本校验，未知原生二进制不自动加载 |

## P0 任务拆分
- [x] 创建 apps/desktop 与 crates workspace
- [x] 定义 PlatformFamily / RuntimeDescriptor / GameDescriptor / FeatureDescriptor 类型
- [x] 建立 MockRegistry：Gen3/Gen4/Gen5/Gen6/PC 各 1 个样例
- [x] 构建 V3 AppShell、游戏卡、动态 ContextSidebar、功能详情与 Inspector（已通过 Web Build，尚未 Windows GUI 视觉验收）
- [x] 固定组件 tokens、空态、只读与错误样式（基础版）
- [x] 接入 Rust fmt/clippy/test、ESLint、TypeScript、Vitest、Vite/Chromium Playwright、Windows Tauri Build 与 GitHub Actions CI；尚无 Prettier 独立配置
- [x] 提交 ADR 与 CI 测试证据（详见 [M0 实施记录](m0-implementation.md)）
- [x] Windows Tauri 2 Shell 编译与 EXE 构建、Rust 命令单测；Chromium 三尺寸布局截图验证
- [ ] Windows Tauri 2 GUI 本机启动、前端到 Rust IPC 连通性、WebView2 缩放与布局验收
- [ ] Windows 实机 mGBA + Emerald Rev0 只读 ROM 指纹/Seed 对照测试

## 公共完成定义（DoD）
所有工作须包含：代码、Fixture/Mock 或实机测试、正常与异常状态、日志策略、许可评估、文档变更和提交摘要。真实连接与修改必须有版本/环境/复现说明；不得把浏览器原型通过当作 Windows+模拟器实测。

## Ruby/Sapphire v4.6 安全迁移专项（2026-11-23）

- **P0**：安全评估、七项禁写 Manifest、Vitest Guard（已提交）。
- **P1**：100% 捕捉只读机器码对照；Fishing/Feebas 只读状态与 RNG；306 地点证据分级目录。
- **P2**：独立 MultiResourcePatchManager 的全合成测试：资源租约、修改前签名、激活顺序/撤销顺序、CPU 安全点和 SQLite 逐 Component 记录。
- **P3/P4**：只有完整 ROM SHA1/原入口静态证据与真实 Windows+mGBA 沙盒实测后，才讨论捕捉、钓鱼、传送的可授权写入。地图/剧情状态不可逆。
- **P5**：BattleCrit 保留研究，不开放用户开关；必中、敌方 Miss、一击必杀上游尚未实现。

[完整技术审查](gen3-rs-migration-security-review.md) · [ADR-0004](decisions/0004-rs-migration-write-gates.md)

## M3 · 精确 ROM 的实验 GDB 写入（2026-11-23）

- Rust trainer-rsp + trainer-live：本机 GDB Guest 读取/写入、完整 ROM SHA1 白名单、Capture 双指令、四 ROM BattleCrit 受控 Hook，独立 SQLite 实操审计。
- Tauri/React 已新增 M3 确认操作界面；目前正在 CI 和安全恢复审查，未实机测试。
- Fishing/Feebas 与 Teleport 的运行时 Hook 仍在后续专项审查阶段，不允许仅把 M2 Fixture 校验关闭就启用。
- [详情](m3-live-gdb-experimental.md) · [ADR-0005](decisions/0005-m3-gdb-guarded-live.md)。

## M4 · FireRed / LeafGreen v2.0.4 受控 Guest GDB 写入

- 新增 16 零售版 ROM SHA1 Profile、Rust trainer-frlg、Tauri m4、React M4 GDB 操作面板以及游戏库 FireRed/LeafGreen 两个独立适配器。
- 捕捉和穿墙 Halfword、116B 受信任实验性 Wild Seed Hook 代码已实现，须从本地 GPL 原源包选择 Payload。mGBA 真正游戏测试、ROM Reload 和 SaveState Hook 恢复安全尚待现场验收。
- 源码钓鱼、传送、游走等其它 Hook 继续只读研究，不开放真实执行。
- [完整实施与验收约束](m4-frlg-v204-live.md) · [ADR-0006](decisions/0006-m4-frlg-experimental-live.md)。
