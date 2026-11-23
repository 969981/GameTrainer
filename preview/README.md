# V3 浏览器交互原型

- 文件：GameTrainer-Studio-V3.html。
- 来源：2026-11-23 的多游戏 V3 UI 设计原型。
- 用法：下载 HTML 后在浏览器本地打开，单文件无需联网、Node 或 Tauri。
- 示例：Gen3/4/5/6/PC Native 游戏库、搜索/筛选、动态功能菜单、工作台、模块中心、Runtime、历史、日记、主题及自定义游戏目录模板。
- 浏览器数据：Mock，使用浏览器本地数据；file:// 环境可能限制持久化。
- **不是修改器：**不会连接进程、注入 DLL、执行 Hook 或修改游戏内存。模拟操作不等于真实审计历史。
- 生产化请参考 docs/ui/README.md、docs/ui/components.md 和 docs/module-contract.md，拆分为 React 组件并接入 Rust/Tauri typed DTO。
