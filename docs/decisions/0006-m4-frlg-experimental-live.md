# ADR-0006 · FireRed / LeafGreen v2.0.4 精确 ROM 实验性写入

日期：2026-11-23
状态：Accepted（仅受控实验能力，**尚无真实 mGBA ROM 测试**）

## 背景

用户上传了 FRLG All-in-One v2.0.4 ScanWildHookFix 的 Lua、CT 与 116B Thumb Payload，要求火红叶绿也拥有类似 Ruby/Sapphire 的 M3 真实内存写入。直接复用红蓝宝石的机器码/地址会导致错误修改，因此必须建立独立 FRLG Profile 和独立事务。

## 决策

1. 新增 trainer-frlg 与 16 个官方零售 ROM 的 GameCode+Rev+SHA1 白名单。Guest ROM 由 mGBA GDB 读取整 16MiB，实际 SHA1 不匹配禁止任何写入；仅有 CRC32、Header 或 ROM Anchor 不构成授权。
2. 仅开放固定的 Capture、Wall 两处 Halfword，以及有上传源证据的 Wild 116B Thumb Payload Hook。前端既不能指定任意 Guest 地址，也不能传任意机器码；Payload 必须是用户本地持有、来源固定 SHA256 和 116B 长度一致的样本。
3. Live 事务由 Rust 控制暂停、完整 ROM 哈希确认、原字节/上下文校验、SQLite Intent、写后回读、恢复所有权和错误状态。Hook 安装入口跳板最后切换，卸载先解绑后擦除 Cave；gMain battle、PC 在 Cave、SaveState 跨会话等情况拒绝危险恢复。
4. M3 Ruby/Sapphire 与 M4 FRLG 的 GDB session 互斥，SQLite 历史独立，失败时不把已不确定的 Guest 原字节自动重放到其它 ROM 或游戏进程。
5. UI 需要明确确认存档备份和离线单人测试；单次 Seed 注入独立再次确认。源码的 GPL-3.0 Thumb Payload 由用户本地选择，不随 GameTrainer 发布。正式发行前独立复核派生许可范围。
6. 原包其它传送、Fishing、游走/特殊遭遇等 Hook 暂不移植成可执行，因为还没有针对世界状态、脚本和邮件槽副作用的完整撤销证明。
7. Windows CI 及 Rust mock 测试不等于真实 ROM 运行验证，未经实机验收一律标记实验性，不宣称稳定可用。

关联：[M4 说明](../m4-frlg-v204-live.md) · [Profile](../../fixtures/frlg-v204-roms.json) · [Rust](../../crates/trainer-frlg/src/live.rs)。
