# V3 UI Wireframe · 桌面布局

这是规划示意，不是实测游戏状态。实际界面使用 React + shadcn/ui 组件。

~~~text
┌─────────┬──────────────────────┬──────────────────────────────────────────────────┬────────────────────┐
│ Rail    │ Context Sidebar      │ Workspace                                        │ Session Inspector  │
│         │                      │                                                  │                    │
│ 总览    │ 游戏库               │ GameTrainer Studio     Search / Cmd+K           │ Status: Detached   │
│ 游戏库  │ ├─ All               │                                                  │ Runtime: —         │
│ 模块    │ ├─ Gen3 (GBA)        │ [Game Hero: title / platform / build]            │ Game Build: —      │
│ 运行时  │ ├─ Gen4 (NDS)        │                                                  │ Profile: —         │
│ 历史    │ ├─ Gen5 (NDS)        │ [Metric / Metric / Metric]                       │ Capability: Read   │
│ 日记    │ ├─ Gen6 (3DS)        │                                                  │                    │
│ 设置    │ └─ PC Native         │ [Feature Card] [Feature Card]                    │ Logs / Issues      │
│         │                      │ [Feature Card] [Feature Card]                    │                    │
│         │ 当前游戏模块：       │                                                  │                    │
│         │ - Overview           │ [Action Preview / Recent History / Notes]        │                    │
│         │ - RNG / Capture ...  │                                                  │                    │
└─────────┴──────────────────────┴──────────────────────────────────────────────────┴────────────────────┘
~~~

## 游戏库动作
搜索并按平台筛选；选中游戏后，Context Sidebar 展示它独立的 FeatureDescriptors；PC 游戏不显示虚构的 Seed/RNG 功能，未实现功能有清楚状态。

## 功能操作布局
标题和权限 → 读取快照 → 参数表单 → 修改预览（session/预期原字节/作用范围/风险）→ 用户确认 → 执行结果/恢复。修改完全由可信 Rust 后端仲裁。

## 响应式
1500：显示全部；1366：缩窄卡片；1100：Inspector 为抽屉、ContextSidebar 可收起；小窗口下不允许溢出的横向控制台区遮挡重要错误信息。

## 辅助
Ctrl+K 搜索；深浅模式；紧凑模式；键盘访问；错误文案；活动历史；Obsidian 日记导出。当前 Demo 只是可视化交互。
