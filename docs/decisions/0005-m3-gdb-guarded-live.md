# ADR-0005 · 从 M2 纯 Fixture 到 M3 精确 ROM 实验性写入

日期：2026-11-23。状态：Accepted（**实验性实现，尚未实机验收**）。

## 背景
用户明确要求实现真实内存写入和实验性 Hook。此前 [ADR-0004](0004-rs-migration-write-gates.md) 禁止仅凭 ROM Header/Anchor 就向外部模拟器写入；本决定没有取消未知 ROM 拒绝和恢复所有权原则，而是新增一个独立且受限的 Guest GDB 通道。

## 决策
1. 新增 trainer-rsp 与 trainer-live，保留 M2 PatchManager 的纯 Fixture 和禁止 live 的原设计；真实写入不通过 Windows WriteProcessMemory。
2. GDB 客户端仅连接本机 127.0.0.1，要求 CPU 有明确 stopped 状态、原版 ROM 的**完整 Guest SHA1**匹配 28 份源报告白名单、功能所在原字节及指令上下文正确、SQLite Intent 先落盘、写后 readback。
3. 仅开放固定的 Capture 两处 halfword 与四 ROM 的 BattleCrit 研究 Hook；函数指针最后激活、恢复先解绑，用户提供的本地 GPL 模板必须匹配固定 SHA256。所有操作必须在可信 Rust 中复核，不能向前端暴露任意地址写入。
4. Rust IPC 和 UI 都要求用户明确确认已经备份存档，并仅在离线单机测试。与 M2 Fixture 历史隔离，异常退出时不跨会话自动恢复旧补丁。
5. Fishing/Feebas 与 Teleport 继续禁用真实 Hook，因为其输入/RNG、地图、Callback 与世界状态的生命周期还没有能够保证正确恢复的实机证据。源工具尚未实现的必中、敌方 Miss 和 OHKO 也不得开放。
6. 当前尚未在真实 Windows+mGBA 原版 ROM 上运行验证；任何 CI 通过只表示构建和合成测试成功，不能当作金手指可用或完全安全的证据。以后若需要生产发布，必须完成许可证和专项恢复审查。

## 未闭合风险
捕捉是两个非原子写入，模拟器或工具崩溃会遗留中间状态；BattleCrit 卸载时只检查 PC 并不足以证明正在返回的回调完全安全；SaveState 可能保留旧 Hook。恢复失败必须保留所有权和错误状态，不得强制覆写。

关联：[M3 实现及操作手册](../m3-live-gdb-experimental.md) · [源 SHA1](../../fixtures/rs-known-rom-sha1.json) · [Rust 代码](../../crates/trainer-live/src/lib.rs)。
