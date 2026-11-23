# M11 · 钻石／珍珠实验性真实内存补丁

日期：2026-11-23。状态：代码与模拟器自动构建可测试；真实 DeSmuME 游戏执行与恢复尚无实机日志。

## 原始来源
用户上传的 DP_DeSmuME_v0.8.14_GuestExecEvidence_Source_CT(1).zip：
- src/main.lua 的 T.WALL_COLLISION_HOOKS、T.wallCollisionSignatureOK：PlayerAvatar Collision Return 原指令 1C20 (20 1C) 改为 2000 (00 20)，核验前 8 字节与后 4 字节。
- T.catchProfiles、T.applyCatch、T.disableCatch：仅英文 EN16 的 0x0223C1F4 原比较 2801 (01 28) 改为 4280 (80 42)；需对应 Battle Overlay 实际加载。
- T.nativeTeleport、T.startBattleCrit/Hit/OHKO 与 T.startGuestExecProbe 均涉及动态 Guest Thumb 与 72B Cave。docs/DP_GUEST_EXEC_LIVE_EVIDENCE_AND_TWO_PHASE_UNLOAD_v0.8.14.md 指出真实 Guest 0x5A/0xA5、ICache/JIT、退出时 Guest PC 安全点尚未实机验证。三次宿主定时轮询不是安全退出证明。

## 已接入的实际写入
| 功能 | ROM | 写入方式 |
| --- | --- | --- |
| 穿墙 | APAE Rev13 / APAI Rev5 | GDB M 写入 16-bit 原指令 20 1C -> 00 20，回读后可按日志恢复 |
| 英文 100% 捕捉 | APAE Rev13 | GDB M 写入 01 28 -> 80 42，回读后可按日志恢复 |
| 钻石 ADA* 与其余八个无完整 ROM SHA256 的版本 | 不支持 | 禁止根据 ARM9 或 Overlay06 分量哈希开启 |
| 72B Cave 空白区暂存与清理 | APAE/APAI | 实验性真实写入 35 条 Thumb NOP + BX LR（共 72B）；确认四个已知入口均为原版后按所有权清零，不安装任何调用点 |\n| 原生传送、会心/命中/OHKO、72B Guest 动态 Thumb 执行与热卸载 | 静态原型 | 尚未可靠接入运行中 Guest；不得把 Cave 暂存等同于真正 Hook 安全 |

这是真实 Guest 写包，不是 Mock。72B Cave 测试只验证受控写入/回读/原字节恢复，**不会修改任何跳到 Cave 的调用入口，也不会执行 0x5A/0xA5 Guest Thumb 回调**。Windows 实机运行、GDB Stub 兼容性、JIT 缓存一致性、Overlay 生命周期仍需现场验证。

## Windows 使用与恢复
1. GameTrainer main 构建 Windows Tauri；将 DeSmuME 固定源码版本按 docs/m10-dp-loaded-rom-sha256-and-overlay11.md 安装 M8/M10 Core ROM 身份桥，开启 GDB_STUB + GT_HGSS_M8_BRIDGE 和 ROM 完全载入内存，在载入游戏前关闭 JIT。
2. 备份合法持有的原版珍珠 APAE/APAI 的 .sav 和即时存档；禁用 Cheat Engine 等其它写入工具。启动 ARM9 与 ARM7 的不同本机 GDB 端口（例如 3333/3334）。
3. 手工暂停 ARM9、ARM7；GameTrainer 运行时管理 -> M11：选择 ROM、DeSmuME PID、两端口和穿墙/捕捉。先点「预览本机原指令（不写入）」。
4. 后端核对 M10 当前 ROM 完整 SHA256、Named Pipe 真 PID/进程创建时间、两 GDB TCP LISTEN 端口同 PID、ROM Epoch，两个 GDB Stop Reply 与双次 PC、JIT-at-load=off。穿墙必须通过 14B 周边指令签名。验证失败即拒绝。
5. 选择「72B Cave 写入／清理」时，需进入 APAE/APAI 战斗并等待四个原版入口可读；程序核对原入口、全零 72B 区域，再写固定 Thumb NOP/BX LR 图像而不安装执行入口。恢复仅清零自身拥有的 72B 图像。用户明确勾选实验风险确认后，点击「确认并执行真实 Guest 写入」。写前对原指令和会话信息保存持久化 prepared 记录，再 GDB M 写、m 回读。成功 applied，异常 uncertain。之后可由外部调试器手动继续游戏。
6. 恢复前再次手动暂停两个 CPU，在相同 ROM/PID/创建时间/ROM Epoch 下点击「从持久化日志恢复原指令」；仅在现场字节恰好等于本工具拥有的补丁时恢复并 readback。
7. 持久化日志位于 %LOCALAPPDATA%\GameTrainer\DP-M11-journal\<CODE>-<FEATURE>.jsonl；发生 ACK 丢失、异常断连或 ROM/Overlay 更换时，不得猜测覆盖地址或删除日志。改由备份/即时存档恢复。
8. 退出 GameTrainer 不代表模拟器代码自动恢复。本模式要求用户手工核验、恢复、再继续运行。

## 尚未达到的门槛
ARM9/ARM7 独立 Stop Reply 不是原子同步停机，JIT 加载时状态不是当前可执行缓存安全证明。**独立 Cave NOP 暂存**不等于可删除真正动态 Hook：原执行回调可能停在尚未返回的尾声，恢复入口也不代表能擦除动态代码。未来须增加 DeSmuME Core 双核安全点、Overlay11 epoch、可靠回调出栅栏、同地址缓存一致性、故障注入和持久日志验证才能开放高级功能热注入/热清理。