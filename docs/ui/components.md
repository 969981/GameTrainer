# UI 组件、状态与交互细则

## 页面状态模型
- Detached：未连接；只显示游戏介绍、功能计划、连接指导。
- Discovering：搜索目标进程；所有写入禁用。
- AttachedReadOnly：可检查部分资料；没有确认的 GameBuildProfile。
- VerifiedReadOnly：身份已验证，仍未授予写入能力。
- WriteEnabled：由后端提供明确的 capability；显示作用范围与风险。
- Stale/Conflict/RestoreFailed：全部写入禁用，右侧 Inspector 显示可见原因。

## 组件标准
| Component | 输入 | 必须呈现 |
| --- | --- | --- |
| GameCard | GameDescriptor | 游戏名、平台、候选 Runtime、实现状态 |
| PlatformFilter | PlatformFamily[] | All/Gen3/Gen4/Gen5/Gen6/PC |
| FeatureCard | FeatureDescriptor | 名称、功能分类、状态、权限 |
| SessionInspector | SessionSnapshot | 进程/模拟器、Build、Profile、权限、错误 |
| LiveMetric | FeatureSnapshot | 有效时间、单位、过期状态 |
| SchemaForm | ParameterSchema | 校验、默认值、禁用理由、错误提示 |
| MutationPreview | PreviewReceipt | 作用范围、before/after 摘要、session、风险、期限 |
| AuditTimeline | ActionEvent[] | 日期、游戏、模块、结果、恢复状态 |
| JournalEditor | markdown | 可编辑、导出预览、用户确认 |

## 典型场景
- Ctrl+K 全局搜索游戏/模块（只导航，绝不触发修改）。
- 切换游戏后结束或隔离旧订阅；不会把 Emerald Seed 数据显示在 Gen6 页面。
- 断线时 Feature 进入 Stale；锁定输入和预览按钮，并提供重新连接。
- Profile 未确认时 Snapshot 可作为只读证据，但不能按旧版偏移进行写入。
- 窄屏时右侧 Inspector 变为显式抽屉；不要丢失错误提示。

## 设计质量门槛
1. 颜色对比、键盘 Tab、焦点圈、图标标签、中文长文案与 125%/150% Windows 缩放。
2. Windows Tauri 测试：1100×700、1366×768、1500×930；触屏不是首要目标。
3. Mock 和 Connected 的差异必须可识别；模拟实时指标不声称来自正在运行的游戏。
4. 主题 token 单一来源；不按游戏单独引入互相冲突的 CSS。
5. UI 故障可显示 error_code 和诊断入口，但不能泄露未经脱敏的内存块。
