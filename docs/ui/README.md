# GameTrainer Studio V3 · UI/UX 布局与设计规范

> 设计方向：专业桌面调试控制台；不是单款 Emerald 修改器。当前为设计规范和 Mock 原型，尚未转换为 React/Tauri 生产应用。

## 全局信息架构

~~~text
AppShell
 ├─ PrimaryRail（总览 / 游戏库 / 模块中心 / Runtime / 历史 / 日记 / 设置）
 ├─ ContextSidebar（当前游戏 / 平台过滤 / 当前功能清单）
 ├─ Header（面包屑、搜索、当前 Runtime / Session 状态）
 ├─ PageOutlet
 │   ├─ GameLibrary：All / Gen3 / Gen4 / Gen5 / Gen6 / PC Native
 │   ├─ GameWorkspace：Overview / Metrics / Feature Grid
 │   ├─ FeatureDetail：Params / Preview / Apply / Result / Restore
 │   ├─ ExtensionCenter、RuntimeManager、AuditHistory、DeveloperJournal
 │   └─ Settings
 └─ SessionInspector（右侧抽屉：PID / ROM / Profile / Capability / Diagnostics）
~~~

## 布局尺寸与响应
- 桌面设计画布 1500×930，主要目标为 1440×900 / 1366×768；最低验证 1100×700。
- 一级 Rail 64–76px；二级 Sidebar 220–250px；Header 56–64px；右侧 Inspector 280–320px。
- 8px spacing scale，控件高 36–40px；页面主内容 padding 24–28px；可折叠面板与导航。
- 1366 宽时缩窄卡片列数；1100 时右侧 Inspector 改抽屉/下方内容；窄屏收起 ContextSidebar。
- 页面必须有 loading / empty / disconnected / read-only / verified / conflict / error / restore-failed 设计，而不是只有成功样式。

## 视觉 Token
| Token | 暗色值 | 用途 |
| --- | --- | --- |
| background | #0A101B | 主画布 |
| rail | #0D1422 | 侧栏 |
| surface | #151F30 | 内容卡 |
| surface-2 | #192539 | 二级卡 |
| border | #2B3850 | 线条 |
| accent | #7356DF | 选中与关键动作（品牌源紫 #5228BD） |
| info | #5994FF | 链接、辅助（品牌蓝 #376BEE） |
| text | #E8ECF6 | 主要文本 |
| muted | #98A3BA | 次级文字 |
| success | #70D4AA | 已验证/成功 |
| warning | #F1C883 | 只读/待确认 |
| danger | #F39B9E | 拒绝/失败 |

配色：7 分极简、3 分科技。不要满屏霓虹、玻璃拟态和无意义渐变。交互成功与失败除颜色外还须有文案和图标。字体 Segoe UI/Inter/中文系统回退，PID/Seed/十六进制使用等宽字体。

## 页面行为
1. 游戏库：搜索、Gen3~Gen6、PC Native 分类、收藏、最近、模板；卡片必须有实现状态而非伪造已支持。
2. 游戏工作台：选择游戏后由 FeatureDescriptor 动态生成导航；不能 hardcode Pokémon 特殊规则到全局导航。
3. Session Inspector：显示 Runtime、目标身份、游戏 build/region、Profile 指纹、只读/可写、异常及最近操作。
4. Feature：读取、参数 SchemaForm、风险提醒、Rust Preview、用户确认、结果与恢复；断线或版本未知时不可操作。
5. 模块中心：安装来源、版本、依赖、能力、冲突状态；不自动运行未知 DLL。
6. 日志与日记：审计历史供查证；日记供编辑和导出，不能把模拟 Demo 操作算成真实审计。
7. 设置：深色/浅色、密度、语言、Runtime 路径、日志、导出、备份、快捷键。

## 前端组件拆分
AppShell / PrimaryRail / ContextSidebar / GlobalSearch / GameLibrary / PlatformFilter / GameCard / GameWorkspace / FeatureGrid / FeatureDetail / SchemaForm / MutationPreview / SessionInspector / RuntimeManager / ExtensionCenter / AuditTimeline / JournalEditor / Preferences。

UI 状态用 Zustand；查询和后端快照用 TanStack Query；连续监控指标用带 session epoch 的 Tauri event subscriptions。UI 组件不得持有 Windows 进程句柄。

见 [[components]] 组件细则以及 [[../architecture]]、[[../module-contract]]。
