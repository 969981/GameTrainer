# M0 · 工程初始化与注册中心实现记录

日期：2026-11-23
状态：M0 核心代码已提交；GitHub Actions [#45104751458](https://github.com/969981/GameTrainer/actions/runs/45104751458) 两项作业均通过。未进行 Windows/Tauri 实机运行验证。

## 代码落地
- Cargo Workspace：根 Cargo.toml、crates/trainer-core；纯 Rust typed GameRegistry / GameAdapter / 校验。
- Tauri 2：apps/desktop/src-tauri 在 M0 提供 get_catalog、get_session_snapshot；M1 后续新增 Windows-only 只读 mGBA 命令。
- pnpm + React 19 + TypeScript + Vite 7 + Tailwind 4；shadcn/ui 组件规范、Button/Card/Badge。
- 单一数据源 fixtures/m0-registry.json：Gen3 Emerald、Gen4 Platinum、Gen5 Black2、Gen6 X/Y、PC Native Template。
- 前端通过 Tauri invoke（桌面）或共享 Fixture（浏览器）加载目录；Tauri 失败不会伪造浏览器数据。
- V3 布局：PrimaryRail / ContextSidebar / Header / GameLibrary / GameWorkspace / Modules / Runtime / History / Journal / Settings / SessionInspector。
- 动态侧栏、参数面板与功能卡由 GameDescriptor/FeatureDescriptor 驱动；无硬编码 Emerald 菜单。
- Rust Core 测试：五平台、六款扩展、冲突 ID、未知运行时、重复功能、不可信目录、序列化。
- Web 测试：五平台筛选、六款扩展、Gen3→Gen6 状态隔离、无匹配空态、所有 Mock 修改禁用。
- GitHub Actions：Rust fmt/clippy/test，pnpm lint/typecheck/test/build。

## 2026-11-23 最终 CI 证据

- [CI #45107272587](https://github.com/969981/GameTrainer/actions/runs/45107272587)：**Rust 16 项通过**（trainer-core 7 + trainer-mgba 9）；**React/Vitest 10 项通过**；ESLint、TS、Vite Web Build 均通过。
- Chromium Playwright：1100 / 1366 / 1500px 的游戏库、Gen6、PC Native 导航和横向溢出检查 **3/3 通过**；[9 张截图](https://github.com/969981/GameTrainer/actions/runs/45107272587/artifacts/14788062150)。
- Windows：Tauri Shell `cargo check` 通过、`get_catalog` 命令 Rust 单测通过、`cargo build` 成功并上传 [debug EXE](https://github.com/969981/GameTrainer/actions/runs/45107272587/artifacts/14787309682)。
- **不代表：**用户桌面启动、真实 WebView2 视觉回归，或 mGBA + Emerald ROM 的实际读取验证。

## 早期 M0 CI 证据

- Rust Core：`cargo fmt --all -- --check`、`cargo clippy -p trainer-core --all-targets -- -D warnings`、`cargo test -p trainer-core` 均通过；**7 passed / 0 failed**。
- React：`pnpm lint`、`pnpm typecheck`、`pnpm test`、`pnpm build` 均通过；**9 passed / 0 failed**，Vite Web build 成功。
- 最初两轮 CI 分别发现 pnpm lockfile 缓存配置、rustfmt 差异、Vitest DOM 未自动清理及一条文本断言；已修复并在上述 CI 验证。
- 测试覆盖：Gen3/Gen4/Gen5/Gen6/PC、增加第六款游戏不改主导航、功能隔离、只读修改按钮、重复 ID / 未知 Runtime 拒绝等。
- 本次测试为 Linux CI 上的 Rust Core、jsdom UI 和 Vite Web 构建，不代表 Windows 原生 Tauri GUI 已编译或实测。

## 限制
- M0 浏览器仍仅 Mock；后续 M1 Windows Tauri 已加入只读进程发现和部分 ROM 验证代码，但未真实连接模拟器实机。没有 Hook、PatchManager、任意写入或 SQLite 实际审计。
- UI History 为空；Journal 是页面内临时草稿，未写入用户个人 Obsidian。
- Windows Tauri `cargo check` + `cargo build` + Rust 命令测试已成功；Windows GUI/WebView2 **实际窗口**尚未人工验证；Chromium 截图/宽度回归已通过。
- 第六款游戏在回归测试中临时注册，不属于生产默认的五款清单。

## 结构检查 / 后续
- [x] CI 全部通过、处理依赖/格式/类型报错（#45104751458）。
- [x] Linux Chromium 浏览器 1100/1366/1500px 布局、导航与截图回归。
- [x] Windows Tauri Shell check/test/EXE 构建通过。
- [ ] Windows 11 真实 Tauri WebView2 GUI 启动、IPC 前端到后端端到端验证和布局实机检查。
- [x] M1 mGBA + Emerald 只读核心/Windows Provider/Tauri 接口代码已完成并 CI 编译；**真实 ROM/进程读取未验**。
- [ ] 适配器/FeaturePack 从数据 Manifest 迁移到 Rust 编译期游戏模块。
- [ ] 审查第三方依赖许可证，决定仓库正式许可证。
