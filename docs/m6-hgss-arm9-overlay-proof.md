---
date: 2026-11-23
project: GameTrainer
milestone: M6
status: read-only-debugger-probe-and-fixture-tests
tags: [HGSS, NDS, ARM9, Overlay12, DeSmuME, GDB]
---

# M6 · HGSS ARM9 停机、Overlay12 生命周期与安全恢复验证

## 证据和范围

本阶段以用户上传的 HGSS v1.4.5.1 CallflowTrace Multilingual FastSafe 源码及 CT 为基准，包括 src/60_patches.lua、61_multilang_patches.lua、62_cheats.lua 和对应 ROM 静态审计文档。源码中的 Cheat Engine pause/unpause 是**宿主进程暂停**接口，不是证明 ARM9 客体 PC、ARM7 同步停机或 JIT 指令缓存安全的完整依据。

DeSmuME 官方 FAQ 说明有 ARM9 GDB Stub；Windows 通常需要启用 Stub 的开发构建。普通 Windows 发行版未必支持，不能假定 mGBA GDB 与 NDS DeSmuME GDB 一样。参见：
- https://wiki.desmume.org/?title=Faq
- https://github.com/TASEmulators/desmume/blob/master/desmume/ChangeLog
- https://blocksds.skylyrac.net/docs/guides/debugging/

**实际已加入**：12 份 HG/SS 日版和欧洲 ROM 静态证据、固定地址的只读 ARM9 GDB 停机与指令采样、Overlay12 状态分类及跨采样比较、纯 Mock 的原指令恢复所有权测试、Tauri/React 操作面板和 JSON 证据复制。

**未完成**：无真实 Windows DeSmuME/ROM 动态采样证据、双 CPU 协调/缓存可见性验证、运行中完整 ROM SHA1 与 GDB 目标进程可信绑定、真实 Guest 写入/读回/回滚。因此生产代码依旧只读，任何采样都不能解除写入限制。

## ARM9 GDB 停止验证

源代码：apps/desktop/src-tauri/src/m6.rs、crates/trainer-hgss/src/m6.rs。

一条明确触发的 M6 命令只连接 127.0.0.1 上的 ARM9 Stub（端口至少 1024），发送 RSP 停止状态查询、读寄存器以及固定已知 Guest 地址的内存读取：**? / g / m**。要求 Stop Reply 且两次 PC 一致，CPU PC 落入 0x02000000–0x02FFFFFF。命令不暴露写接口，也不发送 c 恢复执行，不自动暂停或重新开始模拟器。停止后请用户使用调试器手动继续。

注意：这仅是**该 GDB 会话报告已停止**的证据，不能证明另一 CPU 已停止或 JIT/代码缓存可见性。连接没有和 M5 的 DeSmuME PID 做可信绑定。若连错端口/PC/版本或遇到连接中断，拒绝生成成功证据。

## Overlay12 证据来源及分类

见 fixtures/hgss-m6-overlay12.json；只允许本版静态审计的 12 个 Profile，排除美版英语 Rev0 两个历史 Profile、韩版、未知 Revision。

| ROM | FieldSystem 指针 | Overlay12 预期加载地址 | 捕捉指令 |
| --- | --- | --- | --- |
| IPKJ/IPGJ Rev0 | 0x021D3698 | 0x02236E00 | 0x02246B5A |
| IPKD/IPGD Rev0 | 0x021D4138 | 0x022378A0 | 0x022475F2 |
| IPKE/IPGE Rev10 | 0x021D4158 | 0x022378C0 | 0x02247612 |
| IPKF/IPGF Rev0 | 0x021D4178 | 0x022378E0 | 0x02247632 |
| IPKI/IPGI Rev0 | 0x021D40F8 | 0x02237860 | 0x022475B2 |
| IPKS Rev0 | 0x021D4178 | 0x022378E0 | 0x02247632 |
| IPGS Rev0 | 0x021D4198 | 0x02237900 | 0x02247652 |

沿 FieldSystem → ProcessManager → ChildOverlayManager+0x0C(Overlay ID) / +0x10(Exec State) 读取，识别 12 号 Overlay。激活时读取捕捉指令前后 16+16 字节的源证据签名及中间的半字：
- 未找到 Overlay12：not_loaded
- ID12 已出现但 Exec State 为 0：pending_execution
- 邻域符合且指令为原值 0x2801：original_verified
- 指令已经是来源补丁 0x4280：foreign_patched；**不继承所有权、不恢复外部代码**
- 邻域不符：signature_mismatch
- 其他目标指令：unexpected_instruction

当前不对 Guest 写入。页面读到 original_verified 也不等于验证完整游戏 ROM 的来源或允许打补丁。

## 真实联调操作矩阵

1. 备份本地 .dsv/.sav、SaveState；选择本人持有且完整 SHA1 已离线核对的原版 HGSS ROM。
2. 在具备 ARM9 GDB Stub 的 DeSmuME 开发构建中，开启 ARM9 调试端口。启动 GameTrainer Windows Tauri 应用，进入“运行时 → HGSS M6”。
3. **野外**通过 ARM9 调试器暂停，选择完全相同的 Code/Rev，执行 M6 采样并保存 JSON。
4. **继续 → 进入普通野生战 → 暂停**，再次采样，确认 Overlay ID 12、Exec State 非 0、捕捉邻域与原指令；不要求每次捕捉到短暂 pending_execution。
5. **继续 → 退出战斗 → 暂停**，再次采样；再次进战斗循环，检查相同地址复用、子管理器变更及指令重新载入。
6. 重载 SaveState、切换 ROM、切换语言、GDB 断线、模拟器重启分别检查是否会误把旧证据或旧指令所有权延续到新会话。
7. 如需进一步开放真实写入，另行证明双 CPU/Guest 停机、JIT/instruction cache、一致 ROM 身份、SQLite 事务、写后读回和错误时安全恢复。此阶段**禁止**真实 Patch。

## 仅在 Rust 单元测试中的 Mock 恢复

生产构建中没有相关写接口。crates/trainer-hgss/src/m6.rs 内的 cfg(test) 模拟事务使用一段隔离的 u16 值，必须满足以下条件才能在测试内应用/恢复：目标为精确原字节、模拟 CPU 停机、没有其他所有者、Overlay epoch 不变、恢复时仍为本事务写入的指令。

模拟测试覆盖：非暂停拒绝、外部补丁不接管、字节被篡改不恢复、epoch 变更不恢复、重复恢复拒绝、成功恢复原码。这不等于真实 DeSmuME Guest 内存验收。

关联：[M5](m5-hgss-v1451-readonly.md) · [ADR-0008](decisions/0008-m6-arm9-overlay-evidence.md)。
