//! M2 safe patch engine. The only enabled backend is an isolated in-memory
//! fixture: actual mGBA remains read-only until full ROM/real-runtime validation.
//! No Tauri command can supply an arbitrary guest/host address or raw byte array.

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use thiserror::Error;
use trainer_storage::{ActionEvent, AuditError, AuditStore, EventContext};
use uuid::Uuid;

pub const FIXTURE_SEED_ADDRESS: u32 = 0x0300_5D80;
pub const FEATURE_ID: &str = "features.gen3.rng";
pub const GAME_ID: &str = "games.pokemon.emerald";
pub const FIXTURE_PROFILE_ID: &str = "fixture-only-emerald-seed-v1";

#[derive(Debug, Error)]
pub enum PatchError {
    #[error("SQLite audit failed: {0}")]
    Audit(#[from] AuditError),
    #[error("actual game writes are disabled; M2 allows isolated fixture only")]
    WriteLocked,
    #[error("the verified session/epoch/profile/process identity changed")]
    StaleSession,
    #[error("preview token does not exist or was already consumed")]
    UnknownPreview,
    #[error("preview token expired")]
    PreviewExpired,
    #[error("overlapping or duplicate patch exists")]
    Overlap,
    #[error("expected bytes changed or patch ownership is uncertain")]
    Conflict,
    #[error("patch owner not found")]
    UnknownOwner,
    #[error("restoration failed; ownership preserved for diagnosis")]
    RestoreFailed,
    #[error("unsafe patch length or address")]
    InvalidSpan,
    #[error("target memory read or write failed: {0}")]
    Target(String),
    #[error("post-write bytes were not confirmed")]
    VerifyFailed,
    #[error("patch would not change any value")]
    Noop,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetIdentity {
    pub session_id: String,
    pub epoch: u64,
    pub process_identity: String,
    pub runtime_id: String,
    pub game_id: String,
    pub build_id: String,
    pub profile_id: String,
    /// "fixture" or "live" — the M2 engine rejects live unconditionally.
    pub environment: String,
}
impl TargetIdentity {
    fn validate_fixture(&self) -> Result<(), PatchError> {
        if self.environment != "fixture"
            || self.profile_id != FIXTURE_PROFILE_ID
            || self.game_id != GAME_ID
            || self.runtime_id != "runtime.fixture.memory"
            || self.process_identity != "in-memory-only"
        {
            return Err(PatchError::WriteLocked);
        }
        Ok(())
    }
}

/// No runtime-specific process handles, no primitive exposed to React.
pub trait PatchTarget {
    fn identity(&self) -> Result<TargetIdentity, PatchError>;
    fn read(&self, address: u32, len: usize) -> Result<Vec<u8>, PatchError>;
    fn write(&mut self, address: u32, bytes: &[u8]) -> Result<(), PatchError>;
}

#[derive(Clone, Debug)]
pub struct PatchSpec {
    feature_id: &'static str,
    address: u32,
    desired: Vec<u8>,
}
impl PatchSpec {
    /// This private API construction is trusted server-side, not a JS address.
    pub fn emerald_fixture_seed(seed: u32) -> Self {
        Self {
            feature_id: FEATURE_ID,
            address: FIXTURE_SEED_ADDRESS,
            desired: seed.to_le_bytes().to_vec(),
        }
    }
    fn validate(&self) -> Result<(), PatchError> {
        if self.feature_id != FEATURE_ID
            || self.address != FIXTURE_SEED_ADDRESS
            || self.desired.len() != 4
        {
            return Err(PatchError::InvalidSpan);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchStatus {
    Pending,
    Active,
    Conflict,
    RestoreFailed,
    Restored,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatchPreview {
    pub preview_id: String,
    pub session_id: String,
    pub epoch: u64,
    pub game_id: String,
    pub feature_id: String,
    pub profile_id: String,
    pub address_display: String,
    pub before_display: String,
    pub after_display: String,
    pub expires_in_seconds: u64,
    pub risk: String,
    pub environment: String,
    pub status: PatchStatus,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatchReceipt {
    pub owner_id: String,
    pub session_id: String,
    pub feature_id: String,
    pub status: PatchStatus,
    pub environment: String,
    pub audited: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FixtureSnapshot {
    pub session_id: String,
    pub epoch: u64,
    pub seed: u32,
    pub active_patches: Vec<PatchReceipt>,
    pub read_only_game_runtime: bool,
    pub environment: String,
}

struct Pending {
    preview_id: String,
    identity: TargetIdentity,
    spec: PatchSpec,
    expected: Vec<u8>,
    expires_at: Instant,
}
#[derive(Clone)]
struct Active {
    owner_id: String,
    identity: TargetIdentity,
    spec: PatchSpec,
    original: Vec<u8>,
    status: PatchStatus,
}
pub struct PatchManager {
    pending: HashMap<String, Pending>,
    active: HashMap<String, Active>,
    audit: AuditStore,
    ttl: Duration,
}

impl PatchManager {
    pub fn new(audit: AuditStore, ttl: Duration) -> Self {
        Self {
            pending: HashMap::new(),
            active: HashMap::new(),
            audit,
            ttl,
        }
    }
    pub fn audit(&self) -> &AuditStore {
        &self.audit
    }
    pub fn preview<T: PatchTarget>(
        &mut self,
        target: &T,
        spec: PatchSpec,
    ) -> Result<PatchPreview, PatchError> {
        spec.validate()?;
        let identity = target.identity()?;
        identity.validate_fixture()?;
        if self.overlaps(&identity, spec.address, spec.desired.len()) {
            return Err(PatchError::Overlap);
        }
        let expected = target.read(spec.address, spec.desired.len())?;
        if expected == spec.desired {
            return Err(PatchError::Noop);
        }
        self.validate_identity(target, &identity)?;
        let preview_id = Uuid::new_v4().to_string();
        let preview = PatchPreview {
            preview_id: preview_id.clone(),
            session_id: identity.session_id.clone(),
            epoch: identity.epoch,
            game_id: identity.game_id.clone(),
            feature_id: spec.feature_id.to_string(),
            profile_id: identity.profile_id.clone(),
            address_display: format!("0x{:08X}", spec.address),
            before_display: read_seed(&expected)
                .map_or_else(|| "<unavailable>".into(), |v| format!("0x{v:08X}")),
            after_display: read_seed(&spec.desired)
                .map_or_else(|| "<unavailable>".into(), |v| format!("0x{v:08X}")),
            expires_in_seconds: self.ttl.as_secs(),
            risk: "Fixture only: no emulator memory is changed".into(),
            environment: "fixture".into(),
            status: PatchStatus::Pending,
        };
        self.event(
            &identity,
            spec.feature_id,
            Some(&preview_id),
            None,
            "preview",
            "accepted",
            "Fixture preview created",
            None,
        )?;
        self.pending.insert(
            preview_id.clone(),
            Pending {
                preview_id,
                identity,
                spec,
                expected,
                expires_at: Instant::now() + self.ttl,
            },
        );
        Ok(preview)
    }
    pub fn cancel_preview(&mut self, preview_id: &str) -> Result<(), PatchError> {
        let pending = self
            .pending
            .remove(preview_id)
            .ok_or(PatchError::UnknownPreview)?;
        self.event(
            &pending.identity,
            pending.spec.feature_id,
            Some(preview_id),
            None,
            "preview_cancelled",
            "ok",
            "Fixture preview discarded",
            None,
        )?;
        Ok(())
    }
    pub fn apply<T: PatchTarget>(
        &mut self,
        target: &mut T,
        preview_id: &str,
    ) -> Result<PatchReceipt, PatchError> {
        // Consume BEFORE any subsequent validation: a rejected/expired token cannot be replayed.
        let pending = self
            .pending
            .remove(preview_id)
            .ok_or(PatchError::UnknownPreview)?;
        if Instant::now() >= pending.expires_at {
            return Err(PatchError::PreviewExpired);
        }
        pending.identity.validate_fixture()?;
        self.validate_identity(target, &pending.identity)?;
        if self.overlaps(
            &pending.identity,
            pending.spec.address,
            pending.spec.desired.len(),
        ) {
            return Err(PatchError::Overlap);
        }
        let current = target.read(pending.spec.address, pending.expected.len())?;
        if current != pending.expected {
            self.event(
                &pending.identity,
                pending.spec.feature_id,
                Some(&pending.preview_id),
                None,
                "apply_rejected",
                "conflict",
                "Fixture bytes changed before apply",
                Some("EXPECTED_MISMATCH"),
            )?;
            return Err(PatchError::Conflict);
        }
        self.validate_identity(target, &pending.identity)?;
        let owner_id = Uuid::new_v4().to_string();
        // Durable, append-only intent MUST commit before any target mutation.
        self.event(
            &pending.identity,
            pending.spec.feature_id,
            Some(&pending.preview_id),
            Some(&owner_id),
            "apply_intent",
            "pending",
            "Fixture patch was requested",
            None,
        )?;
        let active = Active {
            owner_id: owner_id.clone(),
            identity: pending.identity.clone(),
            spec: pending.spec,
            original: pending.expected,
            status: PatchStatus::Active,
        };
        if let Err(write_error) = target.write(active.spec.address, &active.spec.desired) {
            return self.rollback_after_failure(target, active, "WRITE_FAILED", write_error);
        }
        let result = target.read(active.spec.address, active.original.len());
        if !matches!(&result, Ok(bytes) if bytes == &active.spec.desired)
            || self.validate_identity(target, &active.identity).is_err()
        {
            return self.rollback_after_failure(
                target,
                active,
                "VERIFY_FAILED",
                PatchError::VerifyFailed,
            );
        }
        // Keep an owned patch in memory even if the subsequent audit outcome fails.
        self.active.insert(owner_id.clone(), active.clone());
        if let Err(error) = self.event(
            &active.identity,
            active.spec.feature_id,
            Some(preview_id),
            Some(&owner_id),
            "applied",
            "ok",
            "Fixture patch applied and read back",
            None,
        ) {
            let _ = self.restore(target, &owner_id);
            return Err(error);
        }
        Ok(receipt(&active))
    }
    fn rollback_after_failure<T: PatchTarget>(
        &mut self,
        target: &mut T,
        mut active: Active,
        reason: &str,
        original_error: PatchError,
    ) -> Result<PatchReceipt, PatchError> {
        // Only undo if the entire value equals our proposed bytes AND identity is intact.
        // Partial/foreign writes must be reported, NOT forcibly overwritten.
        let current = target.read(active.spec.address, active.original.len());
        let still_same_identity = self.validate_identity(target, &active.identity).is_ok();
        if matches!(&current, Ok(bytes) if bytes == &active.original) && still_same_identity {
            self.event(
                &active.identity,
                active.spec.feature_id,
                None,
                Some(&active.owner_id),
                "rollback_confirmed",
                "ok",
                "Target remained unchanged after failure",
                Some(reason),
            )?;
            return Err(original_error);
        }
        if matches!(&current, Ok(bytes) if bytes == &active.spec.desired) && still_same_identity {
            if target.write(active.spec.address, &active.original).is_ok()
                && target
                    .read(active.spec.address, active.original.len())
                    .as_deref()
                    == Ok(active.original.as_slice())
                && self.validate_identity(target, &active.identity).is_ok()
            {
                self.event(
                    &active.identity,
                    active.spec.feature_id,
                    None,
                    Some(&active.owner_id),
                    "rollback_confirmed",
                    "ok",
                    "Exact fixture bytes restored after failed apply",
                    Some(reason),
                )?;
                return Err(original_error);
            }
        }
        active.status = PatchStatus::RestoreFailed;
        let _ = self.event(
            &active.identity,
            active.spec.feature_id,
            None,
            Some(&active.owner_id),
            "restore_failed",
            "error",
            "Potentially partial patch; manual inspection required",
            Some(reason),
        );
        self.active.insert(active.owner_id.clone(), active);
        Err(PatchError::RestoreFailed)
    }
    pub fn restore<T: PatchTarget>(
        &mut self,
        target: &mut T,
        owner_id: &str,
    ) -> Result<PatchReceipt, PatchError> {
        let mut active = self
            .active
            .get(owner_id)
            .cloned()
            .ok_or(PatchError::UnknownOwner)?;
        active.identity.validate_fixture()?;
        self.validate_identity(target, &active.identity)?;
        let current = target.read(active.spec.address, active.original.len())?;
        if current != active.spec.desired {
            active.status = PatchStatus::Conflict;
            self.active.insert(owner_id.to_owned(), active.clone());
            self.event(
                &active.identity,
                active.spec.feature_id,
                None,
                Some(owner_id),
                "restore_conflict",
                "conflict",
                "Restore denied: target is not owned patch bytes",
                Some("EXPECTED_MISMATCH"),
            )?;
            return Err(PatchError::Conflict);
        }
        self.event(
            &active.identity,
            active.spec.feature_id,
            None,
            Some(owner_id),
            "restore_intent",
            "pending",
            "Fixture patch restore requested",
            None,
        )?;
        if target.write(active.spec.address, &active.original).is_err()
            || target
                .read(active.spec.address, active.original.len())
                .as_deref()
                != Ok(active.original.as_slice())
            || self.validate_identity(target, &active.identity).is_err()
        {
            active.status = PatchStatus::RestoreFailed;
            self.active.insert(owner_id.to_owned(), active.clone());
            let _ = self.event(
                &active.identity,
                active.spec.feature_id,
                None,
                Some(owner_id),
                "restore_failed",
                "error",
                "Restore not verified; owner retained",
                Some("RESTORE_FAILED"),
            );
            return Err(PatchError::RestoreFailed);
        }
        // Only remove memory ownership once SQLite confirms restored receipt.
        self.event(
            &active.identity,
            active.spec.feature_id,
            None,
            Some(owner_id),
            "restored",
            "ok",
            "Original fixture value restored and verified",
            None,
        )?;
        self.active.remove(owner_id);
        active.status = PatchStatus::Restored;
        Ok(receipt(&active))
    }
    pub fn active(&self) -> Vec<PatchReceipt> {
        self.active.values().map(receipt).collect()
    }
    pub fn clear_fixture_session(&mut self) {
        // On reset memory is abandoned; no attempt to replay persisted audit into a new session.
        self.pending.clear();
        self.active.clear();
    }
    fn overlaps(&self, identity: &TargetIdentity, address: u32, len: usize) -> bool {
        let overlaps = |addr: u32, span: usize| {
            let start = u64::from(address);
            let end = start + len as u64;
            let other_start = u64::from(addr);
            let other_end = other_start + span as u64;
            start < other_end && other_start < end
        };
        self.pending.values().any(|p| {
            p.identity.session_id == identity.session_id
                && overlaps(p.spec.address, p.spec.desired.len())
        }) || self.active.values().any(|p| {
            p.identity.session_id == identity.session_id
                && overlaps(p.spec.address, p.spec.desired.len())
        })
    }
    fn validate_identity<T: PatchTarget>(
        &self,
        target: &T,
        expected: &TargetIdentity,
    ) -> Result<(), PatchError> {
        let now = target.identity()?;
        now.validate_fixture()?;
        if now != *expected {
            return Err(PatchError::StaleSession);
        }
        Ok(())
    }
    fn event(
        &mut self,
        identity: &TargetIdentity,
        feature_id: &str,
        preview_id: Option<&str>,
        owner_id: Option<&str>,
        kind: &str,
        status: &str,
        summary: &str,
        error: Option<&str>,
    ) -> Result<(), PatchError> {
        let row = ActionEvent::new(
            Uuid::new_v4().to_string(),
            EventContext {
                owner_id: owner_id.map(str::to_owned),
                preview_id: preview_id.map(str::to_owned),
                session_id: identity.session_id.clone(),
                game_id: identity.game_id.clone(),
                feature_id: feature_id.to_owned(),
                environment: "fixture".into(),
            },
            kind,
            status,
            summary,
            error,
        );
        self.audit.append(&row)?;
        Ok(())
    }
}
fn read_seed(bytes: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}
fn receipt(active: &Active) -> PatchReceipt {
    PatchReceipt {
        owner_id: active.owner_id.clone(),
        session_id: active.identity.session_id.clone(),
        feature_id: active.spec.feature_id.into(),
        status: active.status.clone(),
        environment: "fixture".into(),
        audited: true,
    }
}

/// Explicitly synthetic memory: NEVER attached to mGBA or any OS process.
pub struct FixtureTarget {
    identity: TargetIdentity,
    seed: u32,
    fail_next_write: bool,
}
impl Default for FixtureTarget {
    fn default() -> Self {
        Self::new()
    }
}
impl FixtureTarget {
    pub fn new() -> Self {
        Self {
            identity: TargetIdentity {
                session_id: Uuid::new_v4().to_string(),
                epoch: 1,
                process_identity: "in-memory-only".into(),
                runtime_id: "runtime.fixture.memory".into(),
                game_id: GAME_ID.into(),
                build_id: "synthetic-emerald-rev0".into(),
                profile_id: FIXTURE_PROFILE_ID.into(),
                environment: "fixture".into(),
            },
            seed: 0x1234_5678,
            fail_next_write: false,
        }
    }
    pub fn snapshot(&self, patches: Vec<PatchReceipt>) -> FixtureSnapshot {
        FixtureSnapshot {
            session_id: self.identity.session_id.clone(),
            epoch: self.identity.epoch,
            seed: self.seed,
            active_patches: patches,
            read_only_game_runtime: true,
            environment: "fixture".into(),
        }
    }
    #[cfg(test)]
    fn change_seed_externally(&mut self, seed: u32) {
        self.seed = seed;
    }
    #[cfg(test)]
    fn bump_epoch(&mut self) {
        self.identity.epoch += 1;
    }
    #[cfg(test)]
    fn change_build(&mut self) {
        self.identity.build_id = "different build".into();
    }
    #[cfg(test)]
    fn fail_one_write(&mut self) {
        self.fail_next_write = true;
    }
}
impl PatchTarget for FixtureTarget {
    fn identity(&self) -> Result<TargetIdentity, PatchError> {
        Ok(self.identity.clone())
    }
    fn read(&self, address: u32, len: usize) -> Result<Vec<u8>, PatchError> {
        if address != FIXTURE_SEED_ADDRESS || len != 4 {
            return Err(PatchError::InvalidSpan);
        }
        Ok(self.seed.to_le_bytes().to_vec())
    }
    fn write(&mut self, address: u32, bytes: &[u8]) -> Result<(), PatchError> {
        if address != FIXTURE_SEED_ADDRESS || bytes.len() != 4 {
            return Err(PatchError::InvalidSpan);
        }
        if self.fail_next_write {
            self.fail_next_write = false;
            return Err(PatchError::Target("injected fixture write failure".into()));
        }
        self.seed = read_seed(bytes).ok_or(PatchError::InvalidSpan)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn manager() -> PatchManager {
        PatchManager::new(AuditStore::open_memory().unwrap(), Duration::from_secs(30))
    }
    fn preview(manager: &mut PatchManager, target: &FixtureTarget) -> PatchPreview {
        manager
            .preview(target, PatchSpec::emerald_fixture_seed(0xF00D_BABE))
            .unwrap()
    }
    #[test]
    fn preview_has_no_side_effect_and_audits_fixture_only() {
        let mut manager = manager();
        let target = FixtureTarget::new();
        let p = preview(&mut manager, &target);
        assert_eq!(target.seed, 0x1234_5678);
        assert_eq!(p.before_display, "0x12345678");
        assert_eq!(p.after_display, "0xF00DBABE");
        assert_eq!(manager.audit().list("fixture", 50).unwrap().len(), 1);
        assert!(manager.audit().list("live", 50).unwrap().is_empty());
    }
    #[test]
    fn apply_reads_back_and_restores_owned_bytes() {
        let mut manager = manager();
        let mut target = FixtureTarget::new();
        let p = preview(&mut manager, &target);
        let applied = manager.apply(&mut target, &p.preview_id).unwrap();
        assert_eq!(target.seed, 0xF00D_BABE);
        assert_eq!(applied.status, PatchStatus::Active);
        assert_eq!(manager.audit().unresolved_count("fixture").unwrap(), 1);
        let restored = manager.restore(&mut target, &applied.owner_id).unwrap();
        assert_eq!(restored.status, PatchStatus::Restored);
        assert_eq!(target.seed, 0x1234_5678);
        assert!(manager.active().is_empty());
        assert_eq!(manager.audit().unresolved_count("fixture").unwrap(), 0);
    }
    #[test]
    fn token_is_single_use() {
        let mut manager = manager();
        let mut target = FixtureTarget::new();
        let p = preview(&mut manager, &target);
        manager.apply(&mut target, &p.preview_id).unwrap();
        assert!(matches!(
            manager.apply(&mut target, &p.preview_id),
            Err(PatchError::UnknownPreview)
        ));
    }
    #[test]
    fn expired_token_is_never_applied() {
        let mut manager = PatchManager::new(AuditStore::open_memory().unwrap(), Duration::ZERO);
        let mut target = FixtureTarget::new();
        let p = preview(&mut manager, &target);
        assert!(matches!(
            manager.apply(&mut target, &p.preview_id),
            Err(PatchError::PreviewExpired)
        ));
        assert_eq!(target.seed, 0x1234_5678);
    }
    #[test]
    fn stale_epoch_and_build_are_rejected() {
        for build_change in [false, true] {
            let mut manager = manager();
            let mut target = FixtureTarget::new();
            let p = preview(&mut manager, &target);
            if build_change {
                target.change_build();
            } else {
                target.bump_epoch();
            }
            assert!(matches!(
                manager.apply(&mut target, &p.preview_id),
                Err(PatchError::StaleSession)
            ));
            assert_eq!(target.seed, 0x1234_5678);
        }
    }
    #[test]
    fn original_byte_conflicts_are_rejected_without_write() {
        let mut manager = manager();
        let mut target = FixtureTarget::new();
        let p = preview(&mut manager, &target);
        target.change_seed_externally(7);
        assert!(matches!(
            manager.apply(&mut target, &p.preview_id),
            Err(PatchError::Conflict)
        ));
        assert_eq!(target.seed, 7);
    }
    #[test]
    fn overlapping_pending_and_active_are_rejected() {
        let mut manager = manager();
        let mut target = FixtureTarget::new();
        let p = preview(&mut manager, &target);
        assert!(matches!(
            manager.preview(&target, PatchSpec::emerald_fixture_seed(1)),
            Err(PatchError::Overlap)
        ));
        let receipt = manager.apply(&mut target, &p.preview_id).unwrap();
        assert!(matches!(
            manager.preview(&target, PatchSpec::emerald_fixture_seed(2)),
            Err(PatchError::Overlap)
        ));
        manager.restore(&mut target, &receipt.owner_id).unwrap();
        manager
            .preview(&target, PatchSpec::emerald_fixture_seed(3))
            .unwrap();
    }
    #[test]
    fn restore_never_overwrites_foreign_changes() {
        let mut manager = manager();
        let mut target = FixtureTarget::new();
        let p = preview(&mut manager, &target);
        let receipt = manager.apply(&mut target, &p.preview_id).unwrap();
        target.change_seed_externally(0xAA);
        assert!(matches!(
            manager.restore(&mut target, &receipt.owner_id),
            Err(PatchError::Conflict)
        ));
        assert_eq!(target.seed, 0xAA);
        assert_eq!(manager.active()[0].status, PatchStatus::Conflict);
    }
    #[test]
    fn failed_write_preserves_original_value_and_audits() {
        let mut manager = manager();
        let mut target = FixtureTarget::new();
        let p = preview(&mut manager, &target);
        target.fail_one_write();
        assert!(matches!(
            manager.apply(&mut target, &p.preview_id),
            Err(PatchError::Target(_))
        ));
        assert_eq!(target.seed, 0x1234_5678);
        assert_eq!(manager.audit().unresolved_count("fixture").unwrap(), 0);
    }
    #[test]
    fn failed_restore_remains_owned_and_can_retry() {
        let mut manager = manager();
        let mut target = FixtureTarget::new();
        let p = preview(&mut manager, &target);
        let receipt = manager.apply(&mut target, &p.preview_id).unwrap();
        target.fail_one_write();
        assert!(matches!(
            manager.restore(&mut target, &receipt.owner_id),
            Err(PatchError::RestoreFailed)
        ));
        assert_eq!(target.seed, 0xF00D_BABE);
        assert_eq!(manager.active()[0].status, PatchStatus::RestoreFailed);
        manager.restore(&mut target, &receipt.owner_id).unwrap();
        assert_eq!(target.seed, 0x1234_5678);
    }
    #[test]
    fn denied_real_profile_never_writes() {
        let mut manager = manager();
        let mut target = FixtureTarget::new();
        target.identity.environment = "live".into();
        assert!(matches!(
            manager.preview(&target, PatchSpec::emerald_fixture_seed(1)),
            Err(PatchError::WriteLocked)
        ));
        assert_eq!(target.seed, 0x1234_5678);
    }
    #[test]
    fn cancelled_preview_cannot_be_applied() {
        let mut manager = manager();
        let mut target = FixtureTarget::new();
        let p = preview(&mut manager, &target);
        manager.cancel_preview(&p.preview_id).unwrap();
        assert!(matches!(
            manager.apply(&mut target, &p.preview_id),
            Err(PatchError::UnknownPreview)
        ));
    }
    #[test]
    fn reset_disallows_stale_preview_and_old_owner() {
        let mut manager = manager();
        let target = FixtureTarget::new();
        let p = preview(&mut manager, &target);
        manager.clear_fixture_session();
        let mut second = FixtureTarget::new();
        assert!(matches!(
            manager.apply(&mut second, &p.preview_id),
            Err(PatchError::UnknownPreview)
        ));
    }
}
