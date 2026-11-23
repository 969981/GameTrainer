# M2 · 安全补丁事务与 SQLite 审计

日期：2026-11-23
状态：首轮 Fixture-only Rust/Tauri/React 实现。真实 mGBA 与其他游戏进程依旧只读；完整 ROM 身份和 Windows 实机尚未通过验收。

## 实现架构

| 组件 | 位置 | 功能 |
| --- | --- | --- |
| PatchManager | [trainer-patch](../crates/trainer-patch/src/lib.rs) | 预览、令牌 TTL、会话与期望字节校验、受控应用与恢复 |
| AuditStore | [trainer-storage](../crates/trainer-storage/src/lib.rs) | SQLite WAL + FULL 同步、追加式事件及未完成 intent 检测 |
| Desktop IPC | [m2.rs](../apps/desktop/src-tauri/src/m2.rs) | 隔离内存 Fixture，受控 Tauri commands；拒绝任意地址写入 |
| React | [PatchLab.tsx](../apps/desktop/src/components/PatchLab.tsx) | Seed 预览、勾选确认、应用、恢复、SQLite 历史分区 |

## 安全契约

1. Rust 在隔离 Fixture 内创建唯一 session_id，并绑定 epoch、process_identity、game_id、build_id、profile_id。
2. 前端只提交业务参数 `targetSeed`（u32）。地址/长度在可信 Rust 内固定：Fixture `0x03005D80` 的四字节 Seed，不能经 JS 指定任意地址。
3. Preview 读取现值、检查重叠/身份并生成随机一次性 UUID，30 秒过期。Preview 不修改任何数据。
4. Apply 首先消费令牌，然后重新读取身份及预期原字节；SQLite 同步写入 `apply_intent` 后才允许改变 Fixture。
5. 写完再次读取并校验数据与会话。异常后仅在完整确认自己写入的值且身份不变时尝试回滚；不能强行覆盖不明数据。
6. Restore 必须持有 patch owner ID、同一会话和匹配的补丁后字节。先落盘 `restore_intent`，恢复后 read-back 验证。冲突拒绝；失败保留 `RestoreFailed` 状态供人工诊断。
7. SQLite 持久化不能保证游戏内存原子性；异常退出后只展示未闭合 intent，不自动复用旧地址、旧会话或盲目恢复。
8. 当前核心验证 `environment=fixture`，**对 live 进程一律拒绝写入**。mGBA 仍只有读权限，没有 `WriteProcessMemory`、MinHook 或 ROM Patch 命令。

## SQLite Schema（已实现）

本地数据库：`<Tauri app-local-data-dir>/m2-fixture-action-history.sqlite3`。

表：`schema_migrations(version)`、`action_events(event_id,owner_id,preview_id,session_id,game_id,feature_id,environment,action_kind,status,summary,error_code,occurred_at_ms)`。

SQLite `environment` 只能是 `fixture` / `live`；当前仅 fixture 会产生日志。日志只存白名单摘要，不存 ROM、任意内存 dump 或账号信息。真实游戏的历史页签默认空。

## 桌面手动测试

1. 根目录执行 `pnpm install`、`pnpm desktop`；进入主导航 **补丁实验室**。
2. 点击 **开始隔离测试**：后端初始化一枚合成 Seed（初值 0x12345678），与任何 mGBA/游戏进程隔离。
3. 输入 `0xF00DBABE`，生成预览，核对旧值/新值、地址、profile、30 秒过期时间。
4. 勾选明确确认并应用：验证回执 `Active` 且测试值变成 0xF00DBABE。
5. 使用 Owner ID 执行恢复：仅当前值仍属该补丁时恢复 0x12345678，回执 `Restored`。
6. 在 **操作历史 → 隔离实验**查看 SQLite 事件；**真实操作**应仍为空。
7. 重启 app，检查旧 token/owner 不会自动重放，未闭合 intent 只用于审计提醒；没有 SQLite 时拒绝 M2 事务。

## 进一步解锁真实写入的前提（本次未实现）

- 使用真实 Windows+mGBA+合法原版 Emerald Rev0 完成身份、全 ROM CRC/hash、映射稳定性、重启与 Reload 验证。
- 为每个 GameBuildProfile 明确写入白名单、最小权限、用户确认和自动恢复测试；不得让 FeaturePack/React 任意写地址。
- 先审核已有 Emerald Lua/CT 的 GPL-3.0 许可与衍生发行问题，再考虑发布游戏修改功能。

关联：[架构](architecture.md) · [扩展契约](module-contract.md) · [M1 只读](m1-readonly.md) · [[m2-safe-patch]]
