# ADR-0009 · M7 不以离线 SHA1 和独立 GDB Stop Reply 授权 HGSS 实写

日期：2026-11-23
状态：Accepted — fail-closed diagnostics; no NDS production writes

## 决策

1. M5 选定的文件 SHA1、Game Code/Rev 或内存锚点都**不是正在运行 ROM 的完整来源证明**。
2. M6 ARM9 以及 M7 ARM9/ARM7 双本机 GDB `?`/`g` 响应，只能证明两个调试器分别返回了停机证据，不能证明同一 DeSmuME 进程、同一同步停机 Epoch 或 JIT/icache 安全。
3. M7 后端公开 `write_authorized=false` 和固定否决条件。生产代码不实现任何 NDS WriteProcessMemory、Guest GDB M/X 包或 Hook。
4. Overlay12 原指令/邻域一致不能接管外部补丁。模拟事务需相同 Session/Epoch、目标仍为自己的 Patch、读回成功；失败必须保持 Uncertain、禁止盲恢复。
5. 只有可信模拟器端运行中 ROM 完整字节指纹绑定、ARM9/ARM7 原子停机及 JIT/缓存机制、Overlay12 重入/退出和真实断线/SaveState/SQLite 回滚实测全部验收后，才可考虑另开 M8 的真实写入通道。

详细：[M7 技术说明](../m7-hgss-runtime-proof-gates.md)。
