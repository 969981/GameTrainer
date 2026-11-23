# 模块契约与扩展 SDK（设计草案）

## 模块类型
| Kind | 责任 | 不允许 |
| --- | --- | --- |
| runtime | target discovery, attach, read, capability negotiation | 硬编码某款游戏数据 |
| game | identity/build detection, profile, codec, symbols | 绕过会话校验 |
| feature | read/preview/intent, business parameters, UI descriptor | 直接写任意 host address |
| tool | 只读解析、导入/导出、离线存档工具 | 未经允许覆盖源文件 |
| ui | 受控组件及可声明的 schema | 执行未授权 Tauri 命令 |

## 标识与版本策略
- 全局稳定 ID：game_id=games.pokemon.emerald；feature_id=features.gen3.rng；runtime_id=runtimes.mgba.external。
- 使用 semver 记录 module_version；使用独立 api_version 描述协议；Profile 还需 fingerprint、revision、source、verification_status。
- 不可因为 Manifest 写着 compatible 就相信兼容；实际运行时验证 build 与 feature required_capabilities。
- 可用状态：discovered、compatible、disabled、enabled、suspended、error；执行状态独立于 UI toggle。

## 最小能力 API
Read: process.discover、process.read、guest.read、file.read。
Mutation: guest.write.verified、patch.apply、patch.restore、file.export。
UI: ui.register、ui.event；Storage: action.history、notes.export。
只有可信 Host 在二次授权后发放临时可写能力；Runtime 不直接暴露原始任意写入给 JS。

## Rust 接口形状（伪代码，不表示已编译）

~~~rust
trait RuntimeProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn discover(&self) -> Result<Vec<TargetCandidate>, RuntimeError>;
    fn attach_read_only(&self, candidate: TargetCandidate)
        -> Result<ReadOnlySession, RuntimeError>;
}

trait GameAdapter: Send + Sync {
    fn descriptor(&self) -> GameDescriptor;
    fn probe(&self, session: &ReadOnlySession) -> Result<GameIdentity, GameError>;
    fn features(&self, identity: &GameIdentity) -> Vec<FeatureDescriptor>;
}

trait FeaturePack: Send + Sync {
    fn descriptor(&self) -> FeatureDescriptor;
    fn read(&self, context: &ReadContext) -> Result<FeatureSnapshot, FeatureError>;
    fn preview(&self, context: &PreviewContext, input: FeatureInput)
        -> Result<MutationPreview, FeatureError>;
}

// Only a trusted Host orchestrates the verified apply/restore pathway.
~~~

## UI Descriptor
一个模块可声明：id、group、icon、sort_order、capabilities、required_profiles、parameter_schema、readout_schema、risk、disclaimer、version；侧栏和详情由同一份 Schema 生成。

## 修改意图（MutationIntent）
必须绑定 session_id + epoch + game_id + build_id + profile_id + expected_before + expiration。Preview token 单次使用，所有验证在 Rust 端进行；Apply 以实际 Receipt 为结果，不接受前端自称成功。

## 验收
- Registry fixture 证明无全局游戏特例也能新增一款新游戏和一个 feature。
- 所有模块具有 no runtime、unknown build、stale session、conflict、restore failure 的明确界面和测试。
- 模块升级/卸载不会修改已保存的审计记录；旧预设按 schema version 显式迁移。
