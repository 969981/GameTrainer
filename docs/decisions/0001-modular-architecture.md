# ADR-0001：分离 Runtime、Game、Feature 与 UI

日期：2026-11-23
状态：Accepted for planning（未实现）

## 背景
最初从 Pokémon Emerald v5.3.0 的 CE Lua/CT 迁移出发，但后续明确需要 Gen3、Gen4、Gen5、Gen6 和 Windows 非模拟器游戏。若以 Emerald 功能作为应用骨架，增加新运行时就必须重写 UI/进程层。

## 决定
- 采用单一 Tauri 2 主程序，独立 Rust 平台核心。
- 使用 PlatformFamily 进行目录筛选；GameAdapter 识别具体游戏及 build；RuntimeProvider 执行通信；FeaturePack 提供业务功能；UI Descriptor 生成导航和交互。
- 第一阶段内置编译期模块 + Mock Fixture；未来再讨论声明式 Profile、WASM、可信原生 Agent。
- 所有修改通过安全 PatchManager，默认只读；实际进程/ROM 版本须严格验证。
- 调试日志、操作历史、开发日记分离；开发日记通过用户确认导出 Obsidian。

## 后果
优点：增加游戏不改主导航；不同平台隔离；功能模块可测试；风险与日志可追溯。
成本：前期需要 Session/Registry/DTO/API 设计；复杂游戏细节必须有单独适配器；MinHook 不可通吃所有模拟器。

## 验证门槛
五个 Mock（Gen3 / Gen4 / Gen5 / Gen6 / PC）不修改 AppShell 即自动生成导航；Emerald/mGBA 只读实测通过后才实现带写入的功能。

## 关联
[[../architecture]]、[[../module-contract]]、[[../ui/README]]、[[../roadmap]]。
