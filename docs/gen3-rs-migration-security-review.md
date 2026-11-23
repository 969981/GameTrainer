---
date: 2026-11-23
project: GameTrainer
type: security-review
status: review-only
tags: [GameTrainer, Gen3, Ruby, Sapphire, M2, Runtime, Safety]
---

# Ruby/Sapphire v4.6.0 · 捕捉、钓鱼、传送、实验性战斗功能迁移审查

**历史 M2 评估：当时所有 Live 写入关闭。后续已依据 [ADR-0005](decisions/0005-m3-gdb-guarded-live.md) 新增 [M3 实验 GDB 写入](m3-live-gdb-experimental.md)，仅 Capture 与四 ROM BattleCrit 有受限代码，尚未实机验收；本报告其它功能继续禁写。** 本文依据用户上传的 Ruby/Sapphire v4.6.0 ZIP 和 CT，已重新打开并核对源码与文档。它是评估报告，不是实际 ROM 修改功能的验收记录。

## 证据分层

| 证据 | 已确认内容 | 不代表 |
|---|---|---|
| 上传源码 E0 | src/main.lua、src/cheats_panel.lua、payloads/ 与 docs/ 的函数及设计 | 安全运行于 Windows/mGBA |
| 源包静态 ROM 报告 E1 | 29 镜像中 26 零售版 + 2 日版 Box 通过 14 组 ROM 静态检查，德国 Ruby Debug 拒绝 | 本次重新获取 ROM 或复算 SHA1 |
| 本次本地源码/合成测试 E2 | 专项 pytest：**47 passed、2 skipped** | 游戏真实运行通过 |
| GameTrainer 既有 CI E3 | Rust/React/Windows 编译和 M2 纯内存 Fixture 测试 | 已拥有 live ROM WriteCapability |
| mGBA 实机 E4 | **尚未执行** | — |

**完整 pytest 复核限制：**上传 ZIP 不含历史 dist/Pokemon_RubySapphire_AllInOne_CE_v4.2.3_Stability.CT 与 dist/...v4.4_WISHMKR_SaveRNG.CT。直接执行完整 tests，先在 test_built_ct_embedded_scripts_match_source 失败（此前 39 pass/1 skip）；跳过该测试继续，会在 test_lua_compiles_v44_and_ct_embeds_exact_source 因历史构建产物缺失而失败（此前 159 pass/3 skip）。不能声称整个测试集全绿。专项执行命令：

    python -m pytest -q tests/test_rs_capture_wall.py tests/test_rs_teleport.py tests/test_v42_fishing_assist.py tests/test_battle_v46.py tests/test_cleanup_semantics.py tests/test_rs_feebas_fishing.py tests/test_rs_v4_multilang_profiles.py

## 优先级和迁移结论

| 部分 | 源码证据 | 当前安全等级 | 首个可交付功能 |
|---|---|---|---|
| 100% 捕捉 | 两处 ROM halfword，26 零售版完整静态匹配的上游报告 | 高风险 · live 禁用 | 离线两指令和上下文指纹核对、只读预览 |
| 必咬钩 / 必钓上来 | Fishing5/6/8 状态机代理，3 表指针 + code cave + Mailbox | 极高风险 · live 禁用 | 只读钓鱼状态 / RNG 消耗观察 |
| Feebas/确定性钓鱼 | Fishing11 → FishingWildEncounter → CheckFeebas；Fix9 全版静态调用图报告 | 极高风险 · live 禁用 | 只读 Feebas 格与 RNG 模型 |
| 传送地点目录 | 306 条新 PokéFinder 地点项，含地图/warp/物种证据 | 低风险（仅查询） | 只读地点索引、分级、版本过滤 |
| 实际传送 | 一次性 gMain.callback2 + ROM 代码洞 + 16 字节 Mailbox | 极高风险 · live 禁用 | 仅 Preview，禁止执行 |
| 我方必会心 | BattleScript 表第 04 项重定向 + 124-byte Thumb 代理；四 ROM 静态证据 | 实验阻止 | 只读 Battle 指针/证据查看器 |
| 我方必中、敌方 Miss、一击必杀 | 原 v4.6.0 明确为未实现研究 | 不可用 | 仅研究文档 |

## A. 捕捉 — 优先研发只读校验，不能立即接入真实写入

上传包 src/main.lua 的 writeRomHalfword、installCaptureHook、removeCaptureHook（约 4364–4475 行）使用两处 16-bit 指令修改：

- 原 A = E004，目标 = E00C；原 B = D100，目标 = 46C0；
- 先验证独立 Profile、两处预期原值及 ROM 上下文，再写入；每处写入都有单点回读确认；
- 第二处失败时尝试把第一处和第二处恢复；移除时只恢复工具持有的字节，不强行覆盖第三方更改；
- 仍然不是原子更新：两处之间可能已经过游戏 CPU 执行；回滚本身可能失败。

**建议：**先实现业务类型 CapturePatchEvidence，离线 ROM SHA1/CRC32 与两地址上下文只读差异预览。未来需要多资源锁、进程/ROM 身份重验、CPU 安全暂停/帧边界、失败反序回滚以及逐点 owner 的持久化回执。现有 M2 单 u32 Fixture 无法直接作为两指令事务使用。

## B. 钓鱼 — 必须拆为状态机辅助和遭遇 RNG 两层

上传包 docs/RS_FISHING_ASSIST.md 与 src/main.lua 4012–4249 行：

- 必咬钩：**先让原版 Fishing6 执行 Random()**；仅存在钓鱼表且原版落入 NO_BITE 时调整为 GOT_BITE；旧 Legacy 跳过 Random() 方案不应移植。
- 必钓上来：Fishing5 的过早 A 临时屏蔽；Fishing8 的收杆窗口临时注入 A；函数返回后恢复原 gMain.newKeys，不可当作全局按键开关。
- 状态函数表有 16 项；原版新代理替换其中 4/5/7 三项，并分配 code cave + Mailbox。发生部分写入、CPU 仍在洞内或其他工具修改时无法简单还原。
- 原版 installFishingAssistHook 在 payload_stale 情况下尝试“自愈”重写代码洞。迁移后应按 Conflict 处理：没有重新证明所有权和暂停执行，**不可对未知修改自愈覆盖**。

Feebas 关键链：FishingWildEncounter → CheckFeebas（含主 RNG 50% 分支）→ GenerateFishingWildMon；在后一个入口才写 Seed 会错过笨笨鱼判定。Feebas 六钓点又由独立状态决定，不能简化为一个 Seed Hook。源包 Fix9 报告 26 零售版 + 2 Box 的完整 fishing 链静态匹配，German Debug 仍拒绝；Fix8 记录 AXPJ Rev0 调试发现 SaveState 遗留旧版跳板，但**不是所有版本均已实机验证**。

**建议：**先迁移只读 FishingStateSnapshot、钓竿/地图表、RNG 推进解释与 Route 119 Feebas 算法；在纯模拟环境验证随机次数、按键恢复、回调顺序、旧 Hook 冲突后再讨论真实 Runtime。

## C. 传送 — 目录可先交付，实际执行是不可逆世界状态修改

上传包 docs/RELEASE_NOTES.md、docs/POKEFINDER_GEN3_TELEPORT_V1.md 报告 306 条新地点分级：
**87** 条 Warp 存在性已核对；**13** 条旧坐标仅范围合法；**204** 条实验/证据缺失；**2** 条游走型禁止传送。存在 Warp 不代表该 Tile 可站立/冲浪/钓鱼，也不代表剧情事件可以触发。

src/main.lua 的 validateTeleportSymbols / teleportRaw / removeTeleportPayload（约 4659–4999 行）：

- 核验 SetWarpDestination、WarpIntoMap、CB2_LoadMap 的每版入口签名与 gMain 布局；
- 写一次性 ROM code cave、16 字节 Mailbox，再暂时替换 gMain.callback2；callback2=0 是可能的合法原版状态；
- DONE 后等待约 50ms 才清理；5s 超时恢复只是时间策略，**不能证明 Guest CPU 已离开 cave**；
- pfGuardRow 对 Warp 个数、地图边界、实验许可做校验，但存在于 UI 目录调用路径；底层 teleportRaw 本身只验证数值范围及运行时符号。因此 **Rust 后端不得信任 UI Guard**：直接 IPC 输入也必须重新校验目的地。

**建议：**第一期实现纯只读 DestinationEvidence 和地图列表浏览；依据 game_id 区分 Ruby/Sapphire 遭遇物种与剧情地点；游走/不明地点不可变为执行动作。将来实际传送必须明确备份、事件/剧情状态、碰撞与 CPU callback 生命周期。即使 ROM 字节恢复，也不等于游戏地图/剧情状态已回滚。

## D. 战斗 — 实验 Hook 不进正式开关

src/cheats_panel.lua 198–350 行及 docs/RS_CHEATS_V46_BATTLE_RESEARCH.md：

- 我方必会心：gBattleScriptingCommandsTable[0x04] 指针重定向到 **124 字节** Thumb-1 code cave；先正常执行原版 atk04_critcalc，再根据玩家侧/战斗状态修正 gCritMultiplier；
- 静态证据只覆盖 **Sapphire EN Rev1、Sapphire EN Rev2、Ruby JP Rev0、Sapphire JP Rev0**。原包明确没有 Windows CE+mGBA 真实战斗验收；
- 例外需要保留 attacker=0/2、拒绝敌方 1/3、Link、首战/Wally 教学、STATUS3_CANT_SCORE_A_CRIT、Battle Armor/Shell Armor、免疫/守住、双打、脚本指针正常推进；
- 必中、敌方必 Miss、真正一击必杀在原包尚未实现。atk01 的失败脚本跳转与 atk05→atk0C 的伤害生命周期不可被简单的 bx lr、清 MISS 或 65535 替代。

**建议：**先建立 BattleScriptEvidence 只读分析器、四 ROM 白名单和负面战斗测试矩阵。新事务至少要在安装前证明 ROM/cave/table 三处所有权；解除入口指针后还需证明 CPU 已不执行 cave，才允许清除 Payload。恢复洞失败必须保留 RestoreFailed 状态和审计，不能自动“修好”。

## GameTrainer 当前架构缺口

当前 trainer-rs 只做 Header/Revision + 16-byte ROM Anchor 和 Seed/Timer 只读。未独立验证上游 14 个 Function Anchors 或整 ROM SHA1；M2 PatchManager 只有一枚合成 Seed 的事务状态机，禁止真实写入。

要支持多地址功能必须另设计 MultiResourcePatchManager（**设计，不是已实现**）：

1. 后端固定 FeatureIntent → 只读全 ROM 哈希 + 逐功能上下文签名 → 同一 Session/Epoch/进程身份；
2. 对两指令、函数表、code cave、Mailbox、callback 等资源建立独占区间和依赖图；预览阶段显示所有 before/after 证据；
3. CPU Quiescence：只在可证明安全的 Guest 帧/暂停点修改，不能单靠定时等待 50ms；
4. SQLite 在任何修改前记录事务意图/资源清单，先 staged cave 和校验，**最后**激活跳转/指针；
5. 停用先解绑入口/指针，确认 CPU 已离开，再根据相同 owner 的当前字节恢复 cave/mailbox；任何冲突保留诊断，不写未知地址；
6. ROM Reload、SaveState、PID 重用、跨语言/Revision、代码洞被其它工具占用、恢复失败时 fail-closed。不跨重启自动重放旧 Host 地址。

**必须禁止的捷径：**解除 trainer-patch 的 fixture 校验、让 React 传入任意 Guest 地址、对未知 ROM 用 AOB 结果猜指令、在 live 进程中执行原 Lua Thumb payload、把“静态匹配/模拟成功”写成“实机成功”。

## 实施任务与验收

| 阶段 | 下一项具体交付 | 退出条件 |
|---|---|---|
| P0 | 本评估、机器可读门禁 Manifest、禁写测试 | 所有功能 live_write_enabled=false，CI 保留 no-live 约束 |
| P1 | Capture 原字节只读证据；Fishing RNG/状态模拟；306 条传送目录纯查询 | 26 版源数据/独立版本检查，未知 ROM 拒绝 |
| P2 | 多资源纯 Fixture 事务和故障注入 | 半字双点、三表项、cave/mailbox、Callback、跨游戏与 SaveState 冲突 |
| P3 | 仅离线单机沙盒逐步评估 Capture/Fishing Hook | 整 ROM SHA1、写前暂停、实机回读、备份、恢复、许可审核 |
| P4 | 实际传送与 Feebas Hook 的独立审核 | 事件/碰撞/剧情与 RNG 顺序的真实游戏证据 |
| P5 | 战斗 Crit 保持研究，单独 ADR 决定是否继续 | 四 ROM 真正战斗测试、单/双打/异常脚本/崩溃回滚全覆盖 |

通用负面矩阵：非原版/汉化 ROM、德国 Debug、Revision 错、ROM Reload、SaveState 旧 Hook、PID 重用、Host mapping 变化、超时/一次性 Token、两点写入中断、table 部分更新、回滚失败、第三方占用代码洞、CPU 仍执行 cave、战斗中安装、地图/剧情边界、联机状态、SQLite 不可写。

## 来源与交叉验证

- 用户上传 ZIP 的 docs/ROM_VERIFICATION_FULL_MATRIX.md、docs/RS_FISHING_ASSIST.md、docs/RS_FISHING_FIX9_FULL_VERSION_MATRIX.md、docs/RS_FEEBAS_FIX8.md、docs/POKEFINDER_GEN3_TELEPORT_V1.md、docs/RS_CHEATS_V46_BATTLE_RESEARCH.md。
- 对照公开反编译：[pret/pokeruby battle commands](https://github.com/pret/pokeruby/blob/master/src/battle_script_commands.c) 和 [event warp macros](https://github.com/pret/pokeruby/blob/master/include/macros/event.inc)。反编译源码不能替代所选 ROM 的独立二进制指纹。

关联：[Ruby/Sapphire](gen3-ruby-sapphire.md) · [M2](m2-safe-patch.md) · [测试](testing.md) · [ADR-0004](decisions/0004-rs-migration-write-gates.md) · [[gen3-rs-migration-security-review]]
