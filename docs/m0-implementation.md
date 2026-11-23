# M0 · 工程初始化与注册中心实现记录

日期：2026-11-23
状态：代码已提交；自动化构建测试以 GitHub Actions 结果为准。未进行 Windows/Tauri 实机运行验证。

## 代码落地
- Cargo Workspace：根 Cargo.toml、crates/trainer-core；纯 Rust typed GameRegistry / GameAdapter / 校验。
- Tauri 2：apps/desktop/src-tauri，只有 get_catalog、get_session_snapshot 两个只读命令。
- pnpm + React 19 + TypeScript + Vite 7 + Tailwind 4；shadcn/ui 组件规范、Button/Card/Badge。
- 单一数据源 fixtures/m0-registry.json：Gen3 Emerald、Gen4 Platinum、Gen5 Black2、Gen6 X/Y、PC Native Template。
- 前端通过 Tauri invoke（桌面）或共享 Fixture（浏览器）加载目录；Tauri 失败不会伪造浏览器数据。
- V3 布局：PrimaryRail / ContextSidebar / Header / GameLibrary / GameWorkspace / Modules / Runtime / History / Journal / Settings / SessionInspector。
- 动态侧栏、参数面板与功能卡由 GameDescriptor/FeatureDescriptor 驱动；无硬编码 Emerald 菜单。
- Rust Core 测试：五平台、六款扩展、冲突 ID、未知运行时、重复功能、不可信目录、序列化。
- Web 测试：五平台筛选、六款扩展、Gen3→Gen6 状态隔离、无匹配空态、所有 Mock 修改禁用。
- GitHub Actions：Rust fmt/clippy/test，pnpm lint/typecheck/test/build。

## 限制
- 只实现 M0 Mock。没有模拟器附加、进程读取、Hook、ROM Profile 验证、PatchManager、SQLite 实际审计。
- UI History 为空；Journal 是页面内临时草稿，未写入用户个人 Obsidian。
- 未在 Windows 完成 Tauri build/dev GUI 验证；截图/视觉回归属于后续项。
- 第六款游戏在回归测试中临时注册，不属于生产默认的五款清单。

## 结构检查 / 后续
- [ ] CI 全部通过、处理依赖/格式/类型报错。
- [ ] Windows 11 Tauri dev GUI 实测及 1100/1366/1500px 布局回归。
- [ ] M1 mGBA + Emerald 只读接入与 ROM 指纹校验。
- [ ] 适配器/FeaturePack 从数据 Manifest 迁移到 Rust 编译期游戏模块。
- [ ] 审查第三方依赖许可证，决定仓库正式许可证。
