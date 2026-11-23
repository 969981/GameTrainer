---
date: 2026-11-23
project: GameTrainer
milestone: M8
status: source-patch-tested-but-not-real-desmume-tested
tags: [HGSS, DeSmuME, ARM9, ARM7, ROM, SHA1, Windows, GDB]
---

# M8 · DeSmuME 内核加载 ROM 身份桥

## 目标和证据边界

GameTrainer 不能把用户在文件选择器选取的 NDS 文件认定为当前正在运行的 ROM。本版针对 DeSmuME 官方源码 b549f71b83287f265b6d74b9e77ef5d98a617190 增加**可选只读诊断桥**，从 NDS_LoadROM 已有的 ROM Reader 4096 字节 CRC 读取循环同步产生完整 SHA1；只接受将 ROM 整体载入内存的 Reader。然后由 Windows 验证桥服务 PID、进程创建时间，以及 ARM9 与 ARM7 的 TCP 端口是否由同一个 PID 持有。

**注意**：这比用户选择文件路径更可信，但不是密码学远程证明，不阻止同进程恶意代码注入，也不证明 ROM 加载后的任何外部内存修改。SHA1 用于与现有 12 ROM 历史指纹比对，不适合作为抵御主动 SHA1 碰撞攻击的签名。M8 不会授权任何 NDS Guest 写操作。

## 实现清单

| 路径 | 作用 |
|---|---|
| integrations/desmume/gt_hgss_m8_bridge.h | Windows C++11 原地 SHA1 流计算、ROM 加载代次、76B 命名管道快照 |
| integrations/desmume/0001-m8-identity-bridge.patch | 给官方 NDSSystem.cpp、Windows main.cpp 插入 Hash 与管道启停点 |
| integrations/desmume/install-m8-bridge.ps1 | 固定上游 commit 的 Windows 源码补丁安装器 |
| integrations/desmume/bridge_smoke.cpp | MSVC 合成 1 MiB ROM 数据流及卸载、拒绝磁盘流的测试 |
| crates/trainer-hgss/src/m8.rs | 76B 协议、ROM 版本 SHA1 验证及失败拒绝测试 |
| crates/trainer-runtime-win/src/m8_win.rs | GetNamedPipeServerProcessId / GetProcessTimes / GetExtendedTcpTable |
| apps/desktop/src-tauri/src/m8.rs | M8 只读 Tauri IPC |
| apps/desktop/src/components/HgssM8RomIdentityPanel.tsx | ROM 指纹及同进程端口诊断 |
| apps/desktop/src/lib/hgss-m8.test.ts | 前端输入验证与始终禁写的契约测试 |

相关源码：
- DeSmuME ROM Reader 与 NDS_LoadROM: https://github.com/TASEmulators/desmume/blob/b549f71b83287f265b6d74b9e77ef5d98a617190/desmume/src/NDSSystem.cpp
- DeSmuME Windows GDB Stub: https://github.com/TASEmulators/desmume/blob/b549f71b83287f265b6d74b9e77ef5d98a617190/desmume/src/frontend/windows/main.cpp
- 官方 GDB Stub 编译说明: https://wiki.desmume.org/?title=Faq

## Windows 开发构建步骤

1. 克隆 DeSmuME 官方仓库，并在此项目里切换到固定 commit b549f71b83287f265b6d74b9e77ef5d98a617190。否则源补丁可能不兼容；安装器默认拒绝其它 commit。
2. 在 GameTrainer 仓库执行：

   ~~~powershell
   .\integrations\desmume\install-m8-bridge.ps1 -DeSmuMESource "D:\WorkSpace\Project\desmume"
   ~~~

3. 根据官方 FAQ，启用 Windows 开发构建的 GDB_STUB；在统一的 C++ 编译配置中同时定义 GT_HGSS_M8_BRIDGE，必须对 NDSSystem.cpp 和 frontend/windows/main.cpp 都生效。
4. 在 DeSmuME 中启用将 ROM **完整预加载至内存**的选项，对应 CommonSettings.loadToMemory=true；否则即使 SHA1 读取完毕，仍拒绝认定流是稳定的内存 ROM。
5. 在 Visual Studio 编译修改后的 DeSmuME。下面是示例启动命令，需将路径改成实际 Windows 路径：

   ~~~powershell
   & "D:\Tools\DeSmuME\DeSmuME.exe" --arm9gdb=3333 --arm7gdb=3334 "D:\ROMs\HeartGold.nds"
   Get-Process *desmume* | Select-Object Id,ProcessName,Path
   ~~~

6. 启动 GameTrainer Windows Tauri；在运行时界面的 M8 面板里选择日/欧版 HGSS Code+Rev，填写 DeSmuME PID、ARM9 端口和 ARM7 端口。
7. 点击“从内核验证 ROM 身份”，复制返回的 JSON 证据并确认以下项目：
   - core_rom_stream_complete：内存驻留的 ROM Reader 流已完整计算 SHA1；
   - pipe_process_identity_verified：管道真实服务器 PID、Windows 进程创建 FILETIME 一致，读前读后未换进程；
   - gdb_socket_pid_binding_verified：Windows 的 TCP Owner-PID 表中两个端口都归该进程；
   - running_rom_digest_matches_catalog：ROM SHA1 + Game Code + Revision 匹配已静态审计的 12 个 Profile；
   - diagnostic_identity_verified：以上所有诊断条件成立；**write_authorized 仍为 false**。

当前仅支持 12 份 v1.4.5.1 静态审计的日/欧版。历史美版 Rev0、韩版、汉化版与未知修订一律拒绝通过。

## 协议和安全细节

Pipe 地址为 \\.\pipe\GameTrainer.HGSS.M8.<PID>，由模拟器内部线程创建，只提供一次 76B 二进制只读响应。固定字段按小端编码，包含魔数 GTHGSS08、版本、PID、创建 FILETIME、ROM Epoch、Code/Rev、大小、CRC32、SHA1、Loaded Flag、ARM9/ARM7 端口及加载时 JIT 状态。

Win32 客户端先检查进程是否在 DeSmuME 进程列表，再查询其创建 FILETIME；读取管道时调用 GetNamedPipeServerProcessId 获取**Windows 实际服务进程 PID**，之后通过 GetExtendedTcpTable 验证两个 TCP 端口的 OS Owner PID，最后重验进程创建时间。浏览器传给 Tauri 的 PID、端口或哈希均不得代替这些 OS 证据。

特别注意：
- 流式磁盘读取未被视为稳定内存来源。源码桥要求 ROM Reader 的内存缓冲区有效才发布加载身份。
- 每次 ROM 加载尝试或显式卸载都会撤销旧状态并推进 ROM Epoch。载入 SaveState 不必更换卡带 ROM，但会使 CPU/Overlay/补丁事务的状态可能失效；ROM Epoch **不是 Overlay Epoch**。
- 记录的 JIT 开关是加载时观察值，不等于整个 CPU 执行期间 JIT 一直关闭，更不是 ICache 写后可见性证据。
- 本地 Win32 PID / TCP 检查不是防本机恶意代码的强安全证明，不应将诊断桥开放到公网。
- 前端将 64 位 Windows FILETIME 以十进制字符串展示，避免 JavaScript 数值精度损失。

## M9 前必须补足的实机证据

- [ ] 在 Windows DeSmuME 诊断构建中加载真实持有的 HGSS 原版，核对核心 SHA1 / ROM Reader 与已知指纹。
- [ ] 验证多次切 ROM、ROM 加载失败、SaveState 进出和进程重启会撤销不再有效的会话/Overlay 所有权。
- [ ] 证明 ARM9 与 ARM7 处于同一同步停机边界，并确认该 DeSmuME 构建的 JIT、指令缓存失效和可执行内容一致性。
- [ ] 真实 Overlay12 进入/退出/重入、原字节、第三方补丁、写入读回、断线、延迟 ACK 和恢复故障注入。
- [ ] 加入正式版本的持久事务日志及恢复所有权策略前，禁止任何真实写入。

**结论：M8 交付运行中内核 ROM 来源、Windows 进程和 GDB 端口的只读绑定诊断代码；真实模拟器、同步停机与 Guest 写入恢复仍待本机验收。**

关联：[[m7-hgss-runtime-proof-gates]]、[ADR-0010](decisions/0010-m8-rom-core-pid-binding.md)、[[2026-11-23]]。
