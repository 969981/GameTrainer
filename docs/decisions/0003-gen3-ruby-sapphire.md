# ADR-0003 · Ruby/Sapphire 两游戏独立 Profile、共用只读 Runtime 和 M2 Fixture Engine

日期：2026-11-23　状态：Accepted

**动机：** Emerald 的 `BPE*` 地址与 Ruby/Sapphire 的 `AXV*/AXP*` 不兼容；即使共用 mGBA 也不能复用 ROM 签名、Seed 或补丁。上传 v4.6.0 源码为 GPL-3.0，BattleCrit 只有有限静态证据。

**决定：** 两个独立 game_id、Rust `trainer-rs` 共用只读 GuestMemoryReader、Windows mGBA 只读 Provider 根据精确游戏身份选择 Profile；Header/Anchor 只可用于观察、不发放写入能力。M2 PatchManager 扩展到三个互斥的、进程内合成 Seed Fixture，不能接真实 mGBA。SQLite 记录每个 game_id。仅移植事实常量与独立代码，不复制原 Lua/Thumb payload。

**后果：** 五种平台架构保持独立且可扩展。实际 Windows/mGBA + ROM、CRC/SHA1 和完整原机器码检查仍是后续实机验收条件。参见 [Gen3 RS 集成](../gen3-ruby-sapphire.md)。
