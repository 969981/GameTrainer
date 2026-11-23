# ADR-0002 · M2 先做隔离 Fixture 事务，禁止真实游戏写入

日期：2026-11-23
状态：Accepted

## 背景
M1 已具备 mGBA 部分 ROM 锚点及 Seed 只读代码，但未经过 Windows+mGBA+真实 ROM 验证，也没有完整 ROM 哈希。直接启用外部内存写入可能误改错误进程、错误内存映射或不兼容版本。

## 决策
- 将 Rust PatchManager 与 SQLite AuditStore 抽成独立 crates，不直接依赖 mGBA 或 React。
- 当前只有 Rust FixtureTarget 可写；真实 mGBA 仍处于只读，会话身份必须完整校验。
- 前端只发送固定的业务 Seed 参数；预览 UUID 必须短期一次性，必须具备 patch owner。
- 写入前持久化 apply_intent，恢复前持久化 restore_intent，读回并校验；失败不得伪装成功。
- 未知/外部改动的字节、过期 epoch、原字节不符、重叠补丁、审计写入失败一律拒绝写入。
- SQLite 只作审计证据，异常退出后不自动恢复或重放旧地址。
- fixture 和 live 的审计严格区分，后续支持新游戏的能力必须经新 ADR/Profile 验证解锁。

## 后果
同一事务引擎未来可覆盖 Gen3–Gen6 与 Windows Native，代价是此阶段没有真实游戏修改能力。Fixture-only CI 不能替代实机验证。

关联：[M2 细节](../m2-safe-patch.md) · [AGENTS](../../AGENTS.md)
