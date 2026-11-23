# AGENTS.md — GameTrainer AI 编程统一规范

> 本文适用于整个仓库及所有 AI 编程代理。当前属于产品规划阶段；除非已有实际代码与测试，不得把规划描述成实现成果。

## 开始前必须阅读
1. README.md
2. docs/architecture.md
3. docs/extensions.md
4. docs/module-contract.md
5. docs/ui/README.md
6. docs/logging-and-diary.md
7. docs/roadmap.md
8. 触及的游戏/模块下属 README 和 Profile 证据
9. M3 真正 Guest 写入还必须阅读 docs/m3-live-gdb-experimental.md 与 ADR-0005，不能把此前 M2 的限制视为已全面取消
10. 修改 Ruby/Sapphire 捕捉、钓鱼、传送、战斗或 ROM Hook 时必须先阅读 docs/gen3-rs-migration-security-review.md 和 docs/decisions/0004-rs-migration-write-gates.md

## 技术边界
- Windows 优先；Rust Cargo Workspace + Tauri 2 + React/TypeScript/Vite + Tailwind CSS + shadcn/ui。
- 平台核心只依赖稳定接口：RuntimeProvider、GameAdapter、FeaturePack、UI Descriptor、PatchManager、Storage。
- Pokémon Gen3/Gen4/Gen5/Gen6 与 PC Native 并列支持。世代只是游戏库分类，不代表可以复用内存偏移。
- 前端由 Registry/FeatureDescriptor 动态生成侧栏与表单，禁止在全局导航按游戏名堆叠条件分支。
- GameAdapter 不能直接控制 React/Tauri Window；Feature 不直接持有 Windows HANDLE；UI 不能执行任意地址写入。
- 未实现的模拟器和游戏模块必须标为 Mock/Planned；不要声明已经完成连接、写入或运行验证。

## 安全会话及修改规则（强制）
1. 默认只读，未知游戏版本或 Profile 无法验证时禁止写入。
2. 修改流程：明确操作对象 → 后端生成预览 → 用户确认 → 后端再次校验 session id、epoch、process identity、game build、profile、预期原字节及冲突 → apply → 回执及审计。
3. 进程退出、PID 重用、ROM 替换、指针失效、运行时切换均须废弃旧会话和预览令牌。
4. 补丁与 Hook 应有可验证的所有权、生命周期和错误状态：Inactive / Pending / Active / Conflict / RestoreFailed；不能把恢复失败伪装成关闭成功。
5. MinHook 仅用于受控的 Windows x86/x64 原生场景；GBA ARM/Thumb ROM Patch 不等于 MinHook。
6. 禁止实现绕过反作弊、隐藏注入、未授权多玩家作弊、隐蔽提权或规避平台防护的行为。

## Ruby/Sapphire v4.6.0 源证据与安全约束
- Ruby / Sapphire 独立 GameCode/Revision/ROM Anchor/Seed Layout，不得复用 Emerald Guest 地址。
- 26 版本源表通过 Rust 头/16 字节锚点检查只是只读候选证明，不代表整 ROM SHA1/CRC、14 处功能入口或真实 mGBA 验证。
- 上传原源码与 Thumb payload 有 GPL-3.0；不得把原 CT/Lua、ROM、存档/payload 直接加入本项目。
- BattleCrit 是实验性研究，源报告仅四 ROM 静态/合成证据、未实机通过，禁止启用 Hook。
- M2 Ruby/Sapphire 仅可对本机 Rust Fixture 进行事务性 Seed 模拟，跨游戏旧令牌必须拒绝。

## Ruby/Sapphire 多资源功能独立门禁（ADR-0004）
- M3 已依据 ADR-0005 **仅对 Capture 与四 ROM BattleCrit** 新增独立 GDB 实验路径及窄白名单，相关 Manifest 两项允许 experimental live；Fishing/Teleport 等仍被拒绝。不可扩大至任意 Guest 地址或未知 ROM。
- 捕捉：两处 16-bit 写入必须设计多资源原子化风险控制；当前只能提供 ROM 指令/上下文的只读预览。
- 钓鱼：分别研究状态机辅助（保留 Random 调用次数）与 FishingWildEncounter/CheckFeebas RNG；禁止无所有权的 stale-payload 自动覆盖。
- 传送：新 API 必须在可信 Rust 后端重新验证 MapGroup/Map/Warp、地图范围、剧情/碰撞证据和实验许可；不能只依赖 UI 的 pfGuardRow。
- 战斗：原源码仅四 ROM 有静态 BattleCrit 证据，未实机验证。必中、敌方必 Miss、一击必杀为上游未实现。BattleCrit 只允许四 ROM 且用户本地 124B 原研究模板 SHA256 与原上下文双重验证后尝试实验，尚未实机验收；不能直接接受任意 Thumb payload。
- CodeCave/函数指针/Callback/Mailbox 的启停必须有 CPU quiescence、跨度独占、依赖有序撤销、SQLite 逐资源意图及故障注入测试；现有单 Seed Fixture 不具备这些能力。

## M2 Fixture-only 安全约束
- PatchManager 现在只允许隔离的 Rust FixtureTarget；不得因 UI 按钮可点击就开启 Windows 真实进程写内存能力。
- token/session/epoch/owner/写前 SQLite Intent 由 Rust 校验；React 只能发送白名单业务参数，不得指定任意地址或字节。
- 历史事件严格以 fixture/live 分区；异常退出不得盲目重放之前的 patch。
- 解除 live 写入限制前必须更新 ADR、提交完整 ROM/Profile 和实机回归证据。

## 游戏和插件规范
- 每个新游戏：唯一 game_id、platform_family、runtime candidates、独立 build profiles、fixture、功能清单、不兼容状态测试。
- 每个新功能：唯一 feature_id、SemVer、required capabilities、输入/输出 schema、read/preview/apply/restore 的可测试行为、UI loading/empty/error states。
- 版本指纹至少包含由对应 Runtime 能核实的 ROM/EXE 身份；禁止仅凭进程名称或世代号开启写权限。
- 第一阶段编译期 Rust 注册；数据驱动 Profile/Manifest 独立版本化；WASM 与原生 Agent 延后并经过安全评估。
- 与现有 GPL-3.0 Emerald Lua/CT 资料有关的移植，须记录来源并先审核许可证；语言重写不自动消除许可义务。

## UI 要求
- 遵守 docs/ui/README.md 的 V3 布局、状态语义、tokens、可用性与响应式行为。
- 保留游戏库筛选、第二导航动态模块区、Session Inspector、状态辨识、预览/确认/结果/恢复完整流程。
- Mock 数据必须有清楚的 Demo 标记；不允许伪造在线状态、真实数据或已执行修改。
- 重要操作支持键盘、明确焦点、可访问标签；以错误文字和图标辅助颜色。

## 记录与文档
- Rust tracing = 系统诊断；SQLite action history = 确实发生的操作；Markdown/Obsidian = 可编辑的人类开发日记。不得混为一谈。
- 每次任务同步更新相关文档与 tests/fixtures；影响架构必须写 docs/decisions/ ADR。
- 用户个人 Obsidian Vault 不属于本仓库；未经授权不得自动覆盖或推送其内容。本仓库只存项目开发记录和导出模板。
- 任何新文件遵守 Markdown 标准链接并可选 Obsidian [[双链]]；确保 GitHub 上也可导航。

## 提交前检查
- Rust：cargo fmt --check、cargo clippy、cargo test（仅已建立相应 workspace 时）。
- Web：pnpm lint、pnpm test、pnpm build（仅已建立相应应用时）。
- Mock Fixture：Gen3、Gen4、Gen5、Gen6、PC 五组加载且导航不改核心代码。
- 实机测试：必须写明 Windows、模拟器版本、游戏版本、结果、复现流程；未测试则明确标记。
- 输出交付包含：改动文件、主要行为、测试证据、失败/限制、下一步。

## M4 · FireRed/LeafGreen 受控 Guest 写入规则

- 必须阅读 [M4 实施说明](docs/m4-frlg-v204-live.md)、[ADR-0006](docs/decisions/0006-m4-frlg-experimental-live.md)，以及用户上传的 v2.0.4 ScanWildHookFix 源码证据。
- `crates/trainer-frlg` 与 `fixtures/frlg-v204-roms.json` 是独立游戏版本适配器，不得用红蓝宝石地址或任意 Guest/Host 写入替代。只支持 16 个零售版、必须完整 Guest ROM SHA1 匹配。
- 真实修改仅受限 Capture、Wall、Wild（用户本地原 116B SHA256 Payload），CPU 必须暂停，校验函数前缀、上下文、Mailbox/Code Cave、SQLite Intent、readback、逆序 restore。野生 Hook 不在战斗中装卸。
- M3 和 M4 GDB 会话互斥，除非当前会话确认无活跃 Owner 并安全断开。绝不可跨会话旧地址自动恢复；发生 ROM Reload 或 SaveState 旧跳板时拒绝危险写入。
- 禁止把 CI Rust Mock、Windows EXE 编译或来源 ZIP 的静态检查写成真实 mGBA ROM 的运行成功证据。真实联调和恢复失败注入需独立记录。
- 本项目不提交或分发原 GPL-3.0 Thumb Payload/游戏 ROM。商业发布前单独评估开源许可和派生范围。
