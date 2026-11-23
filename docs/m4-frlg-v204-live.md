---
date: 2026-11-23
project: GameTrainer
type: implementation-note
status: ci-passed-real-emulator-not-tested
tags: [M4, FRLG, Gen3, mGBA, GDB, Hook, Capture]
---

# M4 · FireRed / LeafGreen v2.0.4 真实 Guest 写入实验

> **来源**：用户上传的 Pokémon FRLG All-in-One CE v2.0.4 ScanWildHookFix 源码 ZIP 和 .CT。经实际检查 ZIP 的 Lua、README 与 116B wild_payload.bin。这里记录的结构、功能和地址来自这份上传源码，不等于本次已经拿到游戏 ROM、验证了模拟器实机运行。

## 产品接入

- 游戏库新增 FireRed 和 LeafGreen 两个游戏，采用既有的 `GameAdapter / FeatureDescriptor` 架构，保留五种平台分类，游戏总数扩展到 9。
- [FRLG profile Rust](../crates/trainer-frlg/src/lib.rs) 和 [16 项原版 ROM Manifest](../fixtures/frlg-v204-roms.json)：8 个 BPR 火红和 8 个 BPG 叶绿，英文/日文 Rev0/Rev1，其余德/法/意/西零售 Rev0；每个 Profile 绑定完整 SHA1、GameCode/Revision、Capture、Wall、Wild entry、callback2 和 Seed 地址。
- [FRLG 实时事务引擎](../crates/trainer-frlg/src/live.rs)、[受信任的 Tauri IPC](../apps/desktop/src-tauri/src/m4.rs)、[React M4 面板](../apps/desktop/src/components/FrlgLivePanel.tsx)。
- 通过 mGBA 本机 GDB Remote Serial Protocol 在**Guest CPU 暂停**时读写；严禁接受前端自填的任意补丁地址和任意 Thumb 指令。
- M3 红蓝宝石和 M4 火红叶绿的 GDB Session **互斥**，不能同时连接写入；M4 独立 SQLite 日志位于 App Local Data 的 `m4-frlg-live-history.sqlite3`。

## 从原 v2.0.4 源码迁移的三项实验

| 功能 | 上游证据 | M4 行为 | 验证级别 |
|---|---|---|---|
| 100% 捕捉 | `Cmd_handleballthrow` 固定 `D92F`（Thumb BLS） | 安装前核对 10B 指令上下文和捕捉函数前 8B，改为 `46C0`（NOP）；按所有权恢复 | Rust 编码/静态与 mock 测试；游戏实际捕捉未测试 |
| 穿墙 | `20C0` -> `2100`，各版地址不同 | 核对 10B 指令上下文，暂停写入+回读；卸载核对原/本工具字节 | Rust 编码/静态与 mock 测试；地图效果未测试 |
| 普通野生 Seed Hook（v2.0.4） | 116B Thumb Payload、`TryGenerateWildMon` 原始 8B、`0x0203FFC0` 16B Mailbox | 原包本地 Payload SHA256 校验，入口跳板最后写；按单次模式给普通野生遭遇指定 32-bit Seed；触发后读命中数与上次 Seed | **高风险实验 Hook，尚未完成真正 mGBA 游戏行为/卸载/SaveState 证明** |

源码 Payload 的大小是 116 字节，SHA256 为 `83a069853f427e86bc62c2ed3a56741c94ed13301d58321b02a5ac39698fed6a`。其机器码不会随 GameTrainer 发布；UI 从用户自己的 ZIP 解压目录读取 `wild_payload.bin`，Rust 严格校验哈希、占位符及来源约束，不允许任意 Payload。

Hook 安装顺序：暂停 CPU → 核对完整 ROM SHA1、Header/Rev、函数原字节/代码洞、Mailbox 全零 → SQLite Intent → Mailbox 初始化 → 代码洞写入+回读 → 函数入口跳板**最后生效** → 恢复运行。卸载先暂停、检查 gMain `inBattle` 标志和 PC 不在代码洞，入口**先恢复**，再清理代码洞及 Mailbox，SQLite 记录确认。

**普通遭遇仅 `flags==1`；Sweet Scent 是独立路径，没有并入 M4。** 源 v2.0.4 的 ScanWildHookFix 还处理了命中时将 flags 暂设为 0 避免 Repel 把野怪消费的问题；这里沿用上传模板原字节而不是自行改编 Hook 算法。没有验证所有战斗分支、双打状态或旧 SaveState 残留。

## Windows 使用步骤

1. 用 mGBA 打开自己合法获取、**未修改**的 FireRed/LeafGreen ROM，备份 .sav 和 SaveState，使用可丢弃的离线测试进度。不要联机或同时运行 Cheat Engine/其它 Hook 工具。
2. 打开 mGBA 的本机 GDB Server，确认 Guest CPU 停止，通常测试端口为 `2345`；在 GameTrainer → 运行时 →「火红／叶绿 · 真实写入实验室」连接。
3. M4 读取完整 16MiB Guest ROM 并计算 SHA1；必须匹配 16 个来源 Profile 中当前 Code/Rev 的完整 SHA1，不能只凭 CRC32 或游戏名称。
4. 选 100% 捕捉 / 穿墙 / 野生 Seed Hook，执行只读差异预览（300 秒一次性令牌）。后者还需本地选择 ZIP 解压出的 116 字节 `wild_payload.bin`。
5. 在 UI 中确认已备份且仅离线单机实验，Rust IPC 也独立验证确认文本；先持久化 SQLite 操作 Intent，再尝试写入、Guest readback，确认后恢复执行。
6. 野生 Hook 安装后输入 8 位十六进制 Seed，另行勾选单次确认并点击执行。可暂停 CPU 查询邮箱触发计数/旧 Seed，再继续运行；结束后通过「暂停并验证恢复原指令」卸载。
7. 任意 ROM Reload、SaveState 旧跳板、未知写入/Hook、GDB 连接失联、RestoreFailed：**停止执行并保留审计记录**，不要跨进程自动回写原地址；从干净备份恢复模拟器状态。

## 全 ROM 校验性能说明

首次连接和每次预览、Apply、Restore 都会通过受限 GDB RSP 流式读取完整 16 MiB Guest ROM。mGBA 协议分包可能耗时较长，因此 M4 Preview Token 设为**300 秒（5 分钟）一次性**，而不是 30 秒；到期就拒绝应用，不会跳过重新计算 ROM SHA1 来延长有效期。

## 不在本次自动启用的功能

- 旧 CE 包包含地图导航/传送、游走三圣兽跟踪、队伍与盒子、遭遇与 Fishing 等更多模块。M4 当前只新增它们的游戏菜单或来源研究链接；**地图传送、随意精灵生成、钓鱼与游走 Hook 均未开启真实写入**，因为存在更复杂的状态机、存档和跨场景生命周期。
- 新增功能 **只限 GBA mGBA**；不是 Windows Native MinHook，不会影响现有 NDS、3DS、Windows Native Template 的写入状态。

## 验收与安全红线

M4 允许在 Rust 停机事务里发送**真实 GDB Guest 写包**，但**源包测试、单元测试、Windows CI 编译，均不等于真的 mGBA + FireRed/LeafGreen ROM 游戏实测**。必须另外完成：

- 实测不同语言/Rev 完整 ROM SHA1，确认原版 ROM 映射、ROM patch 与 GDB 中断帧状态。
- 捕捉：战斗、训练家阻断、成功/失败演算、存档持久性和恢复。
- 穿墙：地图切换、脚本切换、建筑入口、碰撞恢复。
- Wild Hook：普通野生 vs Sweet Scent、Repel、0 命中/多命中、PC 在代码洞停止时拒绝卸载、战斗期拒绝操作。
- SaveState、ROM Reload、模拟器崩溃、中途 GDB 失联、SQLite 写失败、第三方修改、部分恢复失败的安全诊断。UI 操作轨迹和恢复证据需要单独保存。

关联：[M3](m3-live-gdb-experimental.md) · [ADR-0006](decisions/0006-m4-frlg-experimental-live.md) · [测试矩阵](testing.md) · [[2026-11-23]]。

## 2026-11-23 M4 CI 与 Windows 调试版验收记录

- [GitHub Actions #45121396498](https://github.com/969981/GameTrainer/actions/runs/45121396498)：Rust Core、React UI/Chromium、Windows Tauri 2 **三组都成功**。Rust 57 项单元测试，前端 ESLint/TypeScript/Vitest 与三档 Chromium E2E 通过。
- Windows `cargo check -p game-trainer-desktop`、`cargo test -p game-trainer-desktop --lib`、`cargo build -p game-trainer-desktop` 成功，产出 [M4 debug EXE ZIP Artifact](https://github.com/969981/GameTrainer/actions/runs/45121396498/artifacts/14794430558)。Artifact 的展示名 `gametrainer-m0-m1-win64-debug` 为 CI 的历史命名，不代表旧版本代码。
- 本次构建源码 SHA `c0f42ab7024f881757c7783f30edeb2c2df68646`，后续仅补充文档 [skip ci]，未改编译源码。
- **未完成**：Windows 图形界面人工点击/连接 mGBA GDB、用本人合法原版 16 种 ROM 的 GameCode/Rev/SHA1 验证、真实捕捉/穿墙/野生 Hook 表现、SaveState/ROM Reload、恢复失败注入。绝不把 57 项 Rust 合成/单元测试当成 GBA 实机验收。

