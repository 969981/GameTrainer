---
date: 2026-11-23
project: GameTrainer
type: implementation-note
status: experimental-read-only-not-emulator-validated
tags: [M5, HGSS, DeSmuME, NDS, Gen4, Rust, Tauri2]
---

# M5 · 心金／魂银 v1.4.5.1 · DeSmuME 只读接入

## 上游证据

分析输入为用户提供的 HGSS v1.4.5.1 CallflowTrace Multilingual FastSafe CT 及 Source ZIP。
此套上游是 DeSmuME + Cheat Engine 宿主内存实现，不是 mGBA GDB；不能照搬 M3/M4 的 GBA Guest 指令地址。
源码只支持日、欧版共 12 份 v1.4.5.1 静态审计；同时保留美版英语 HG/SS Rev0 两份历史 Profile（不属于本版 12 ROM 调用链审计），所以共 14 Profile。
韩版、未知 Revision、汉化补丁版或随机版不承诺兼容。项目不分发游戏 ROM 或上游原始 Hook 二进制。

## 已实现代码

- [14 ROM Manifest](../fixtures/hgss-v1451-profiles.json)：Game Code、Revision、SHA1 参考值、Current Seed / MT / MT Index / Trainer Pointer / Delay 地址。
- [纯 Rust trainer-hgss](../crates/trainer-hgss/src/lib.rs)：NDS Header 0x0C GameCode / 0x1E Revision，白名单查找、离线完整 SHA1 比较、单元测试。
- [Windows Provider](../crates/trainer-runtime-win/src/windows.rs)：DeSmuME 进程扫描、唯一本机主 RAM 映射探针、只申请 Windows QUERY 与 VM_READ 访问。
- [M5 Tauri IPC](../apps/desktop/src-tauri/src/m5.rs)：用户选择 PID 后附加、只读快照及断开；没有任何写入指令。
- [React UI](../apps/desktop/src/components/HgssPanel.tsx)：心金/魂银、DeSmuME 进程列表、RNG/MT/Delay/TID/SID、错误提示及本地 .nds SHA1 核验。
- [游戏 Registry](../fixtures/m0-registry.json)：新增 2 个 Gen4 游戏适配器和研究模块，不改变现有 GBA M3/M4 功能。

## 安全边界与限制

1. Windows DeSmuME 只读附加不等于完整 ROM 验证：进程端识别仅 Code/Revision 与 NDS RAM 状态锚点。本地文件 SHA1 属独立证据，并未绑定运行中镜像。
2. M5 只读 Current LCRNG Seed、MT[0]、MT Index、Delay 和 Trainer ID；MT[0] 不是 Initial Seed。未经 DTCM 证实不显示 Advances。
3. 上游 100% 捕捉是 Overlay12 动态生命周期补丁；穿墙、训练家视线属于 ARM9 原指令修改；MT 重播包含 624 状态字，不能用一个任意地址写入替代。
4. 传送、神兽事件及剧情 Flag 可能破坏状态机和存档脚本，不开放执行。
5. 必中、必会心、敌方 MISS、一击必杀等战斗 Hook 缺少已验证的 ARM9 trampoline / code cave / 卸载证据，未提供可执行开关。
6. 上游 BattleContext 250ms CE 采样不等于逐指令执行追踪。仍需 Windows + 真实 DeSmuME 模拟器联调，CI 编译不等于游戏行为验收。

## 操作

Windows 使用 Tauri 桌面版，打开 DeSmuME 并加载自己持有的原版游戏，先备份 .dsv/.sav 与 SaveState。
打开「运行时」→「DeSmuME · HGSS」，选择心金或魂银，然后刷新进程并选择 PID。
如内存布局识别成功会显示 Current Seed、MT State[0]、MT Index、Delay 和 ID；失败则拒绝连接，不尝试修改游戏。
也可通过本地 .nds 文件选择器验证完整 SHA1，计算完全在本机，不上传文件。
页面中的捕捉、穿墙、战斗 Hook 目前是研究清单，不应视为可用的作弊开关。

## 下一阶段验收

- [ ] Windows DeSmuME 0.9.x 各 32/64bit 版本验证 PID 与主 RAM 镜像定位
- [ ] 日版 HG/SS 与欧英 Rev10 的 ROM 身份、内存地址和 Seed 对照
- [ ] 切换 ROM、载入 SaveState、模拟器重启、扫描歧义时拒绝错误会话
- [ ] 与原 CT 在同一存档的 MT/Delay/TID/SID 数值对照
- [ ] 如需写入，独立验证 ARM9 暂停、运行中完整 ROM 身份、Overlay12 加载与恢复所有权、SQLite 故障恢复和游戏行为

关联：[M4](m4-frlg-v204-live.md) · [ADR-0007](decisions/0007-m5-hgss-read-only.md) · [[2026-11-23]]。
