# 测试矩阵与证据要求

## Mock / UI（M0）
- [ ] Gen3、Gen4、Gen5、Gen6、PC 五套 Manifest 注册成功。
- [ ] 加入第六个 Mock 不改 AppShell/全局导航。
- [ ] 跨游戏切换不复用前游戏 Session、Seed、Profile、预览令牌。
- [ ] unknown / disconnected / stale / conflict / restore-failed 有可理解 UI。
- [ ] 主题/搜索/键盘/1100×700、1366×768、1500×930 截图回归。

## Runtime（M1）
- [ ] 进程不存在、权限不足、架构不匹配、进程退出、PID 重用。
- [ ] ROM/EXE build、地区、revision 不兼容、Profile 改变。
- [ ] Guest Address 与 Host Address 类型分离，测试错误映射不会写入。

## Patch（M2）
- [ ] 预览 token 有时效、一次性；会话 epoch 变化拒绝。
- [ ] 改写前原字节冲突拒绝；同时插件重叠拒绝。
- [ ] 恢复只触达有所有权且可验证的区域；恢复失败不伪装成功。
- [ ] SQLite 只存字段白名单与错误摘要；无需持久化敏感 dump。

## 实机证据模板
- OS、架构、Runtime/模拟器版本、游戏版本/哈希（合法分享范围内）。
- 测试条件、步骤、预期、观察、日志文件位置、结果和已知限制。
- Mock/静态测试/集成测试/Windows 实机需要分别标注，不能互相替代。

## 2026-11-23 已执行测试（CI，不代表实机 mGBA）
- [x] `trainer-core` 7 项 + `trainer-mgba` 9 项合成单元测试。
- [x] React/Vitest 10 项，ESLint、TypeScript 和 Web Build。
- [x] Playwright Chromium：1100/1366/1500px 3 条 E2E，共 9 张截图，游戏库/Gen6/PC 切换与横向溢出检查。
- [x] Windows `cargo check -p game-trainer-desktop`、`cargo test -p game-trainer-desktop --lib`（Rust get_catalog）和 `cargo build -p game-trainer-desktop`。
- [ ] Windows 原生 WebView2 GUI 启动及 React→Tauri IPC 实测。
- [ ] mGBA + Emerald Rev0 的真实 ROM/Seed 地址、读数、Reload/候选歧义验证。
- 证据：[CI #45107272587](https://github.com/969981/GameTrainer/actions/runs/45107272587)。

## M2 Fixture 回归
- Rust PatchManager：token 过期/重放、epoch/build 变化、字节冲突、重叠拒绝、读回、恢复所有权、失败状态与 live 禁止。
- SQLite AuditStore：先持久化 intent、模拟/真实事件隔离、重启保留不确定事件且不自动恢复。
- React 浏览器：补丁实验室不能伪造后端成功、无法在浏览器内实际应用。
- Windows Tauri/真实 mGBA + ROM 分别验收；详见 [M2 说明](m2-safe-patch.md)。

## M2 CI 验收结果（2026-11-23）
- [x] 32 项 Rust 单测：Core 7 + mGBA 9 + Patch 13 + SQLite 3。
- [x] React/TypeScript/ESLint/Vitest/Vite。
- [x] Playwright Chromium 1100、1366、1500px 游戏库/Gen6/PC/M2 页面与 12 张截图。
- [x] Windows Tauri check、Rust 桌面命令测试、调试 EXE 构建。
- [ ] Windows WebView2 GUI 人工交互验收、真实 mGBA/ROM 验证与任何 live 写入。
- [CI 实际证据](https://github.com/969981/GameTrainer/actions/runs/45109373457)。

## Ruby/Sapphire v4.6.0 回归
- Rust：26 Header/Revision、区域 Seed/Timer、德国 Debug Anchor 不符、未知 ROM/Revision、进程身份变化。
- M2：Ruby/Sapphire 独立 Fixture、原值恢复、game_id SQLite 审计、跨游戏旧 token 拒绝。
- React/Chromium：Gen3 游戏库、两款独立页面、1100/1366/1500 px 与其它世代切换。
- Windows：Tauri Win32 编译与桌面单测；真实 mGBA/ROM 实机仍须单独验收。
- [详细说明](gen3-ruby-sapphire.md)。

## Gen3 Ruby/Sapphire CI 结果（2026-11-23）
- [x] Rust 44 项（Core 8 + Emerald 9 + Ruby/Sapphire 8 + M2 Patch 16 + SQLite 3）。
- [x] React/Vitest 11 项、TypeScript、ESLint、Vite。
- [x] Chromium 1100/1366/1500px 红蓝宝石、Gen6、PC Native、M2 页面及 18 张截图。
- [x] Windows Tauri check、桌面 Rust unit tests、Windows Debug EXE。
- [ ] Windows 真实 GUI/WebView2 与 mGBA Ruby/Sapphire 真 ROM、ROM Reload 实机测试，所有真实游戏写入仍禁止。
- [证据](https://github.com/969981/GameTrainer/actions/runs/45111652638)。

## 2026-11-23 Ruby/Sapphire 四类功能源码复核

- [x] 实际解压 v4.6.0 原包，检查 CaptureHook、FishingAssist、TeleportRaw/Guard、BattleCrit 及源包证据文档。
- [x] 专项本地 pytest（7 个测试文件）：**47 passed, 2 skipped**。
- [!] 完整原包 pytest 的历史构建一致性测试缺少 dist/v4.2.3 与 dist/v4.4 等老版 CT，出现 FileNotFoundError / 缺档断言；**不认定为完整通过**，与真实模拟器功能无关。
- [x] 新增机器可读 Manifest 及 4 个 Vitest 门禁测试，均保持 live_write_enabled=false。
- [ ] 26 版整 ROM SHA1 / 捕捉两处指令差异只读复查；钓鱼状态与 RngAdvance 轨迹模拟；306 条地图目录/Ruby-Sapphire 物种分离；SaveState stale Hook 冲突。
- [ ] 多资源事务合成故障注入：2 指令、3 表指针、code cave、Mailbox、callback、恢复失败，尤其是写入中断与 CPU quiescence。
- [ ] Windows+mGBA 真实游戏行为和独立实机记录（当前不允许任何 Live 写入）。

相关：[迁移评估](gen3-rs-migration-security-review.md) · [ADR-0004](decisions/0004-rs-migration-write-gates.md)。
