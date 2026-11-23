---
date: 2026-11-23
project: GameTrainer
milestone: M7
status: diagnostic-only-no-live-writing
tags: [HGSS, DeSmuME, ARM9, ARM7, ROM, SHA1, JIT, Overlay12, fault-injection]
---

# M7 · HGSS 运行时 ROM 身份、双 CPU/JIT 和恢复故障门禁

## 事实基础及证据分类

原上传的 HGSS v1.4.5.1 CallflowTrace Multilingual FastSafe Source ZIP 中：
- `src/60_patches.lua`：捕捉码位于 Overlay12，原半字 `0x2801`、补丁半字 `0x4280`，前后各 16B 签名；CE 旧版允许在发现相同补丁时 adopt 已有半字。M6/M7 **明确禁止接管不属于自己会话的代码**。
- `src/35_mt.lua`：`pause()/unpause()` 停的是 CE 已打开的宿主进程，无法单独证明 ARM9/ARM7 的 Guest 调试同步或 JIT icache。
- `docs/V1451_THUMB_CALLFLOW_AND_TRACE.md`：静态 Thumb BL 分析、250ms 观察，不是已通过原生 Hook 实机验收。
- 用户提供的 ZIP 不包含 12 份完整 .nds ROM，因而本次不能独立复核完整 SHA1，也未能在此环境中运行 Windows DeSmuME 和真实游戏。

DeSmuME 官方资料：
- [官方 GDB Stub FAQ](https://wiki.desmume.org/?title=Faq)：Windows 通常须在 userconfig.h 中设置 `#define GDB_STUB` 并自编译，运行时可用 `--arm9gdb=3333 --arm7gdb=3334`。
- [CLI 实现（ARM9/ARM7 独立 Stub，部分构建禁用 JIT）](https://github.com/TASEmulators/desmume/blob/master/desmume/src/frontend/posix/cli/main.cpp)：该路径在 Stub 活跃且 JIT 打开时关闭 JIT；**不能推断每一种 Windows 构建也执行同样动作**。
- [GDB race 修复历史讨论](https://sourceforge.net/p/desmume/patches/167/)：已有对 GDB Stub/模拟线程竞态的公开讨论。因此两次 PC 不动只是局部证据，不能替代同步证明。

## M7 已实现代码（非真实 Guest 写入）

| 文件 | 行为 |
|---|---|
| [m7.rs](../crates/trainer-hgss/src/m7.rs) | 用十项后端 Blocker 构成完整否决式安全门禁；不同 ROM、CPU 标签错、缺失 GDB 停机、Overlay 不匹配或任何未知运行时身份都 fail closed |
| [Tauri m7.rs](../apps/desktop/src-tauri/src/m7.rs) | **顺序**连接两个 127.0.0.1 GDB Stub，分别读 Stop Reply 和 PC 双次一致性；如 ARM9 足够稳定，还按 M6 源数据只读查询 Overlay12 |
| [React M7](../apps/desktop/src/components/HgssM7SafetyPanel.tsx) | 12 已审计版本、独立 ARM9/ARM7 端口、双 CPU PC 结果、阻断原因、JSON 复制 |
| [M7 前端协议](../apps/desktop/src/lib/hgss-m7.ts) | 明确将 `write_authorized` 约束为 `false`，不提供写入按钮或任意 Guest 地址输入 |
| [M7 前端测试](../apps/desktop/src/lib/hgss-m7.test.ts) | 测试双端口范围、重复端口、假停机回执、读寄存器失败及永不获写入授权 |

### M7 明确无法自动证明的内容

**1. 运行中完整 ROM 身份**

本地用户手选 `.nds` 后计算 SHA1 只证明该文件本身，不证明 DeSmuME 正在执行的就是它。NDS 游戏卡 ROM 也不能假定作为一段线性 Guest 物理内存通过 GDB 的 `m` 完整读取。**M7 不会把用户输入、离线指纹或 Header Code/Rev 升格成运行中证明。**

未来需要可信的 **DeSmuME 自编译诊断桥**：由启动该 ROM 的同一 DeSmuME 核心读取实际加载的 ROM 完整字节，输出 Hash、Game Code、Revision、运行时 Session Epoch、进程 PID/Creation Time、调试 Stub 对应端口，并将报告与同一模拟器实例中的 ARM9/ARM7 客体状态绑定；切 ROM、载入 SaveState、重新启动都必须撤销旧证明。此桥**尚未实现/验收**。

**2. ARM9/ARM7 一致停机 + JIT**

单个 GDB Stop Reply、PC 两次相同、两个不同端口都停止，只能表示两处独立观察；即使它们全通过，也不能证明 2 个 CPU 在同一停机边界处于原子一致状态，或保证 Guest ARM9 指令缓存/JIT 转译视图。是否关闭 JIT、Guest 代码写入后的 cache invalidate、Resume 何时可见等必须针对确切 DeSmuME build 做实测。

**3. Overlay12 加载/卸载、身份与代码所有权**

只接受 v1.4.5.1 十二个静态审计 Profile。需要在同一可信进程中证明每次进入战斗时 Overlay12 是否重载、子管理器地址是否复用、加载 Epoch 如何变化；`original_verified` 只是静态签名一致，不能授权写入。外部已有的 `0x4280` 必须标记第三方拥有；不要 adopt 或自作主张恢复。

## 仅 Mock 总线的故障注入测试

`#[cfg(test)]` 模块在**合成 u16 内存**中模拟带 Epoch/Session 的 Intent 日志，不向真实 DeSmuME 写入。

- 正常：原值检查 → 注入模拟补丁 → 回读 → 同一 Epoch/Session 下恢复原值。
- ARM9 未停止／ROM 会话换代：写入/恢复前拒绝。
- 第三方预先写过目标／本事务写后外部修改：拒绝接管及覆盖。
- 写入前失败：Intent 不可视为已应用；禁止不检查目标就盲目恢复。
- **写已发生但 ACK 丢失**：日志保留 `Uncertain`，只有同一身份、停机、目标精确等于自己写入码才允许审慎恢复。
- 恢复时 ACK 丢失：即使模拟内存已恢复原值，也不能宣称事务完成、不能重复覆盖原值。
- 恢复读回异常：不能谎报回滚成功。

**这里模拟了恢复决策的失败处理，不等于真实 Overlay12 Guest 写回、异常断线或 SQLite 持久事务实测。**

## Windows 本地检查步骤

1. 使用带 ARM9 和 ARM7 GDB Stub 的 DeSmuME 开发构建，启动本人合法持有的原版 HGSS。备份存档和即时存档，不开启其他写入式金手指。
2. 让 ARM9 和 ARM7 调试器分别暂停，记录各自端口（例如 3333/3334）。
3. GameTrainer Windows Tauri →「运行时」→「M7 · ARM9/ARM7 + ROM 身份安全门禁」，精确选择 Game Code + Revision，输入两个独立端口并点击**只读预检双 CPU**。
4. 查看两颗 CPU 的 Stop Reply、PC、Overlay12 只读证据和全部阻断原因；复制 JSON。此处不会自动 Resume，需用自己的调试器继续运行。
5. 单独对野外→战斗→退出战斗→再次进战斗、SaveState 重载、切 ROM、断连和双端口同进程归属收集证据。只读采样不会触发真实代码更改。
6. 在可信 ROM 运行态桥、双 CPU/JIT 缓存实测和真实 Guest 持久故障恢复验收完成之前，**M7 写入能力保持关闭**。

关联：[M5](m5-hgss-v1451-readonly.md) · [M6](m6-hgss-arm9-overlay-proof.md) · [ADR-0009](decisions/0009-m7-runtime-rom-dualcpu-deny.md)。
