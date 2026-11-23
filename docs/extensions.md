# 多游戏扩展规划 · Pokémon Gen3–Gen6 & Windows Native

状态：规划，尚未提供任何已验证的游戏适配器。

## 分类表

| 平台 | 目标举例 | 候选 Runtime | 特定关注点 |
| --- | --- | --- | --- |
| Gen3 / GBA | Emerald、FireRed、LeafGreen | mGBA External / mGBA Lua Bridge | ARM/Thumb、GBA Guest Memory、Gen3 加密/存档 |
| Gen4 / NDS | Diamond/Pearl、Platinum、HGSS | NDS emulator provider（待选择） | NDS 双 CPU、Gen4 游戏差异和版本 |
| Gen5 / NDS | Black/White、Black2/White2 | NDS emulator provider（待选择） | Gen5 内存布局、种子与不同 ROM 配置 |
| Gen6 / 3DS | X/Y、ORAS | 3DS emulator provider（待选择） | 3DS 虚拟内存、版本/地区/更新补丁 |
| Windows Native | 自建单机测试程序、未来其它 PC 游戏 | Windows External / Native Agent（可选） | EXE 哈希、模块基址、x86/x64、版本与引擎 |

注：此表是规划候选，并非声称所有运行时都已支持。仅凭游戏世代不可启用任何地址或补丁。

## 不可混淆的关系

~~~text
PlatformFamily 1..N Games
Game          1..N BuildProfiles
Game          N..M FeaturePacks (通过 Capability / compat 约束)
Runtime       N..M Games (不同游戏可能复用一个 Runtime)
Session       = one Runtime + one detected Game + one verified BuildProfile
~~~

## 共用什么，不共用什么？
- 可以共用：宝可梦物种/道具/术语、只读指标卡、表单引擎、错误状态、RNG 通用数学工具（需注明算法）、审计。
- 不可盲目共用：存档加密、PID/IV 结构、内存地址、游戏函数指针、ROM Patch、版本签名、进程指针路径。
- PC Native 无需依赖 Pokémon 领域模块；可直接支持 Watch、Patch、Event、Offline Tool 等功能。

## 扩展流程
1. 创建 GameAdapter Manifest + 独立 game_id。
2. 添加 RuntimeProvider（若已存在可复用），验证进程/ROM 身份。
3. 至少一个只读 Fixture，并将未知/不兼容版本正确禁用。
4. 提供 Profile 的来源、证据和精确版本；无证据时仅允许 read-only。
5. 注册 FeaturePack、UI Schema 与必要的复杂受控组件。
6. 编写新旧游戏同时加载的回归测试；确保切换后 session、预览令牌和订阅会失效。
7. 通过资源更新、异常断线与冲突测试，再开放带写入的功能。

## 优先路径
- M0 同时注册 Gen3/4/5/6/PC 五个 Mock，证明导航、过滤、模块与详情自动生成。
- M1 Emerald/mGBA 只读连接；M2 安全 Seed 编辑及审计；M3 扩展其他 Emerald 功能。
- M4 先选第二个 GBA 游戏验证同 Runtime 多游戏，再用 NDS/3DS 验证不同地址空间，最后用自建 Windows 单机测试目标验证 Native。
- 不把 MinHook 作为所有平台的必需依赖；其适用范围是 Windows x86/x64 原生函数。
