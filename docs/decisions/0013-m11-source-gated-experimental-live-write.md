# ADR-0013 · 先接入可审计的原版 DP 两字节真实写入，不宣称复杂 Cave Hook 安全

日期：2026-11-23
状态：实验性授权；未取得真正 Windows DeSmuME Guest 动态执行证据

用户明确要求真实开启捕捉、穿墙、传送、战斗 Hook 与 72B Thumb Cave。结合原始 v0.8.14 Source/CT，采用风险与证据隔离的分阶段决策：

1. APAE/APAI 两份有完整原版 ROM SHA256 的珍珠 ROM，可在用户明确选择高风险实验时进行穿墙原版半字 patch；英文 APAE 另有 EN16 捕捉半字 patch。
2. 提前 M10 内核 ROM SHA256/Windows PID/双 GDB TCP Owner PID/ROM Epoch 绑定，手动暂停双 CPU、PC 稳定、原指令签名和 interpreter-at-load=off。仍必须明确展示「不是原子停机／JIT 安全证明」。
3. 写入之前对原指令、ROM/session/feature/token 执行 fsync append-only journal；恢复只在确认当前字节匹配本工具持有的补丁及相同会话时执行，否则 fail-closed。
4. 传送、战斗动态 Hook 与完整 Cave 不是简单两字节 patch；原始来源明确缺 Guest 安全执行/卸载证据，不允许复用这些短指令能力伪造安全。未来引入可信 Core Safe Point + JIT Cache + Overlay Epoch + Guest callback quiescence 后另行提交。
5. 此时没有 Windows 真正模拟器运行结果，不应承诺应用可安全防崩溃或自动恢复。

参见 ../m11-dp-live-patch-lab.md。