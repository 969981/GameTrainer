# 开发路线 M0–M6

状态：规划，未开始 Rust/Tauri 的正式实现。每个里程碑同时验收结构、UI、文档、测试与故障路径。

| Milestone | 交付 | 完成标准 |
| --- | --- | --- |
| M0 平台与 UI 基座 | Cargo/pnpm workspace；Tauri 2；React Design System；typed Registry；Gen3/4/5/6/PC Mock | 第六款 Mock 加入时不需改主导航；跨游戏状态隔离；UI 1100/1366/1500px 可用 |
| M1 Emerald 只读 | Win32/mGBA provider、ROM Rev/region 验证、Seed Snapshot、诊断与 Session Inspector | Win+mGBA 实测与原 Lua 比对；未知 ROM 只读；进程重启断开 |
| M2 安全修改 | Seed 受控写入/锁定、Preview/Apply/Restore、冲突检查、SQLite ActionReceipt | stale epoch/版本变更拒绝写；恢复失败可审计；强退后不盲目恢复 |
| M3 Gen3 功能迁移 | 捕捉/传送/钓鱼/培育屋等按独立 FeaturePack 迁移 | 每个功能有 Profile 证据、语言差异测试、真实恢复测试 |
| M4 游戏拓展 | 同 Runtime 的第二个 Gen3 游戏，再扩展 NDS/3DS 与 PC Native | 新 GameAdapter 不修改 core 或 AppShell；不同 Runtime 有一致的 UI 状态 |
| M5 插件、预设与日记 | 模块列表、版本/依赖管理、导出审计、Obsidian 明确确认式导出 | 不覆盖原笔记、不丢审计、模块升级/卸载行为可测 |
| M6 高级研究 | WASM 纯逻辑插件 PoC、可选可信 Native Agent/MinHook | 跨进程协议经版本校验，未知原生二进制不自动加载 |

## P0 任务拆分
- [ ] 创建 apps/desktop 与 crates workspace
- [ ] 定义 PlatformFamily / RuntimeDescriptor / GameDescriptor / FeatureDescriptor 类型
- [ ] 建立 MockRegistry：Gen3/Gen4/Gen5/Gen6/PC 各 1 个样例
- [ ] 还原 V3 AppShell、游戏卡、动态 ContextSidebar、功能详情与 Inspector
- [ ] 固定组件 tokens、空态、只读与错误样式
- [ ] 加入基本单测、ESLint/Prettier、cargo fmt/clippy、CI（确有对应代码后）
- [ ] 提交 ADR 与测试证据

## 公共完成定义（DoD）
所有工作须包含：代码、Fixture/Mock 或实机测试、正常与异常状态、日志策略、许可评估、文档变更和提交摘要。真实连接与修改必须有版本/环境/复现说明；不得把浏览器原型通过当作 Windows+模拟器实测。
