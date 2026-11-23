//! Platform-neutral, read-only M0 catalog and registry.
//! No Win32 APIs, no emulator handles, no arbitrary memory writes.
//! RuntimeProvider, GameAdapter and FeaturePack can be extended in later milestones.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const CATALOG_SCHEMA_VERSION: u32 = 1;
pub const M0_CATALOG_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/m0-registry.json"
));

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PlatformFamily {
    Gen3,
    Gen4,
    Gen5,
    Gen6,
    Pc,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImplementationStatus {
    Mock,
    Planned,
    TemplateOnly,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RuntimeDescriptor {
    pub runtime_id: String,
    pub name: String,
    pub status: ImplementationStatus,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct FeatureDescriptor {
    pub feature_id: String,
    pub name: String,
    pub group: String,
    pub description: String,
    pub icon: String,
    pub status: ImplementationStatus,
    pub required_capabilities: Vec<String>,
    pub parameter_schema: Vec<ParameterDescriptor>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ParameterDescriptor {
    pub id: String,
    pub label: String,
    pub kind: ParameterKind,
    pub placeholder: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ParameterKind {
    Text,
    Hex,
    Integer,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GameDescriptor {
    pub game_id: String,
    pub name: String,
    pub platform_family: PlatformFamily,
    pub target_platform: String,
    pub summary: String,
    pub icon: String,
    pub status: ImplementationStatus,
    pub runtime_candidates: Vec<String>,
    pub features: Vec<FeatureDescriptor>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GameCatalog {
    pub schema_version: u32,
    pub source: String,
    pub read_only: bool,
    pub runtimes: Vec<RuntimeDescriptor>,
    pub games: Vec<GameDescriptor>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Detached,
    AttachedReadOnly,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub state: SessionState,
    pub read_only: bool,
    pub session_id: Option<String>,
    pub game_id: Option<String>,
    pub runtime_id: Option<String>,
    pub profile_id: Option<String>,
    pub message: String,
}

impl Default for SessionSnapshot {
    fn default() -> Self {
        Self {
            state: SessionState::Detached,
            read_only: true,
            session_id: None,
            game_id: None,
            runtime_id: None,
            profile_id: None,
            message: "M0 仅提供 Mock 目录，未连接模拟器或游戏进程".into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("invalid catalog JSON: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("unsupported catalog schema: {0}")]
    Schema(u32),
    #[error("M0 catalog must be explicitly mock and read-only")]
    NotMock,
    #[error("duplicate or empty runtime id: {0}")]
    RuntimeId(String),
    #[error("duplicate or empty game id: {0}")]
    GameId(String),
    #[error("duplicate or empty feature id in {game_id}: {feature_id}")]
    FeatureId { game_id: String, feature_id: String },
    #[error("game {game_id} references unknown runtime {runtime_id}")]
    UnknownRuntime { game_id: String, runtime_id: String },
    #[error("game {0} has no runtime candidate")]
    MissingRuntime(String),
}

pub trait GameAdapter: Send + Sync {
    fn descriptor(&self) -> GameDescriptor;
}

#[derive(Clone, Debug)]
pub struct GameRegistry {
    catalog: GameCatalog,
}

impl GameRegistry {
    pub fn from_json(input: &str) -> Result<Self, RegistryError> {
        let catalog: GameCatalog = serde_json::from_str(input)?;
        Self::new(catalog)
    }

    pub fn new(catalog: GameCatalog) -> Result<Self, RegistryError> {
        let this = Self { catalog };
        this.validate()?;
        Ok(this)
    }

    pub fn mock() -> Result<Self, RegistryError> {
        Self::from_json(M0_CATALOG_JSON)
    }

    pub fn snapshot(&self) -> GameCatalog {
        self.catalog.clone()
    }

    pub fn game(&self, id: &str) -> Option<&GameDescriptor> {
        self.catalog.games.iter().find(|game| game.game_id == id)
    }

    pub fn list_by_platform(&self, platform: &PlatformFamily) -> Vec<&GameDescriptor> {
        self.catalog
            .games
            .iter()
            .filter(|game| &game.platform_family == platform)
            .collect()
    }

    pub fn register<A: GameAdapter>(&mut self, adapter: &A) -> Result<(), RegistryError> {
        let mut next = self.catalog.clone();
        next.games.push(adapter.descriptor());
        // Validate the complete new snapshot before modifying the registry.
        let validated = Self::new(next)?;
        self.catalog = validated.catalog;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), RegistryError> {
        let catalog = &self.catalog;
        if catalog.schema_version != CATALOG_SCHEMA_VERSION {
            return Err(RegistryError::Schema(catalog.schema_version));
        }
        if catalog.source != "mock" || !catalog.read_only {
            return Err(RegistryError::NotMock);
        }
        let mut runtime_ids = HashSet::new();
        for runtime in &catalog.runtimes {
            if runtime.runtime_id.trim().is_empty()
                || !runtime_ids.insert(runtime.runtime_id.as_str())
            {
                return Err(RegistryError::RuntimeId(runtime.runtime_id.clone()));
            }
        }
        let mut game_ids = HashSet::new();
        for game in &catalog.games {
            if game.game_id.trim().is_empty() || !game_ids.insert(game.game_id.as_str()) {
                return Err(RegistryError::GameId(game.game_id.clone()));
            }
            if game.runtime_candidates.is_empty() {
                return Err(RegistryError::MissingRuntime(game.game_id.clone()));
            }
            for runtime in &game.runtime_candidates {
                if !runtime_ids.contains(runtime.as_str()) {
                    return Err(RegistryError::UnknownRuntime {
                        game_id: game.game_id.clone(),
                        runtime_id: runtime.clone(),
                    });
                }
            }
            let mut feature_ids = HashSet::new();
            for feature in &game.features {
                if feature.feature_id.trim().is_empty()
                    || !feature_ids.insert(feature.feature_id.as_str())
                {
                    return Err(RegistryError::FeatureId {
                        game_id: game.game_id.clone(),
                        feature_id: feature.feature_id.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AddedGame(GameDescriptor);
    impl GameAdapter for AddedGame {
        fn descriptor(&self) -> GameDescriptor {
            self.0.clone()
        }
    }

    #[test]
    fn loads_five_platforms_and_three_independent_gen3_games() {
        let registry = GameRegistry::mock().expect("valid M0 fixture");
        assert_eq!(registry.snapshot().games.len(), 7);
        for platform in [
            PlatformFamily::Gen3,
            PlatformFamily::Gen4,
            PlatformFamily::Gen5,
            PlatformFamily::Gen6,
            PlatformFamily::Pc,
        ] {
            assert_eq!(
                registry.list_by_platform(&platform).len(),
                if platform == PlatformFamily::Gen3 {
                    3
                } else {
                    1
                }
            );
        }
        assert!(registry.snapshot().read_only);
        assert_eq!(SessionSnapshot::default().state, SessionState::Detached);
    }

    #[test]
    fn eighth_adapter_requires_no_registry_core_changes() {
        let mut registry = GameRegistry::mock().unwrap();
        let mut new_game = registry.game("games.pokemon.emerald").unwrap().clone();
        new_game.game_id = "games.pokemon.firered".into();
        new_game.name = "Pokémon FireRed".into();
        registry.register(&AddedGame(new_game)).unwrap();
        assert_eq!(registry.snapshot().games.len(), 8);
        assert_eq!(registry.list_by_platform(&PlatformFamily::Gen3).len(), 4);
    }

    #[test]
    fn ruby_and_sapphire_have_independent_descriptors_and_no_patch_capability() {
        let registry = GameRegistry::mock().unwrap();
        let ruby = registry.game("games.pokemon.ruby").unwrap();
        let sapphire = registry.game("games.pokemon.sapphire").unwrap();
        assert_eq!(ruby.platform_family, PlatformFamily::Gen3);
        assert_eq!(sapphire.platform_family, PlatformFamily::Gen3);
        assert_eq!(ruby.features.len(), sapphire.features.len());
        assert_ne!(ruby.game_id, sapphire.game_id);
        for game in [ruby, sapphire] {
            assert!(game
                .features
                .iter()
                .any(|feature| feature.feature_id == "features.rs.battlecrit"));
            assert!(game
                .features
                .iter()
                .any(|feature| feature.feature_id == "features.gen3.rng"));
            assert!(!game.features.iter().any(|feature| feature.status
                != ImplementationStatus::Planned
                && feature
                    .required_capabilities
                    .iter()
                    .any(|c| c == "patch.apply")));
        }
    }

    #[test]
    fn rejects_duplicate_game_id_without_mutating_catalog() {
        let mut registry = GameRegistry::mock().unwrap();
        let existing = registry.snapshot().games[0].clone();
        assert!(matches!(
            registry.register(&AddedGame(existing)),
            Err(RegistryError::GameId(_))
        ));
        assert_eq!(registry.snapshot().games.len(), 5);
    }

    #[test]
    fn rejects_unknown_runtime() {
        let mut registry = GameRegistry::mock().unwrap();
        let mut game = registry.snapshot().games[0].clone();
        game.game_id = "games.other".into();
        game.runtime_candidates = vec!["runtimes.unknown".into()];
        assert!(matches!(
            registry.register(&AddedGame(game)),
            Err(RegistryError::UnknownRuntime { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_feature_in_single_game() {
        let mut registry = GameRegistry::mock().unwrap();
        let mut game = registry.snapshot().games[0].clone();
        game.game_id = "games.other".into();
        game.features.push(game.features[0].clone());
        assert!(matches!(
            registry.register(&AddedGame(game)),
            Err(RegistryError::FeatureId { .. })
        ));
    }

    #[test]
    fn rejects_malformed_and_untrusted_catalog() {
        assert!(GameRegistry::from_json("{").is_err());
        let mut catalog = GameRegistry::mock().unwrap().snapshot();
        catalog.read_only = false;
        assert!(matches!(
            GameRegistry::new(catalog),
            Err(RegistryError::NotMock)
        ));
    }

    #[test]
    fn round_trip_snapshot_uses_snake_case_platform_fields() {
        let snapshot = GameRegistry::mock().unwrap().snapshot();
        let json = serde_json::to_string(&snapshot).unwrap();
        let parsed = GameRegistry::from_json(&json).unwrap();
        assert_eq!(snapshot, parsed.snapshot());
    }
}
