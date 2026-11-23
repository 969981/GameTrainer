# ADR-0012 · DP M10 使用独立 SHA256 内核桥，安全诊断与 Guest 写入完全隔离

日期：2026-11-23
状态：Accepted / Read-only diagnostics only

## 决策

1. 继续使用 DeSmuME 的 **相同 ROM Reader 核心读取流程**，但 DP 另建 \`GTPDPM10\`、89B 与 \`GameTrainer.DP.M10.<PID>\`。原 HGSS M8 的 SHA1/76B 管道不改变。
2. 从流式 ROM Reader 采集 Code、Revision、Header6C 和完整 SHA256；必须拥有内存驻留 ROM，读满才公布 \`hash_complete\`。这避免把用户选择的离线 ROM 文件当作运行中的 ROM。
3. Win32 侧独立验证管道真实服务 PID、Process Creation FILETIME、双 GDB LISTEN TCP Port 的 PID 归属；在两个 GDB Stop/PC 采样前后再读一次 ROM 身份，并要求 ROM Epoch/原始指纹与 OS 身份一致。
4. 原 DP v0.8.14 用户资料中只有 APAE/APAI 有完整原版 ROM SHA256，其他八个 ROM 的 ARM9/Overlay06 SHA256 **不是**完整 ROM 指纹。它们可以读取运行中 SHA256，但不能宣称与可信原版匹配。
5. 双 CPU 独立 Stop Reply、加载时 JIT Flag、静态 Overlay11 Thumb BL/Cave 检查均是局部诊断，不能证明 ARM9/ARM7 原子停机、实时缓存一致性、Overlay 重入/卸载 Epoch、同地址 Guest Thumb 动态执行或安全回滚。
6. 不提供 Guest 内存写入、Hook、自动 Resume、Cave 删除或模拟器进程暂停能力；始终 \`write_authorized=false\`。

参见 [M10 技术与 Windows 验收说明](../m10-dp-loaded-rom-sha256-and-overlay11.md)。
