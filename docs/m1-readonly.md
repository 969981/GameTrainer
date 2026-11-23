# M1 · Emerald/mGBA 只读探测（第 1 迭代）

日期：2026-11-23
状态：**源码与 Mock/Unit Tests**；Windows+mGBA 实机接入仍待验证。

## 来源和许可证
依据用户提供的 \`Pokemon_Emerald_v5.3.0_Full_Source(1).zip\` 中 \`src/main.lua\` 的参数和数据布局重新组织，只保留原版 ROM 的指纹/地址等事实。该上游 Lua 源码明确采用 GPL-3.0，任何复制源码/改写代码再发行必须审查许可证。当前代码没有将原 Lua 文件或 ROM/存档纳入仓库，工程许可证尚未定稿。

## 已编码的只读功能
- 独立 \`crates/trainer-mgba\` crate，不依赖 Tauri，不提供写内存 API。
- GBA ROM 头：游戏标题前缀、4 字符 code、Rev0、0xB2 固定值、GBA Header checksum。
- 六语言 Profile：BPEJ/JPN、BPEE/ENG、BPED/DEU、BPEF/FRA、BPEI/ITA、BPES/ESP；按 JPN/INTL 分配 Seed、Initial Seed、Advances Guest 地址。
- **部分**原版 ROM 锚点：wild、fishing、SetWarp、DoWarp、Sweet Scent land/water。校验失败则拒绝建会话；没有全量 15 项 ROM entry 校验或全 ROM CRC，不得称为完整官方验证。
- \`GuestMemoryReader\`：底层运行时必须提供只读 Guest 内存读取及独立重新获取的进程/内存映射指纹。
- \`EmeraldReadOnlySession\`：绑定 pid、进程创建时间、ROM/EWRAM/IWRAM host base、pointer slot、game code、rev；变化立即 Stale 并永久失效，禁止复用旧会话。
- Seed Snapshot：按 32 位 little-endian 读取 Current Seed 和 Advances，按 16 位读取 Initial Seed；读前/读后双重身份校验，防止跨 ROM Reload 的混杂结果。
- 仅返回 \`ProbeStrength::PartialAnchorsReadOnly\`，不会生成写入授权令牌。

## 旧代码数据参考（Source Evidence）
- \`LAYOUTS.JPN\`: seed=0x03005AE0、initial=0x02020000、advances=0x02024664。
- \`LAYOUTS.INTL\`: seed=0x03005D80、initial=0x02020000、advances=0x020249C0。
- Header：0xA0 起标题，0xAC 游戏 code，0xBC revision。
- 六个 ROM Profile 的 crc32 字符串仅作为来源期望值记录，**M1 未计算完整 ROM CRC**。
- 旧版 CE 的 GBAMemory 指针槽和候选评分逻辑依赖具体 mGBA 宿主映射；迁移时必须独立建立多候选评分、歧义拒绝和进程创建时间校验，不能简单复用 PID 或固定地址。

## M1 后续
1. Windows RuntimeProvider 只读进程发现与内存映射（权限最小化、无 PROCESS_VM_WRITE / 无注入）。
2. 校验 Guest → Host Mapping（Rom/EWRAM/IWRAM pointer group），多候选冲突拒绝。
3. 连接 Rust Provider 到 Tauri Commands，UI 展示 Detached/Discovering/ReadOnly、Seed Snapshot。
4. 完整 ROM 指纹与 15 锚点验证、跨核心 Reload 测试。
5. Windows 真实 mGBA 实机测试；不能用 Mock 通关代替。
