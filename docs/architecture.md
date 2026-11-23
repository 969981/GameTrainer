# GameTrainer Studio — 系统架构

状态：设计规范，2026-11-23。参考：[[extensions]]、[[module-contract]]、[[ui/README]]、[[roadmap]]。

## 分层关系

~~~text
React App (AppShell / GameLibrary / Workspace / FeatureDetails / Journal)
   ↓ Tauri 2 Commands + typed Events; UI never receives raw write API
Application Services (GameRegistry / SessionManager / CapabilityPolicy)
   ├── Runtime Providers (mGBA bridge / Windows external / future emulator)
   ├── Game Adapters   (Emerald / Gen4 / Gen5 / Gen6 / Native title)
   ├── Feature Packs   (Seed RNG / Capture / Map / Watch / Save / ...)
   ├── PatchManager    (preview / verified-apply / restore)
   └── Storage         (tracing / SQLite action ledger / journal exports)
                ↓
          Target Process / Emulator / Offline Save
~~~

核心依赖不能指向具体游戏；GameAdapter 不能假设某种 Runtime 一定是 Win32 地址空间。UI 只消费类型化 DTO 和模块描述符。

## 术语（不能互换）
- PlatformFamily：游戏库筛选，如 GBA/Gen3、NDS/Gen4/Gen5、3DS/Gen6、Windows native。
- RuntimeProvider：目标发现、attach、read、address translation、capabilities、detach；代表通信渠道。
- GameAdapter：标识游戏、解析 build/region/revision、装配 Profile、解码游戏数据。
- FeaturePack：对兼容游戏的功能单元，具有数据读取、参数 schema 与可审计修改意图。
- GameBuildProfile：经过证据验证的特定版本配置，不由平台类型自动推导。
- UI Extension：通用 Slot + SchemaForm + 受控组件；禁止加载不受信任的远程 React/JS。

## 推荐 Workspace（目录为规划，未实现）

~~~text
apps/desktop/src                 React 应用
apps/desktop/src-tauri           Tauri 命令与权限
crates/trainer-core              实体、Registry、会话与能力
crates/trainer-runtime-win       Windows 外部进程 API
crates/trainer-runtime-mgba      mGBA 地址映射/桥接
crates/trainer-patch             事务预览、校验、写入、恢复
crates/trainer-storage           SQLite schema 与 tracing
crates/trainer-ui-schema         跨 Rust/TypeScript 的 DTO/schema
games/pokemon/gen3..gen6        游戏/世代特化逻辑
games/native                    PC Native 游戏适配器
features/                        可复用业务模块
profiles/                        带来源证据的版本 Profile
agents/native-win                未来可选 MinHook Agent
~~~

## Session 状态机
Detached → Discovering → AttachedReadOnly → VerifiedReadOnly → WriteEnabled（显式授权）。
任何身份、Profile、epoch 不匹配都退回 ReadOnly / Stale；Apply 失败必须返回明确原因，不允许静默成功。
写入权限不可由 UI 自行推断或保存为跨启动授权。

## 修改的完整生命周期
1. 读取 Runtime 身份 + GameAdapter 指纹 + Profile，创建不可复用 session_id 与单调 epoch。
2. Feature 请求预览，Rust 计算要改的对象、范围、风险、预期原值与过期条件。
3. 用户确认 preview_id（短期有效）。
4. Rust 重新检查身份、epoch、Profile、地址范围、预期字节与补丁冲突。
5. 通过 PatchManager 执行写入，存 ActionReceipt、错误及恢复元数据。
6. 恢复时仍须检查当前会话/原始字节/补丁所有权；无法安全恢复则标记 RestoreFailed，不强制覆盖。

## 插件发布策略
P0 内置编译期 Rust；P1 带 schema/哈希/来源的声明式数据包；P2 受限 WASM 纯逻辑模块；P3 明确来源、用户知情且受控的本机 Agent。优先避免不可信 DLL 动态加载。

## 范围与限制
首版只做离线/单机研究环境。MinHook 不等于 GBA ROM Patch。已上传的 Emerald Lua/CT 资料作为研究来源；实际代码移植需单独完成许可审查和 Windows+mGBA 验证。
