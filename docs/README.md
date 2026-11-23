# GameTrainer 文档索引（Obsidian / GitHub 双兼容）

> 本目录是项目设计规范与开发研究，不包含个人 Vault 内容。双链用于 Obsidian，标准相对链接用于 GitHub。

## 工程实施
- [M0 工程与 CI 验证](m0-implementation.md) · [[m0-implementation]]
- [M1 Emerald/mGBA 只读实现、实机验收清单](m1-readonly.md) · [[m1-readonly]]

- [M2 安全补丁与 SQLite 实施](m2-safe-patch.md) · [[m2-safe-patch]]

- [红蓝宝石四类功能的安全迁移审查](gen3-rs-migration-security-review.md) · [[gen3-rs-migration-security-review]]
- [ADR-0004 多资源补丁与 Live 写入门禁](decisions/0004-rs-migration-write-gates.md)

- [M3 真实 Guest GDB 写入与实验性 BattleCrit](m3-live-gdb-experimental.md) · [[m3-live-gdb-experimental]]
- [ADR-0005 精确 ROM 实验写入门禁](decisions/0005-m3-gdb-guarded-live.md)

## 核心设计
- [总体架构](architecture.md) · [[architecture]]
- [扩展规划：Gen3–Gen6 + PC](extensions.md) · [[extensions]]
- [模块接口与 Profile 契约](module-contract.md) · [[module-contract]]
- [V3 UI 与导航布局](ui/README.md) · [[ui/README]]
- [UI 组件状态规范](ui/components.md) · [[ui/components]]
- [UI Wireframe](ui/wireframe.md) · [[ui/wireframe]]
- [日志 / 审计 / Obsidian 日记](logging-and-diary.md) · [[logging-and-diary]]
- [Emerald v5.3.0 迁移基线](emerald-migration.md) · [[emerald-migration]]
- [开发路线和验收](roadmap.md) · [[roadmap]]
- [测试矩阵](testing.md) · [[testing]]
- [许可证与安全边界](license-and-safety.md) · [[license-and-safety]]
- [ADR-0001 模块化架构](decisions/0001-modular-architecture.md)
- [ADR-0002 Fixture-only 事务](decisions/0002-fixture-only-mutations.md)
- [2026-11-23 项目开发记录](2026-11-23-planning.md)

所有 Markdown 应说明是设计、Mock、已实现或已测试。切勿将本仓库的计划性链接误认为对用户的 Personal-Life-OS 已完成更新。

- [Ruby/Sapphire Gen3 适配与证据](gen3-ruby-sapphire.md) · [[gen3-ruby-sapphire]]
- [ADR-0003 Gen3 RS 独立游戏适配器](decisions/0003-gen3-ruby-sapphire.md)
