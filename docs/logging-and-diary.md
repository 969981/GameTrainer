# 日志、审计、研究笔记与 Obsidian 开发日记

## 先分清数据归属
| 数据流 | 建议技术 | 内容 | 默认写入 |
| --- | --- | --- | --- |
| Diagnostic | Rust tracing + rotating file | 连接、校验、性能、错误代码 | 应用本地，可配置保留 |
| Action History | SQLite event ledger | 真实预览、执行、拒绝、恢复与失败的可审计事件 | 仅由可信 Rust Host |
| Game Research Note | Markdown / 本地笔记 | 游戏研究、结构分析、实验观察 | 用户显式保存 |
| Developer Diary | Markdown / Obsidian | 本日提交、研究结论、架构决策、待办 | 预览后用户确认导出 |

操作历史和诊断日志不能自动写入个人日记。UI Demo 交互不等于真实游戏修改；演示数据不得记入 Action History。

## SQLite 初步 schema（设计）
~~~sql
CREATE TABLE sessions (
  session_id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL,
  runtime_id TEXT NOT NULL,
  profile_id TEXT,
  started_at TEXT NOT NULL,
  ended_at TEXT,
  end_reason TEXT
);
CREATE TABLE action_events (
  event_id TEXT PRIMARY KEY,
  session_id TEXT,
  game_id TEXT NOT NULL,
  feature_id TEXT NOT NULL,
  action_kind TEXT NOT NULL,
  status TEXT NOT NULL,
  risk_level TEXT NOT NULL,
  occurred_at TEXT NOT NULL,
  summary TEXT NOT NULL,
  before_summary_json TEXT,
  after_summary_json TEXT,
  receipt_id TEXT,
  error_code TEXT
);
CREATE TABLE saved_presets (
  preset_id TEXT PRIMARY KEY,
  game_id TEXT NOT NULL,
  feature_id TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  name TEXT NOT NULL,
  config_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE journal_exports (
  export_id TEXT PRIMARY KEY,
  period_date TEXT NOT NULL,
  destination_path TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  exported_at TEXT NOT NULL
);
CREATE INDEX idx_action_game_time ON action_events(game_id, occurred_at DESC);
~~~

保留 before/after 仅限结构化、白名单摘要，避免记录内存 dump、敏感账号信息和大批量原始字节。错误内容应截断并脱敏。恢复机制由 session 及 PatchManager 判断，不因审计库保存了原值就盲目覆盖地址。

## Obsidian 的项目目录（可配置示例）
~~~text
Work/Projects/GameTrainer/
  README.md
  Research/
  Decisions/
  Changelog/
  Daily/2026-11-23.md
~~~
个人库 Personal-Life-OS 是后续可选导出目标，不属于当前 GitHub 仓库。使用 Obsidian [[GameTrainer]]、[[2026-11-23]] 双链，也提供标准 Markdown 相对链接，保证 GitHub 可阅读。

## 导出行为
- 默认先生成可编辑的本地草稿；预览、选择目标路径、确定后才执行写文件。
- 不自动覆盖 Vault 既有文件；同日重复导出通过 export_id/content_hash 检查，发现冲突先展示 diff/选项。
- 导出成功记录元信息，但不自动 git commit/push 个人知识库。
- 如果以后允许关联 Git commit，日志只引用实际 SHA/文件；工作计划标为「计划」，不得写成「已完成」。

## 日记模板
~~~markdown
---
date: 2026-11-23
project: GameTrainer
type: dev-diary
status: planning
tags: [rust, tauri, react, game-trainer]
---
# GameTrainer · 2026-11-23
## 今日进展
- 规划多游戏 V3 桌面 UI、模块与会话架构。
## 技术结论
- Runtime、Game、Feature 分离；MinHook 不替代 GBA Patch。
## 后续
- [ ] M0 React Mock Registry
## 关联
- [[GameTrainer]] · [[2026-11-23]]
~~~
