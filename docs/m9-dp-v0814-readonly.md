---
date: 2026-11-23
project: GameTrainer
milestone: M9
status: readonly-gdb-static-source-no-live-desmume-evidence
tags: [Diamond, Pearl, NDS, DeSmuME, ARM9, Overlay11, RNG]
---

# M9 · 宝可梦钻石／珍珠 v0.8.14 GuestExecEvidence 接入

## 来源与实际证据

- 上传的 DP_DeSmuME_v0.8.14_GuestExecEvidence_Source_CT(1).zip 包含 127 个文件，包括 source/main.lua（与版本化主 Lua 相同）、CT、ROM/Profile 静态审计、Guest Thumb 汇编和测试。
- 独立上传的 DP_DeSmuME_v0.8.14_GuestExecEvidence(1).CT 与压缩包里的 cheat_table/DP_DeSmuME_CE_Tool_v0.8.14_GuestExecEvidence.CT 逐字节一致；单独 CT SHA256 为 8eabb78b8bde2d3027fae734a424b1787392d36e396b285440838d3a1d5b1580。
- main.lua SHA256 为 f0fdb5ed6cb1f1bdbfcf8c69ed35d22719255fae6dcf3beabdcbf122a7140db8。
- ROM 10 版本地址基线来自 docs/rom_profiles_verified.json 与 main.lua 第 178–259 行：Game Code、ROM Header Rev（0x1E）、Header 字节 0x6C、ARM9 和 Overlay06 单独 SHA256、Current/MT/Delay、Trainer/PID Pointer、Field、Warp、Collision/Cave。
- 不包含 10 份完整 ROM 的镜像；来源文件中的 ARM9_SHA256、Overlay06_SHA256 是单独分量指纹，不能当成完整 ROM SHA256。完整 ROM SHA256 静态来源只有珍珠 APAE 与 APAI 的相关原始镜像审计记录。
- v0.8.14 README 和 docs/DP_GUEST_EXEC_LIVE_EVIDENCE_AND_TWO_PHASE_UNLOAD_v0.8.14.md 明确：Guest 0x5A/0xA5 的真实 DeSmuME 执行没有得到，三次宿主轮询不是 ARM9 PC 安全点，三 Hook 共享 Dispatcher 尚未开放。

## 已实现

| 文件 | 已实现 |
|---|---|
| fixtures/dp-v0814-profiles.json | 10 个五语言钻石/珍珠 Profile，RNG/游戏全局/ROM Revision/双组件哈希；所有 write_allowed=false |
| fixtures/dp-v0814-overlay11-evidence.json | 仅 APAE/APAI 的 Overlay11 Callsite、会心/命中/OHKO 地址、Heap、DC/IC、72B Thumb Cave 静态审计 |
| crates/trainer-dp | Rust 独立 Profile、严格来源值校验、固定 Guest 读地址、ROM Header 和六个原版 ARM9 opcode 锚点检查、MT Index 与指针校验、只读 Overlay11 Thumb BL/Cave 当前观察 |
| apps/desktop/src-tauri/src/m9.rs | 用户触发 DeSmuME ARM9 GDB Stop Reply 与 PC 双次一致性检查，然后固定只读 m 请求，永不发送 GDB 写包、继续/恢复命令 |
| apps/desktop/src/components/DiamondPearlPanel.tsx | 钻石/珍珠切换、十个原版档案、指定本机 ARM9 端口、读取 LCRNG、MT State[0]、Index、Delay、Trainer Pointer；APAE/APAI 增加 Overlay11 静态研究面板、JSON 复制 |
| apps/desktop/src/lib/dp.ts | 离线 .nds 的完整 SHA256 校验：只在 APAE/APAI 有原版完整 SHA256 参考时能判断整包相符；其它八 ROM 明确提示缺少完整参考 |
| fixtures/m0-registry.json | 两个独立 Gen4 游戏适配器，各 8 个研究模块，13 个总游戏；没有任何执行补丁授权 |

说明：这是真实的 DeSmuME ARM9 GDB *只读采样路径*，但本机 GameTrainer 没有证明调试端口和操作系统进程、正在运行的完整 ROM 属于同一可信会话；没有真实 DeSmuME 动态游戏记录。用户的本地 .nds 文件 SHA256 结果也不可解除写入限制。

## 版本映射

| 游戏 | 语言 | Game Code | ROM Rev | Header 0x6C | Guest Exec |
|---|---|---|---:|---:|---|
| 钻石 | 英语 | ADAE | 13 | 0x31 | 未经实机验证 |
| 钻石 | 法语 | ADAF | 5 | 0x98 | 未经实机验证 |
| 钻石 | 德语 | ADAD | 5 | 0x6D | 未经实机验证 |
| 钻石 | 意大利语 | ADAI | 5 | 0x32 | 未经实机验证 |
| 钻石 | 西班牙语 | ADAS | 5 | 0x61 | 未经实机验证 |
| 珍珠 | 英语 | APAE | 13 | 0x4C | Overlay11 静态审计 |
| 珍珠 | 法语 | APAF | 5 | 0x81 | 未经实机验证 |
| 珍珠 | 德语 | APAD | 5 | 0xEB | 未经实机验证 |
| 珍珠 | 意大利语 | APAI | 5 | 0x9C | Overlay11 静态审计 |
| 珍珠 | 西班牙语 | APAS | 5 | 0x22 | 未经实机验证 |

日版、韩版地址在 Lua 中只属于旧探索条件分支，不在这 10 ROM 经独立审计的档案中，不应通过复制其它语言地址强行启用。

## 真实只读联调步骤（Windows）

1. 从 GameTrainer main 分支构建并运行 Windows Tauri2 可执行文件。
2. 使用支持 ARM9 GDB Stub 的 DeSmuME 开发版，加载本人合法持有的原版钻石或珍珠。建议关闭其他 Cheat Engine 附加和 Guest 写入补丁；先备份游戏存档及即时存档。
3. 在 ARM9 调试器手动暂停后，GameTrainer → 运行时 → DeSmuME 钻石／珍珠 M9，选择完全相同的 Game Code/Revision 和 ARM9 GDB 本机端口。
4. 点击只读采样 ARM9/RNG，检查 Source Header Code/Rev/6C、ARM9 的 Field/Warp/Collision 六个原版指令半字、Current Seed、MT/Delay 与指针范围。
5. APAE/APAI 仅额外读取会心调用点的 Thumb BL 结构和 72B Cave 零值；这些不是 Guest 已执行、Overlay11 安全驻留或代码洞可用的证明。
6. 可本地选择 .nds 计算离线 SHA256；有完整来源指纹的 APAE/APAI 版本可以作文件比对，但并不绑定 GDB 所指向的运行实例。
7. 复制 JSON 保存来源证据。采样后需要由外部 GDB 调试器手动继续 CPU；本程序不发送 GDB c/M/X 包。

### 后续安全前置

- 先扩展 M8 当前专属 HGSS 的内核 ROM 身份协议至 DP：从 DeSmuME 当前 ROM Reader 读取完整 SHA256（非仅外部文件）；绑定 OS PID/创建时间/GDB 两端口；支持 SaveState/ROM Epoch。
- 再用同一 DeSmuME 构建验证 ARM9/ARM7 同步暂停、JIT/D/I-Cache 与 Overlay11 加载/卸载/重入。
- 最后才可设计单一拥有者的 Guest Thumb Cave/Callsite 原字节保留、事务 Intent、原指令读回与异常恢复故障注入。当前工具不含任何真实 DP 写入/Hook。
- v0.8.14 的 HOST-QUIESCENCE-OBSERVED 是宿主轮询结果，不是 Guest CPU 已离开 Thumb Cave 的证明，不能用于直接擦除 Guest 动态代码。

关联：[[m8-hgss-loaded-rom-bridge]]、[ADR-0011](decisions/0011-m9-dp-readonly-guest-exec.md)。
