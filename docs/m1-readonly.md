# M1 · Emerald/mGBA 只读探测（第 1 迭代）

日期：2026-11-23
状态：**M1 只读源码已提交，Rust 9 项核心测试和 Windows Tauri 编译已通过**。窗口 GUI/真实 mGBA 进程 + ROM 读取仍待实机验证。[CI](https://github.com/969981/GameTrainer/actions/runs/45107272587)。

## 来源和许可证
依据用户提供的 `Pokemon_Emerald_v5.3.0_Full_Source(1).zip` 中 `src/main.lua` 的参数和数据布局重新组织，只保留原版 ROM 的指纹/地址等事实。该上游 Lua 源码明确采用 GPL-3.0，任何复制源码/改写代码再发行必须审查许可证。当前代码没有将原 Lua 文件或 ROM/存档纳入仓库，工程许可证尚未定稿。

## 已编码的只读功能
- 独立 `crates/trainer-mgba` crate，不依赖 Tauri，不提供写内存 API。
- GBA ROM 头：游戏标题前缀、4 字符 code、Rev0、0xB2 固定值、GBA Header checksum。
- 六语言 Profile：BPEJ/JPN、BPEE/ENG、BPED/DEU、BPEF/FRA、BPEI/ITA、BPES/ESP；按 JPN/INTL 分配 Seed、Initial Seed、Advances Guest 地址。
- **部分**原版 ROM 锚点：wild、fishing、SetWarp、DoWarp、Sweet Scent land/water。校验失败则拒绝建会话；没有全量 15 项 ROM entry 校验或全 ROM CRC，不得称为完整官方验证。
- `GuestMemoryReader`：底层运行时必须提供只读 Guest 内存读取及独立重新获取的进程/内存映射指纹。
- `EmeraldReadOnlySession`：绑定 pid、进程创建时间、ROM/EWRAM/IWRAM host base、pointer slot、game code、rev；变化立即 Stale 并永久失效，禁止复用旧会话。
- Seed Snapshot：按 32 位 little-endian 读取 Current Seed 和 Advances，按 16 位读取 Initial Seed；读前/读后双重身份校验，防止跨 ROM Reload 的混杂结果。
- 仅返回 `ProbeStrength::PartialAnchorsReadOnly`，不会生成写入授权令牌。

## 旧代码数据参考（Source Evidence）
- `LAYOUTS.JPN`: seed=0x03005AE0、initial=0x02020000、advances=0x02024664。
- `LAYOUTS.INTL`: seed=0x03005D80、initial=0x02020000、advances=0x020249C0。
- Header：0xA0 起标题，0xAC 游戏 code，0xBC revision。
- 六个 ROM Profile 的 crc32 字符串仅作为来源期望值记录，**M1 未计算完整 ROM CRC**。
- 旧版 CE 的 GBAMemory 指针槽和候选评分逻辑依赖具体 mGBA 宿主映射；迁移时必须独立建立多候选评分、歧义拒绝和进程创建时间校验，不能简单复用 PID 或固定地址。

## 当前 Windows + UI 实现（代码已提交，非实机验证）

- `crates/trainer-runtime-win/src/windows.rs`：仅识别 mGBA 可执行文件、ToolHelp 进程枚举、PID 创建时间校验；只请求 PROCESS_VM_READ 与查询权限。
- 启用有扫描预算的只读 ROM + GBAMemory Pointer tuple 候选发现；映射不存在、候选歧义、部分 ROM 锚点不符则拒绝附加。候选筛选逻辑仍须在不同 mGBA 版本验证。
- `apps/desktop/src-tauri/src/lib.rs`：Windows-only `list_mgba_processes`、`attach_emerald_read_only`、`read_emerald_rng`、`detach_emerald`、`get_session_snapshot`；Rust 错误直接返回给 UI。
- `apps/desktop/src/components/MgbaConnection.tsx`：运行时面板、显式选择进程、Seed 读数、断开和错误显示。浏览器 Mock 不假冒进程连接。
- 切换游戏时 UI 会请求断开旧 Emerald 会话，异常/映射变更在下一次 Seed 读取时返回 Stale 并清理连接；不能保证尚未读取时 UI 状态立即刷新。
- Windows CI 已完成 Shell check、`get_catalog` Rust 命令单测和调试 EXE 构建；**真实 WebView2 → IPC → ROM 端到端仍未通过实机验证**。

## Windows 本地手动验收步骤

1. 安装 Node.js 22+、pnpm、Rust stable / MSVC C++ Build Tools、WebView2；在仓库根目录执行 `pnpm install`、`pnpm desktop`。
2. 验证桌面「游戏库」列出 Gen3/4/5/6/PC 五组 Mock 游戏，且不会出现浏览器模式假连接；用窗口宽 1100、1366、1500px 对照 Chromium CI 截图。
3. 启动 mGBA，载入**自己合法取得的未修改 Emerald Rev0 ROM**；在「运行时」刷新目标进程列表并选择目标 PID，点击「只读连接」。
4. 如果成功，确认只读连接回执显示 ROM GameCode、语言、`partial_anchors_read_only`；在 Emerald → RNG / Seed 观察 Current Seed（u32）、Initial Seed（u16）、Advances（u32）。
5. ROM 非 Emerald、Rev 不符、映射歧义、进程权限不足时必须明确拒绝连接，不得猜地址。
6. 退出/重启 mGBA 或切换不同 ROM，在下一次 Seed 快照请求必须出现 Stale/连接丢失且停止沿用旧数据；切换到 Gen6/PC 工作台应解除旧 Emerald 会话。
7. 记录 Windows 系统、mGBA 版本、ROM GameCode/Revision、操作结果和错误日志。不要把 ROM、个人存档或专有游戏资源加入 GitHub。

**此流程目前是用户本机验收清单，不代表已经执行。**

## M1 后续
1. [x] Windows RuntimeProvider 只读进程发现与映射候选校验代码、Tauri IPC 和 React Seed 面板已实现；CI 编译通过，但需实际 mGBA 验证算法可靠性。
2. [ ] 完整 ROM 指纹（含源期望 CRC 的实际计算）与更全面 ROM 锚点验证，跨核心 Reload 测试。
3. [ ] Windows 真实 mGBA 实机测试与 WebView2 IPC/窗口布局验收；不能用 Mock / 浏览器截图代替。
4. [ ] 后续优化：运行时会话定时健康探测、不在后台无请求时保留过期连接信息、结构化 tracing 日志。
