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
