# ADR-0004：Ruby/Sapphire 多资源修改必须先满足独立的 Live 写入门禁

日期：2026-11-23。状态：Accepted，**当前 Live 写入全部禁止**。

## 背景

用户上传的 RS v4.6.0 捕捉是两处 ROM 半字修改；钓鱼代理涉及三表指针、代码洞、Mailbox；传送涉及 callback2 和游戏世界状态；BattleCrit 是实验性战斗指针表重定向。GameTrainer 当前 M2 仅有单 u32 纯内存 Fixture，无法保证上述资源的统一生命周期。

## 决策

1. [安全迁移审查](../gen3-rs-migration-security-review.md) 与 [可测试门禁清单](../../fixtures/rs-migration-review.json) 的所有 live_write_enabled 均固定为 false。
2. 第一阶段只迁移只读 Profile/原指令差异、Fishing 状态与 RNG 模型、地图/目标位置风险目录，不接真实 ROM 写入。
3. 若未来单独评审多资源事务，必须具备：全 ROM 和逐功能完整机器码身份、CPU quiescence/帧边界证据、资源重叠锁、逐资源 SQLite Intent、激活指针最后写、停用先解绑入口、证明 CPU 不在 code cave、按所有权恢复、未知/第三方修改拒绝、重启 SaveState 旧跳板不盲目自愈。
4. 地图变更是不可逆的游戏世界副作用；恢复 Hook 和 Payload 不等于恢复玩家位置/Flag。
5. BattleCrit 四 ROM 上游静态证据不构成 live 执行授权；必中、敌方 Miss、OHKO 仍属于原包未实现。
6. GPL-3.0 Lua/Thumb payload 的派生移植与发行需独立许可审查；不复制原包代码到本仓库。
7. 任何将来申请开放真正 live 写入的功能都必须提交独立 ADR、真实 mGBA 实测日志与可验证失败恢复证据，不能仅关闭 Rust WriteLocked。

## 结果

目前可以提升 UI 的只读可见性和设计安全性，但不得让 React、Tauri IPC 或 Rust 获取任意内存写地址，也不应把源包的静态 ROM 报告称为本项目真实实机测试。
