# ADR-0008 · HGSS M6 ARM9 GDB 与 Overlay12 安全写入门禁

日期：2026-11-23
状态：Accepted（M6 只读诊断 + Fixture；真实写入关闭）

## 决策

1. 源 CE 的 pause/unpause 只能说明其宿主暂停尝试，不能推出 ARM9、ARM7 与 JIT 指令执行同步安全。先用官方支持的 DeSmuME ARM9 GDB Stub 生成独立 Guest 停机证据。
2. M6 只读协议限制为停止响应、PC/寄存器、固定地址 Guest 内存。前端不可发送任意地址或任何写包、复位指令、继续命令。
3. 12 份 HGSS 静态审计 Profile 固定 Guest FieldSystem、Overlay12 与指令邻域。只读辨识加载/退出、异常机器码与外部补丁，不把任何观察结果解释成写入许可。
4. 为防止误恢复第三方指令，M6 严禁沿用原 CT 对相同补丁码的 adopt 策略；未来真正写入必须有完整会话级所有权证据。
5. 恢复算法先用 cfg(test) 隔离 Mock 证明暂停、原字节、所有者、相同 Overlay 代次和读回的必要性，不开放真实 Guest 修改。
6. 正式写入前还需要运行中 ROM SHA1 与 Stub 目标绑定、ARM9/ARM7 停机和 JIT 缓存可见性、Overlay12 重载完整生命周期、真实断线/SaveState/SQLite 失败注入和原版 ROM 行为测试。

[M6 说明](../m6-hgss-arm9-overlay-proof.md)。
