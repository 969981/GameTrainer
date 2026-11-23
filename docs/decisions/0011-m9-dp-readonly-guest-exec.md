# ADR-0011 · DP v0.8.14 先迁移十个版本的只读诊断，暂不迁移 Guest Thumb 实写

日期：2026-11-23
状态：Accepted（Readonly DP M9，未获实机写入安全认证）

原始资料：用户上传 DP v0.8.14 GuestExecEvidence Source/CT；十 ROM 来源 docs/rom_profiles_verified.json；Overlay11 来源 docs/DP_MULTILANG_OVERLAY11_AUDIT_v0.8.11.json 与 docs/DP_GUEST_EXEC_ROM_AUDIT_v0.8.14.json。

决策：
1. Diamond/Pearl 各拥有单独 Gen4 GameDescriptor，但共享一个源码档案严格限定的 Rust GuestReader；只允许 10 个独立原始 ROM Code/Revision/Header6C 配置，日/韩旧地址不直接用于当前代码。
2. ARM9 GDB 只用于停止状态、PC、静态 Header、固定 ARM9 指令和只读 RNG 观测；Header 与 opcode 锚点不是完整 ROM 身份，不能授权内存修改。
3. 来源 10 个 ARM9/Overlay06 SHA256 是 ROM 分量指纹；完整 ROM SHA256 静态基线只对珍珠 APAE/APAI 存在。区分两种证据，绝不伪造其余八份完整 ROM 指纹。
4. Overlay11 原版 Callsite、Heap/Cache/Cave 有 APAE/APAI 的静态验证；Guest Thumb 0x5A/0xA5 的实机返回、同地址代码缓存可见性、双阶段卸载的 Guest PC 安全点都未证明。不得把 3 次宿主计时器轮询当作 ARM9 安全点。
5. 真实 Guest 写包、Hook、清理 Thumb Cave、自动继续 CPU 暂不实现；功能模块只做 read-only 研究。后续须独立验收 DP 内核运行态 ROM 来源、ARM9/ARM7 + JIT 停机、Overlay11 reload、单一所有者的原字节恢复和真实故障注入。

见 [M9 验证指南](../m9-dp-v0814-readonly.md)。
