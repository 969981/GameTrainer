---
date: 2026-11-23
project: GameTrainer
milestone: M10
status: implementation-and-ci-synthetic-only
tags: [Diamond, Pearl, DeSmuME, ROM, SHA256, Win32, ARM9, ARM7, Overlay11, JIT]
---

# M10 · 钻石／珍珠 DeSmuME 内核运行中 ROM SHA-256 身份桥

## 目标与证据范围

M9 只读读取的是 ROM Header、ARM9 指令地址和 RNG；这些字节并不能证明所连接的 DeSmuME 正在运行哪一份完整 NDS 原版。本阶段基于 **M8 DeSmuME 源码身份桥**，为 DP 独立增加完整 ROM SHA-256，绑定 Win32 进程身份、ROM 加载代次和双 GDB TCP 监听端口。采样时读取 ARM9 和 ARM7 **各自**的 Stop Reply/PC，并复核前后两份完整加载身份。它们仍**不是原子停机、JIT/ICache 或安全 Patch 恢复的证明**。

本次参考用户上传的原始文件：

- \`DP_DeSmuME_v0.8.14_GuestExecEvidence_Source_CT(1).zip\`，特别是 \`docs/rom_profiles_verified.json\`、\`docs/DP_MULTILANG_OVERLAY11_AUDIT_v0.8.11.json\`、\`docs/DP_GUEST_EXEC_ROM_AUDIT_v0.8.14.json\`、\`docs/DP_GUEST_EXEC_LIVE_EVIDENCE_AND_TWO_PHASE_UNLOAD_v0.8.14.md\`；
- \`DP_DeSmuME_v0.8.14_GuestExecEvidence(1).CT\`。
- 该来源中 **APAE Rev13、APAI Rev5** 有完整原版 ROM SHA256 和 Overlay11 SHA256 静态验证；其余 8 个 ROM 只有 ARM9/Overlay06 分量 SHA256。不得把分量 SHA256 当作整包指纹。

## 已提交的技术实现

| 位置 | 内容 |
|---|---|
| [gt_dp_m10_bridge.h](../integrations/desmume/gt_dp_m10_bridge.h) | 与 M8 同一个 DeSmuME NDS_LoadROM 流式 CRC 读取同时计算 DP SHA-256，捕获 ROM Header 0x6C，独立发布只读 \`GameTrainer.DP.M10.<PID>\` 命名管道 |
| [gt_hgss_m8_bridge.h](../integrations/desmume/gt_hgss_m8_bridge.h) | 同一 ROM 事件通知 DP SHA256；**保留 HGSS 原始 76B SHA1 协议和端口，不改变老协议语义** |
| [bridge_smoke.cpp](../integrations/desmume/bridge_smoke.cpp) | Windows MSVC 合成 1MiB 字节流：SHA256 精确结果、Header6C、89B 布局、ROM 加载 Epoch、卸载/非内存驻留拒绝、HGSS M8 兼容性 |
| [install-m8-bridge.ps1](../integrations/desmume/install-m8-bridge.ps1) | 一次性部署现有 M8 头文件和新增 M10 头文件；继续固定 DeSmuME 原始源码 commit |
| [trainer-dp/m10.rs](../crates/trainer-dp/src/m10.rs) | 89B 严格解码；完整 ROM SHA256 / Code / Revision / Header6C、OS 身份及端口验证；原始 ROM Epoch 前后比较；永远禁止真实写入 |
| [trainer-runtime-win/m10_win.rs](../crates/trainer-runtime-win/src/m10_win.rs) | Windows 获取真实管道 Server PID、进程 FILETIME、双 GDB TCP LISTEN Port 的归属；恢复前后二次校验 |
| [Tauri m10.rs](../apps/desktop/src-tauri/src/m10.rs) | \`m10_dp_core_identity\`、\`m10_dp_dual_cpu_overlay11_readonly\`：读内核管道前后快照 + ARM9 / ARM7 GDB 独立 PC 观察 + M9 只读 Overlay11 指令 |
| [React M10](../apps/desktop/src/components/DiamondPearlM10IdentityPanel.tsx) | 选择 DeSmuME PID、ROM 版本、ARM9/ARM7 端口，显示加载中 ROM SHA256、OS 归属、ROM Epoch、Overlay11 诊断和全部拒绝理由 |

## Win32 命名管道协议

**旧 HGSS M8 仍为 76B / SHA1 / \`GTHGSS08\` / \`GameTrainer.HGSS.M8.<PID>\`。** 新 DP M10 为 89B / SHA256 / \`GTPDPM10\` / \`GameTrainer.DP.M10.<PID>\`。

| Offset | 长度 | 字段 |
|---:|---:|---|
| 0 | 8 | \`GTPDPM10\` |
| 8 | 4 | 版本 \`1\` |
| 12 | 4 | 实际 DeSmuME PID |
| 16 | 8 | Windows Process Creation FILETIME |
| 24 | 8 | 本进程 ROM 加载 Epoch |
| 32 | 4 | ROM Code（例：APAE） |
| 36 | 1 | Revision |
| 37 | 1 | ROM Reader 第 0x6C 字节 |
| 38 | 8 | ROM Reader 完整字节数 |
| 46 | 4 | ROM CRC32 |
| 50 | 32 | 同一次字节读取计算的完整 SHA256 |
| 82 | 1 | loaded |
| 83 | 2 | ARM9 GDB Port |
| 85 | 2 | ARM7 GDB Port |
| 87 | 1 | 加载时 JIT 状态，仅观察值 |
| 88 | 1 | hash_complete |

整个 89B 协议采用 packed little-endian。仅 Windows 进程内的完整 ROM Reader、SHA-256 读取成功、全部数据已就绪且来源符合 10 个 DP Code 之一时，才发布有效 DP 快照；**要求 ROM 内存驻留**，拒绝用户选择的外部文件路径作为替代。

Rust Windows 端还需使用系统 API 验证 \`GetNamedPipeServerProcessId\` 返回的真实服务 PID、\`GetProcessTimes\` 创建时间、\`GetExtendedTcpTable\` 中处于 LISTEN 且归同 PID 的 ARM9/ARM7 端口。否则返回 fail-closed 结果。仅报告本机诊断一致性，不是加密远程证明。

## Windows 安装 / 联调

1. 使用受支持的 DeSmuME 源码固定版本 \`b549f71b83287f265b6d74b9e77ef5d98a617190\`。从 GameTrainer 仓库执行：

   ~~~powershell
   .\integrations\desmume\install-m8-bridge.ps1 -DeSmuMESource "D:\WorkSpace\Project\desmume"
   ~~~

   安装器会添加两个头文件（M8 HGSS、M10 DP）和同一上游代码插桩补丁。如果之前已安装旧 M8 桥，安装器不会直接覆盖已有文件；请在独立 Git 分支中人工审查新版 \`gt_hgss_m8_bridge.h\` 的变更和新增 \`gt_dp_m10_bridge.h\`，不要覆盖未经检查的本地改动。

2. 构建 Windows DeSmuME 时定义 \`GDB_STUB\` 和 \`GT_HGSS_M8_BRIDGE\`（第二个宏现在包含 DP M10 兼容扩展），并在 DeSmuME 配置中设置 \`CommonSettings.loadToMemory=true\`，让 ROM Reader 的完整内容固定在内存。
3. 参考官方 GDB Stub 构建文档，运行两个独立的本机 GDB 端口，例如：

   ~~~powershell
   & "D:\Tools\DeSmuME\DeSmuME.exe" --arm9gdb=3333 --arm7gdb=3334 "D:\ROMs\Pearl.nds"
   Get-Process *desmume* | Select-Object Id,ProcessName,Path
   ~~~

   示例路径需替换成实际路径。先关闭其他会主动修改 ROM Guest 内存的作弊工具；保存游戏备份和即时存档。普通官方 DeSmuME 未内置这些命名管道，不能直接使用本功能。

4. GameTrainer Tauri → 运行时管理 → **钻石／珍珠 M10**，选择正确 Code/Revision、DeSmuME PID、ARM9/ARM7 GDB 端口。
5. 第一按钮只读取 DeSmuME 的内核 ROM SHA256 和 Win32 进程/端口归属，不要求暂停 Guest。第二按钮在你**已使用外部调试器手动停下 ARM9 和 ARM7**的前提下，分别读取两个 GDB Stub 的 PC、Overlay11 原始 Callsite 和 72B Cave，然后再次读取 ROM 身份并比较 Epoch/指纹。**第二按钮不会替你停机、继续或写入。**
6. 导出 JSON，并记录实际 ROM 和 DeSmuME 版本、Windows PID、完整 SHA256、场景（野外/战斗中/战斗退出/再次进入）、是否载入 SaveState。如果停机结果不稳定或 Overlay 不可读，请记录而不是强行修改指令。

## 各版本身份验收能力

| 版本 | ROM 内核 SHA256 | 原版完整 SHA256 可直接匹配 | Overlay11 当前静态验证 |
|---|---|---|---|
| APAE Rev13 / APAI Rev5 | 可读取 | **是** | 有完整原版 Overlay11 来源（但不保证实时已加载） |
| ADAE / ADAF / ADAD / ADAI / ADAS | 可读取 | **否**，缺完整原版 SHA256 参照 | 缺当前版完整来源 |
| APAF / APAD / APAS | 可读取 | **否**，缺完整原版 SHA256 参照 | 缺当前版完整来源 |

在其余八个版本中，如果管道、PID 和 Byte Stream 都有效，会展示加载 SHA256，但 \`diagnostic_identity_verified=false\`，并显示「缺失完整原版 SHA256」。不能自动用一次本机结果填充「可信原版目录」。

## Overlay11 / 双 CPU / JIT 验证边界

- **Overlay11**：仅 APAE/APAI 有经原版 ROM 静态审核的 Callsite、Thumb BL 结构、72B Cave、Heap 和 D/I Cache 函数地址。程序只做读取，不发送写包。当前 72B Cave 全零也不表示可安全占用。
- **ARM9 与 ARM7**：两个 GDB Stop Reply、PC 前后稳定和 ROM Epoch 前后一致是**独立采样**；不会证明两核心在一个原子停机边界，没有自动停机/恢复 API。
- **JIT / ICache**：桥记录「ROM 加载时是否启用 JIT」，不保证加载后未更改，不证明 Guest 指令缓存被刷新，更不证明同地址 Thumb 重写对 CPU 可见。
- **动态 0x5A/0xA5**：v0.8.14 用户来源只证明原生 Thumb payload 静态构造与模拟状态机；真实 DeSmuME 还没有完整返回日志。**不自动尝试 Guest 执行**。
- **两阶段恢复**：来源描述 \`ENTRY-DISARMED\`→\`HOST-QUIESCENCE-OBSERVED\` 的三次宿主轮询，但明确说明 **不是 Guest PC 安全点**。在 CPU 的执行上下文和持久事务所有权未确认之前不能擦除代码洞。
- **SaveState**：本次 ROM Epoch 针对 ROM 加载/卸载，不追踪 SaveState 的 CPU 状态 Epoch；载入即时存档后必须将 Overlay11 和 Guest 指令观察视为失效，重新采样。
- **安全模型**：本机非对抗性开发诊断。Windows OS PID 和 TCP 归属不能防止同一进程代码篡改或恶意驱动。

## 自动化 / 实机验收矩阵

1. SHA256 原始 APAE/APAI 匹配 + 同进程双 GDB Port：\`diagnostic_identity_verified=true\`，**\`guest_write_allowed=false\`**。
2. 其它八个 ROM 的真实完整哈希即使从内核读出，也没有现成原版对照，不宣布原版身份通过。
3. 错 ROM、Header6C、Revision、内核 SHA256，或非 RAM 驻留、未读满 ROM：拒绝。
4. 另一个进程控制的 GDB 端口、连接已复用 PID、管道冒用、正在切 ROM：拒绝或报告 Epoch 变化。
5. 单 CPU 停机、ARM9/ARM7 PC 不稳、Overlay11 未加载：仍只读，列明各自证据缺口。
6. ROM 身份前后两次一致也**不是 SaveState Epoch 的证明**；实际 Overlay 退出重载、Guest JIT 可见性、两轮 0x5A/0xA5 与恢复故障注入仍未实机完成。

**最终门禁：本阶段 M10 无真实 Guest 写入，所有战斗 Hook、传送、穿墙、强制捕捉及代码洞清理均继续关闭。**

关联：[[m9-dp-v0814-readonly]]、[[m8-hgss-loaded-rom-bridge]]、[ADR-0012](decisions/0012-m10-dp-running-rom-identity.md)。
