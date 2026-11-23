---
date: 2026-11-23
project: GameTrainer
type: implementation
status: experimental-not-emulator-verified
tags: [M3, GDB, Ruby, Sapphire, Capture, BattleCrit, SQLite]
---

# M3 · Ruby / Sapphire 本地 mGBA GDB 实验性写入

**已通过完整 CI 的 Rust/React/Windows 桌面构建，但仍未在用户真实 Windows+mGBA+原版 ROM 上实测捕捉或战斗功能。**

## 技术路径

- [GDB RSP Rust 模块](../crates/trainer-rsp/src/lib.rs)：仅连接 127.0.0.1，用户主动开启 mGBA GDB 服务后，确认 CPU 停止；按 GDB 协议处理 ACK/校验和、读取、写入、写后回读、继续与中断。使用 mGBA Guest patch 接口，不向 Windows 目标进程申请 VM_WRITE。
- [Live Patch Rust 模块](../crates/trainer-live/src/lib.rs)：利用 mGBA GDB 读取完整 ROM 并计算 SHA1，与 26 个零售版和 2 个日本 Box 变体的 [源报告哈希白名单](../fixtures/rs-known-rom-sha1.json)比较，之后再核对 Header、ROM Anchor、各功能的原始指令。
- [Tauri M3 IPC](../apps/desktop/src-tauri/src/m3.rs)：受限功能命令。React 不允许提交任意 Guest 写入地址、未审核的代码洞地址或原始调试命令。
- [React UI](../apps/desktop/src/components/M3LivePanel.tsx)：显示完整 ROM SHA1、捕捉/BattleCrit 预览、确认、执行、恢复及真实操作历史。浏览器测试不能模拟实际 GDB 执行。
- SQLite 文件：app-local-data/m3-live-action-history.sqlite3，和 M2 的纯内存 Fixture 历史分开。写入 Intent 在任何 Guest 写操作前提交；异常退出只能留下待核验记录，**不会自动写入旧地址**。

GDB Stub 的底层 Guest 写入参见 mGBA 官方源码：https://github.com/mgba-emu/mgba/blob/master/src/debugger/gdb-stub.c 。

## 两项实验功能

| 功能 | 已编码的操作 | 功能限制 |
|---|---|---|
| 100% 捕捉 | 核对源包所列两个 ROM halfword 原值和上下文；Guest CPU 停止时依次修改两处并读回；恢复反序进行 | 两个写入不是不可分割的硬件事务；任何部分失败必须核验，不得强行自愈 |
| 我方必会心 | **仅 AXPE Rev1/2、AXVJ Rev0、AXPJ Rev0**。根据源包的 ROM 指令前缀、字面池和脚本尾验证；124 字节本地 Thumb 模板 SHA256 白名单；代码洞先 staged、dispatch 指针最后激活，停用时指针先解绑 | 原资料是四 ROM 静态/合成证据，没有用户实机战斗验收。不得在战斗中装卸；异常双打/Link/状态位必须实测 |

BattleCrit 本地 Payload 必须从你已持有的原 v4.6.0 研究源码中选择 payloads/cheat_crit_dispatch_thumb.bin，长度 124 字节，SHA256 为 c0b3b51b0bea60c6e14107c1060891b88688993e5ca67ebb32486b139551a7db。**本仓库不包含/不分发该 GPL-3.0 二进制。**

**未启用：钓鱼辅助/Feebas 的三个状态机函数指针与随机数 Hook、一次性传送 callback2/代码洞 Hook，以及原资料未完成的必中、敌方 Miss、OHKO。** 后续仍需要独立的资源生命周期和游戏状态验证。

## 本机测试步骤与安全红线

1. 使用自己合法取得的原版、未汉化、未修改 Ruby/Sapphire ROM；复制备份 .sav 和 SaveState，并先准备可丢弃的离线单机测试进度。不要与在线联机或其它金手指工具同时运行。
2. 在 mGBA 开启本机 GDB Server（测试默认端口 2345）并暂停游戏 CPU。运行 GameTrainer Windows Tauri 版，打开「运行时」→「真实写入 · mGBA GDB 调试模式」。
3. 连接后核对 GameCode、Revision、完整 Guest ROM SHA1。只有匹配源包静态报告的 28 份完整镜像白名单才可继续。
4. 对捕捉或实验 Crit 生成**只读差异预览**。确保游戏当前不在战斗中；Crit 另须原 GPL 124B 模板。
5. 在界面明确确认已备份、仅离线单机使用；后端也要求固定的确认字符串。Preview token 30 秒，一次性；应用前再次检查原指令、会话和 SQLite。
6. 若两处写入与读回成功，Guest CPU 将通过 GDB 恢复运行。使用独立测试进度验证战斗结果。卸载时暂停 CPU、核对仍属于本工具的 Guest 字节、按逆序恢复并记录 SQLite 结果。
7. 任何 GDB 连接断开、ROM/SaveState 改变、第三方修改、Partial Write、RestoreFailed：停止测试，保留日志；若旧 Hook 无法验证，回到已备份的 clean ROM/存档。不要根据旧 Session 或 Host 地址强行恢复。

**未完成的真正验收：** Windows WebView2+mGBA 连接、RSP 真机兼容、完整 Guest ROM Hash 与源报告逐项对照、捕捉成功率、四版 Crit 正确性与战斗例外、运行中暂停/代码洞 CPU 位置、SaveState/ROM Reload 和异常恢复。因此，当前只能称“实现了实验性的真实 Guest 写入代码”，不能称“已经可靠完成真实金手指”。

## 代码复核优先项

- GDB RSP 对运行态的 Stop Reply 与 Packet ACK 时序，涉及实际 mGBA 版本差异。
- 多 Span 写中途失败的反序恢复与 SQLite 审计关闭；跨进程崩溃时不自动恢复。
- 仅靠 PC 不在 Cave 并不能证明返回地址、栈和正在执行的回调全部不再引用该 Cave。实验性 Crit 必须有真实单/双打、特殊状态与暂停点证据，不能仅以 Clippy/Windows CI 通过放行。
- 原源码 GPL-3.0 许可、用户本地模板和后续正式发行边界。

关联：[M2](m2-safe-patch.md) · [RS 安全研究](gen3-rs-migration-security-review.md) · [ADR-0005](decisions/0005-m3-gdb-guarded-live.md)。

## 2026-11-23 实际构建与自动化结果

- [CI #45118655477](https://github.com/969981/GameTrainer/actions/runs/45118655477)：**三组全部通过**。Rust 50 项单元测试，包含新增 fake mGBA GDB RSP 停止/Guest read/write/readback/continue 协议回归；React/Vitest 15 项；Chromium 3 个浏览器布局 E2E。
- Windows Tauri 2 Shell 的 check、桌面 Rust 单测和 debug exe 生成全部成功；[Windows x64 ZIP Artifact](https://github.com/969981/GameTrainer/actions/runs/45118655477/artifacts/14792413807)（artifact 名中 m0-m1 为历史 CI 名称，ZIP 内实际构建的是最新 M3 exe）。
- 修复了 Capture 的地区字母单独选择地址问题：必须具有严格 AXV/AXP GameCode 前缀。预览和 Apply 阶段均重新读取并核对完整 Guest ROM SHA1；不能悄悄切换到其它 ROM。
- SQLite app-local-data 无法解析时直接拒绝 live 连接，不再降级临时目录；多资源恢复可识别已有原字节，故障后保留 Owner 供诊断。
- **未通过/尚无的证据**：在真实 Windows WebView2 程序内手工操作 mGBA GDB Server、CRC/SHA 实机比对、捕捉/战斗行为、代码洞安全卸载、SaveState/ROM Reload 实测。Rust fake GDB 不等于真实 mGBA。

