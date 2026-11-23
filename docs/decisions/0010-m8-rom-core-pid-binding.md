# ADR-0010 · HGSS DeSmuME 内核 ROM 身份与 Windows OS 绑定

日期：2026-11-23
状态：Accepted（仅开发诊断；真实 NDS 写入禁用）

**背景**：手工选择文件和离线 SHA1 不等于 DeSmuME 运行中 ROM，也不能证明 GDB 两端口来自同一 Windows 进程。

**决策**：
1. 在可选 Windows GDB_STUB + GT_HGSS_M8_BRIDGE 构建的 NDS_LoadROM 核心读取循环同步计算 SHA1，校验读取完整性并要求 ROM 内存驻留。拒绝不完整、磁盘流、非 HGSS。
2. 在模拟器内建立只读 Named Pipe，输出 76 字节含 ROM Epoch 的二进制快照。任何加载尝试及卸载先撤销旧快照。
3. GameTrainer 在 Rust 后端通过 Win32 Query 验证真实 Pipe Server PID、Windows Process Creation FILETIME、TCP ARM9/ARM7 监听端口 OS Owner PID，核对 12 个已审计 ROM 版本 SHA1。PID 复用与跨进程、不同端口默认拒绝。
4. FILETIME 64-bit 精确值向 JavaScript 展示时改为字符串，OS 比对继续在 Windows Rust 内执行。
5. 以上证明只在“本机可信无恶意进程”的诊断威胁模型中成立。它不认证两 CPU 同步停机、实时 JIT/ICache、SaveState/Overlay Epoch 和真实 Patch 恢复。
6. 生产后端不提供 NDS Guest 写入函数，诊断身份匹配时仍返回 write_authorized=false。

详见 [M8 安装与实机验收](../m8-hgss-loaded-rom-bridge.md)。
