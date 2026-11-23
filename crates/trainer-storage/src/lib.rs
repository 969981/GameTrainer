//! Append-only SQLite ledger. A session is not recovered from persisted events:
//! incomplete/active intents are surfaced for inspection, never replayed.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

pub const SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Error)]
pub enum AuditError {
    #[error("SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("audit directory: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid audit event")]
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionEvent {
    pub event_id: String,
    pub owner_id: Option<String>,
    pub preview_id: Option<String>,
    pub session_id: String,
    pub game_id: String,
    pub feature_id: String,
    pub environment: String,
    pub action_kind: String,
    pub status: String,
    pub summary: String,
    pub error_code: Option<String>,
    pub occurred_at_ms: i64,
}

impl ActionEvent {
    /// Only human-safe descriptions: never persist raw memory dumps or credentials.
    pub fn new(
        event_id: String,
        owner_id: Option<String>,
        preview_id: Option<String>,
        session_id: String,
        game_id: String,
        feature_id: String,
        environment: &str,
        action_kind: &str,
        status: &str,
        summary: &str,
        error_code: Option<&str>,
    ) -> Self {
        let occurred_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
            .unwrap_or(0);
        Self {
            event_id,
            owner_id,
            preview_id,
            session_id,
            game_id,
            feature_id,
            environment: environment.to_owned(),
            action_kind: action_kind.to_owned(),
            status: status.to_owned(),
            summary: summary.chars().take(200).collect(),
            error_code: error_code.map(|v| v.chars().take(48).collect()),
            occurred_at_ms,
        }
    }
}

pub struct AuditStore {
    connection: Connection,
}
impl AuditStore {
    pub fn open_file(path: &Path) -> Result<Self, AuditError> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let connection = Connection::open(path)?;
        // WAL for concurrency, FULL sync before any external mutation.
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.busy_timeout(std::time::Duration::from_secs(3))?;
        Self::initialize(connection)
    }
    pub fn open_memory() -> Result<Self, AuditError> {
        Self::initialize(Connection::open_in_memory()?)
    }
    fn initialize(connection: Connection) -> Result<Self, AuditError> {
        connection
            .execute_batch(
                "PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY
             );
             CREATE TABLE IF NOT EXISTS action_events (
                event_id TEXT PRIMARY KEY NOT NULL,
                owner_id TEXT,
                preview_id TEXT,
                session_id TEXT NOT NULL,
                game_id TEXT NOT NULL,
                feature_id TEXT NOT NULL,
                environment TEXT NOT NULL CHECK(environment IN ('fixture','live')),
                action_kind TEXT NOT NULL,
                status TEXT NOT NULL,
                summary TEXT NOT NULL,
                error_code TEXT,
                occurred_at_ms INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_action_events_scope_time
                ON action_events(environment, occurred_at_ms DESC, rowid);",
            )
            .or_else(|error| {
                // SQLite rejects rowid in indexed expressions. Preserve any existing
                // schema and create a portable index instead.
                if error.to_string().contains("no such column: rowid") {
                    connection.execute_batch(
                        "CREATE INDEX IF NOT EXISTS idx_action_events_scope_time
                     ON action_events(environment, occurred_at_ms DESC);",
                    )
                } else {
                    Err(error)
                }
            })?;
        connection.execute(
            "INSERT OR IGNORE INTO schema_migrations(version) VALUES (?1)",
            params![SCHEMA_VERSION],
        )?;
        Ok(Self { connection })
    }
    pub fn append(&mut self, event: &ActionEvent) -> Result<(), AuditError> {
        if event.session_id.is_empty()
            || event.game_id.is_empty()
            || event.feature_id.is_empty()
            || event.event_id.is_empty()
            || !matches!(event.environment.as_str(), "fixture" | "live")
            || event.summary.chars().count() > 200
            || event.action_kind.len() > 48
        {
            return Err(AuditError::Invalid);
        }
        let tx = self.connection.transaction()?;
        tx.execute(
            "INSERT INTO action_events (
                event_id, owner_id, preview_id, session_id, game_id, feature_id,
                environment, action_kind, status, summary, error_code, occurred_at_ms
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                event.event_id,
                event.owner_id,
                event.preview_id,
                event.session_id,
                event.game_id,
                event.feature_id,
                event.environment,
                event.action_kind,
                event.status,
                event.summary,
                event.error_code,
                event.occurred_at_ms
            ],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn list(&self, environment: &str, limit: u32) -> Result<Vec<ActionEvent>, AuditError> {
        if !matches!(environment, "fixture" | "live") {
            return Err(AuditError::Invalid);
        }
        let mut statement = self.connection.prepare(
            "SELECT event_id, owner_id, preview_id, session_id, game_id,
               feature_id, environment, action_kind, status, summary, error_code, occurred_at_ms
             FROM action_events WHERE environment=?1
             ORDER BY occurred_at_ms DESC, rowid DESC LIMIT ?2",
        )?;
        let iter = statement.query_map(params![environment, limit.min(200)], |row| {
            Ok(ActionEvent {
                event_id: row.get(0)?,
                owner_id: row.get(1)?,
                preview_id: row.get(2)?,
                session_id: row.get(3)?,
                game_id: row.get(4)?,
                feature_id: row.get(5)?,
                environment: row.get(6)?,
                action_kind: row.get(7)?,
                status: row.get(8)?,
                summary: row.get(9)?,
                error_code: row.get(10)?,
                occurred_at_ms: row.get(11)?,
            })
        })?;
        Ok(iter.collect::<Result<Vec<_>, _>>()?)
    }
    /// Active or interrupted apply intents from a prior session; NEVER auto-restore.
    pub fn unresolved_count(&self, environment: &str) -> Result<u64, AuditError> {
        if !matches!(environment, "fixture" | "live") {
            return Err(AuditError::Invalid);
        }
        let total: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM action_events AS intent
             WHERE intent.environment=?1 AND intent.action_kind='apply_intent'
             AND NOT EXISTS (
                 SELECT 1 FROM action_events AS end_event
                 WHERE end_event.owner_id = intent.owner_id
                 AND end_event.action_kind IN ('restored', 'rollback_confirmed')
             )",
            params![environment],
            |row| row.get(0),
        )?;
        Ok(total as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(kind: &str, status: &str) -> ActionEvent {
        ActionEvent::new(
            format!("{kind}-{status}"),
            Some("owner".into()),
            Some("preview".into()),
            "fixture-session".into(),
            "games.pokemon.emerald".into(),
            "features.gen3.rng".into(),
            "fixture",
            kind,
            status,
            "4-byte fixture mutation",
            None,
        )
    }

    #[test]
    fn events_are_append_only_and_separated_by_environment() {
        let mut store = AuditStore::open_memory().unwrap();
        store.append(&event("preview", "ok")).unwrap();
        assert_eq!(store.list("fixture", 20).unwrap().len(), 1);
        assert!(store.list("live", 20).unwrap().is_empty());
        assert!(store.append(&event("preview", "ok")).is_err());
    }
    #[test]
    fn interrupted_apply_survives_reopen_without_auto_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite3");
        {
            let mut store = AuditStore::open_file(&path).unwrap();
            store.append(&event("apply_intent", "pending")).unwrap();
            assert_eq!(store.unresolved_count("fixture").unwrap(), 1);
        }
        let mut reopened = AuditStore::open_file(&path).unwrap();
        assert_eq!(reopened.unresolved_count("fixture").unwrap(), 1);
        reopened.append(&event("restored", "ok")).unwrap();
        assert_eq!(reopened.unresolved_count("fixture").unwrap(), 0);
    }
    #[test]
    fn events_validate_environment_and_summary() {
        let mut store = AuditStore::open_memory().unwrap();
        let mut e = event("apply_intent", "pending");
        e.environment = "untrusted".into();
        assert!(matches!(store.append(&e), Err(AuditError::Invalid)));
        assert!(matches!(
            store.list("unknown", 10),
            Err(AuditError::Invalid)
        ));
    }
}
