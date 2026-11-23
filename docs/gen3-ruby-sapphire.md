# Gen3 Ruby / Sapphire v4.6.0 · 只读适配与 M2 Fixture

**日期：**2026-11-23　**状态：**Rust、Windows mGBA 只读接口、React 模块与 M2 三游戏 Fixture 已提交；真实 ROM/Windows mGBA 读数尚未实机验收。

## 依据与许可证

本次实际读取用户提供的 `Pokemon_RubySapphire_AllInOne_CE_v4.6.0_BattleCritExperimental(1).zip`、同名 `.CT`、其中 `src/main.lua`、`docs/ROM_VERIFICATION_FULL_MATRIX.md`、`docs/RS_CHEATS_V46_BATTLE_RESEARCH.md` 等资料。源包声明 **GPL-3.0**。仓库只编写独立 Rust Profile/只读探测代码，提取可核实的 GameCode/Revision/Anchor/内存布局事实，不提交原 Lua/CT/Thumb payload 或 ROM。本次没有接触或重新检验其原版 29 个 ROM 镜像。

## 零售版本矩阵（来自上游研究包）

| 语言 | Ruby Game Code | Sapphire Game Code | Revision |
| --- | --- | --- | --- |
| 英文 | AXVE | AXPE | 0 / 1 / 2 |
| 日文 | AXVJ | AXPJ | 0 / 1 |
| 德文 | AXVD | AXPD | 0 / 1 |
| 法文 | AXVF | AXPF | 0 / 1 |
| 意大利文 | AXVI | AXPI | 0 / 1 |
| 西班牙文 | AXVS | AXPS | 0 / 1 |

共 **26 个 Header Profile**（Ruby 13 + Sapphire 13）。源报告另有日版 Ruby/Sapphire Pokémon Box 两份兼容 Rev0 别名，以及被判为不支持的德国 Ruby Debug。源报告宣称 26/26 零售镜像通过 14 组 ROM 机器码静态核对；**GameTrainer 当前实现并未复现全部 14 项检查，也未计算完整 ROM CRC32/SHA1**。

## 已实现的独立只读安全校验

Rust [trainer-rs](../crates/trainer-rs/src/lib.rs) 对 0xA0 游戏标题、0xAC–0xAF GameCode、0xBC Revision、0xB2 GBA 固定值、0xBD Header checksum、Guest ROM 0x08000128 的 16 字节零售 Anchor 做白名单校验。错误 Revision、未知 ROM、调试版 Anchor 不符一律拒绝。给出的 `profile_strength=header_revision_boot_anchor_only` **不授予写入权限**。

Windows [trainer-runtime-win](../crates/trainer-runtime-win/src/windows.rs) 先按 Ruby/Sapphire ROM 标题进行候选扫描，再按 GameCode、Revision、ROM Anchor 和 Game/地区相关的 IWRAM/EWRAM 地址验证 Guest→Host 映射；候选数量不唯一就拒绝连接。会话指纹包含 PID/创建时间、ROM/EWRAM/IWRAM Host base、pointer slot、GameCode 和 revision。仍然只申请 PROCESS_VM_READ 及必要查询权限，未开放写内存。

## Seed / Timer 地区差异（来自原 `LAYOUTS`）

| Layout | Seed Guest | Timer Guest | SaveBlock1 | SaveBlock2 |
| --- | --- | --- | --- | --- |
| 英文 USA | `0x03004818` | `0x03001790` | `0x02025734` | `0x02024EA4` |
| 日本 JPN | `0x03004748` | `0x03001700` | `0x02025494` | `0x02024C04` |
| 德法意西 EUR | `0x03004828` | `0x03001790` | `0x02025734` | `0x02024EA4` |

**禁止沿用 Emerald 的 Seed 地址**。原 Lua 的 RNG Advances 为内存中的运行时计算值，当前 Rust RS 面板只显示 Current Seed 和 Timer，不伪造固定 Advances。

## GameRegistry 和 UI

- 新增 `games.pokemon.ruby`、`games.pokemon.sapphire` 两个 Gen3 GameDescriptor，各 12 个模块（研究/规划），而不是 Emerald 模块的别名。
- Rust Tauri 命令：`list_rs_profiles`、`attach_rs_read_only(process_id, game_id)`、`read_rs_rng`，与 Emerald 同时只能存在一个 Gen3 只读会话。
- React 运行时可选择“连接红宝石”“连接蓝宝石”，并显示完整 26 项 Rust Profile 表和 Seed/Timer。
- 已登记模块：RNG、野生遭遇/PokéFinder、钓鱼/笨笨鱼、地图/传送、Egg RNG、拉帝与幻之岛、队伍/盒子、Safari/Repel/Wallet、WISHMKR、捕捉、穿墙、战斗研究。**除只读核心外均为 Planned**，不能因为菜单存在就称功能可用。

## v4.6.0 BattleCrit 实验性的严格边界

源资料对“我方必会心”的来源证据仅覆盖 **Sapphire English Rev1/2、Ruby Japanese Rev0、Sapphire Japanese Rev0** 四款 ROM 的静态/合成验证，原包明确没有 Windows mGBA/CE 真实战斗验收。`必中`、`敌方必 Miss`、`真正一击必杀` 在原包尚未开放。GameTrainer 不启用实验 Hook，也不把这些列为已完成修改。

## M2 安全补丁实验室（仅合成 Fixture）

- 在“补丁实验室”选择 Emerald、Ruby、Sapphire 分别创建带不同 game_id/profile_id 的 Rust 进程内合成 Seed。Ruby/Sapphire Fixture 使用**英语布局** `0x03004818`，Emerald Fixture 使用 `0x03005D80`。
- 保留 30 秒单次 Preview、人工确认、原字节比对、SQLite apply_intent、write/read-back、Owner-scope restore 和错误状态。跨游戏 token 不可重放，审计按各自 game_id 记录。
- 即使 Rust Fixture 测试成功，**也不会写入真实 mGBA 的 Ruby/Sapphire/Emerald 内存**。未知/未验证 Profile 永远不允许写。

## 测试与待办

Rust 测试：26 个组合、错误 Title/Checksum/Revision、German Debug Anchor 变更、进程映射身份变化、跨游戏 Fixture token、独立 apply/restore、SQLite game_id。浏览器：Gen3 红蓝游戏页面、Gen6/PC 原有导航、1100/1366/1500 三种宽度。Windows CI：Tauri/Win32 编译和桌面 Rust Fixture 单测。

仍必须由用户在真实 Windows + mGBA + 合法原版 ROM 环境中逐语言验证实际 Seed/Timer 读取、重启失效、选错游戏拒绝、Debug/ROM Hack 识别，以及 M2 Fixture 不能影响真实游戏内存。GameTrainer 当前的 Header/Anchor 验证不足以为真实游戏写入发放授权；后续先做全 ROM SHA1/CRC32 + 全部功能入口签名，再考虑离线单机修改权限。

相关：[架构](architecture.md) · [M1](m1-readonly.md) · [M2](m2-safe-patch.md) · [[gen3-ruby-sapphire]]

## 2026-11-23 已执行的自动化验收（区分实机）

- [CI #45111652638](https://github.com/969981/GameTrainer/actions/runs/45111652638)：全部三个 Job 成功。Rust 格式/Clippy/44 项单元测试成功，其中 Ruby/Sapphire 8 项、三游戏 Fixture Patch 16 项、Registry 8 项。
- React ESLint、TypeScript、11 项 Vitest、Web Build、Chromium 1100/1366/1500 px 的 Ruby/Sapphire/Gen6/PC/M2 导航和页面截图均通过，共 18 张。
- Windows Tauri 2 Shell `cargo check`、桌面 Rust 命令测试、debug EXE `cargo build` 及 Artifact 上传通过：[Windows Debug EXE](https://github.com/969981/GameTrainer/actions/runs/45111652638/artifacts/14789568790)。
- 仍未执行的测试：在用户真实 Windows WebView2 上打开并连接 mGBA、使用真实 Ruby/Sapphire ROM 比对 Seed/Timer、复现 ROM Reload/错误版本拒绝与恢复。
