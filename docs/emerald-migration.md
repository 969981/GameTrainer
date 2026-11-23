# Pokémon Emerald v5.3.0 · 源码迁移基线

研究来源（本地上传资料，未随仓库分发）：Pokemon_Emerald_v5.3.0_Full_Source(1).zip；Pokemon_Emerald_AllInOne_CE_v5.3.0_PokeFinderEvidenceNavigator(1).CT。

## 已观察到的旧版特性
- CE Table 以 LuaScript 为核心；不是普通 CheatEntry 地址表。
- 旧逻辑针对 mGBA 和 Emerald Rev0 的多语言 ROM Profile，涉及主机与 GBA Guest Memory 映射、RNG/Seed、捕捉、穿墙、传送、钓鱼、培育屋等功能。
- 包含补丁原始字节校验、所有权标识 PKHK 和恢复策略。移植时不能省略这些约束。

## 正确拆分
- trainer-runtime-mgba：建立只读 Session、模拟器 Guest Memory 访问与失效探测。
- game-pokemon-emerald：ROM 版本与语言指纹、游戏地址与结构；不包含 UI。
- features/gen3-rng：只读 Seed / RNG 数学 + Fixture；
- features/gen3-capture、gen3-warp、gen3-breeding 等在 M2/M3 根据证据拆分；
- PatchManager：统一受控修改和恢复，不允许每个功能随意写地址。

## 危险假设
不能把 GBA ARM/Thumb Patch 误认为 Windows MinHook；不能从一个语言或修订版本的 Patch 地址推断另一个版本；不能把 CE 静态/模拟测试描述为 Rust 版本已实机通过。未知 ROM、已经被其他工具 Patch 的区域和不稳定落点必须拒绝写入。

## 许可
旧项目的 GPL-3.0 来源背景需要在复用/移植时完整核查；暂不在此仓库存储旧源码或 ROM、存档、专有游戏资源。

## 首个可交付功能
Emerald/mGBA 启动 → 识别目标 → ROM Profile → 显示只读 Seed → 断线/换 ROM 正确失效。之后才增加带预览和恢复的写入。
