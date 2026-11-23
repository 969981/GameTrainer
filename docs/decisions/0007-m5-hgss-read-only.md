# ADR-0007 · HGSS NDS DeSmuME 独立只读 Provider

日期：2026-11-23
状态：Accepted（仅受控只读实验能力，尚无真实 DeSmuME 联调）

1. HGSS 原 CE 工具通过 DeSmuME 宿主内存读取 NDS Guest 数据。不能以 GBA/mGBA 的 GDB 地址当作 NDS 内存偏移。
2. 首版独立 M5 Windows Provider 只申请 VM_READ / QUERY 权限，拒绝写入和任意用户指定内存地址，避免未确认的 ARM9/Overlay12 补丁执行。
3. 目标进程必须由用户选择；检查进程创建时间、精确 GameCode 和 Revision、MT/关键指针的可读性及候选映射唯一性。任何歧义默认失败关闭。
4. 保留 14 个来源明确的 HGSS Profile；12 份本版静态审计，2 份美版历史资料。离线 ROM SHA1 与模拟器运行中身份分开展示，不授予写入权限。
5. 上游战斗 Callflow 定时采样并不是实时 CPU Trace，必会心、必中、OHKO 等并非已实现的可恢复 Hook。
6. 真正开放写入前必须证明 ARM9 暂停和代码缓存可见性、Overlay12 生命周期、ROM 身份绑定、原字节和回读、故障注入与安全卸载。

实现记录：[M5](../m5-hgss-v1451-readonly.md)。
