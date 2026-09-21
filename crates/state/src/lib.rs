use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

const SCHEMA: &str = r#"
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS local_pr_state (
  pull_request_id TEXT PRIMARY KEY,
  acknowledged_fingerprint TEXT,
  acknowledged_at TEXT,
  snoozed_fingerprint TEXT,
  snoozed_until TEXT
);

CREATE TABLE IF NOT EXISTS attention_observations (
  pull_request_id TEXT PRIMARY KEY,
  is_attention_required INTEGER NOT NULL CHECK (is_attention_required IN (0, 1)),
  observed_fingerprint TEXT NOT NULL,
  observed_at TEXT NOT NULL,
  last_emitted_fingerprint TEXT,
  last_emitted_at TEXT
);

CREATE TABLE IF NOT EXISTS outstanding_feedback (
  pull_request_id TEXT NOT NULL,
  feedback_fingerprint TEXT NOT NULL,
  detected_at TEXT NOT NULL,
  PRIMARY KEY (pull_request_id, feedback_fingerprint)
);
"#;

#[derive(Debug)]
pub struct StateStore {
    connection: Connection,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct AttentionObservation {
    pub notification_due: bool,
    pub baselined: bool,
}

/// A local-only user action issued by a client for the currently projected PR.
///
/// Acknowledgement and snoozing deliberately include the current signal
/// fingerprint supplied with the card. State is never attached merely to a PR ID:
/// a changed signal makes the prior action inapplicable and reactivates the card.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LocalStateCommand {
    Acknowledge {
        pull_request_id: String,
        current_fingerprint: String,
    },
    SnoozeUntil {
        pull_request_id: String,
        current_fingerprint: String,
        until: DateTime<Utc>,
    },
    ClearSnooze {
        pull_request_id: String,
    },
}

impl StateStore {
    /// Location for per-device state in the operating system's application-data
    /// directory. This store contains no GitHub credentials.
    pub fn default_path() -> Result<PathBuf> {
        Ok(application_data_directory()?
            .join("review-radar")
            .join("review-radar-state.sqlite3"))
    }

    /// Open the per-device state database at its standard application-data path.
    pub fn open_default() -> Result<Self> {
        let path = Self::default_path()?;
        let parent = path
            .parent()
            .ok_or_else(|| anyhow!("state database path has no parent directory"))?;
        fs::create_dir_all(parent)?;
        Self::open(path)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    pub fn in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self> {
        connection.execute_batch(SCHEMA)?;
        Ok(Self { connection })
    }

    /// Apply a command from a client without requiring GitHub access.
    pub fn apply(&self, command: LocalStateCommand, now: DateTime<Utc>) -> Result<()> {
        match command {
            LocalStateCommand::Acknowledge {
                pull_request_id,
                current_fingerprint,
            } => self.acknowledge(&pull_request_id, &current_fingerprint, now),
            LocalStateCommand::SnoozeUntil {
                pull_request_id,
                current_fingerprint,
                until,
            } => self.snooze_until(&pull_request_id, &current_fingerprint, until),
            LocalStateCommand::ClearSnooze { pull_request_id } => {
                self.clear_snooze(&pull_request_id)
            }
        }
    }

    pub fn acknowledge(
        &self,
        pull_request_id: &str,
        newest_event_fingerprint: &str,
        now: DateTime<Utc>,
    ) -> Result<()> {
        self.connection.execute(
            "INSERT INTO local_pr_state (pull_request_id, acknowledged_fingerprint, acknowledged_at)
             VALUES (?, ?, ?)
             ON CONFLICT(pull_request_id) DO UPDATE SET
               acknowledged_fingerprint = excluded.acknowledged_fingerprint,
               acknowledged_at = excluded.acknowledged_at",
            params![pull_request_id, newest_event_fingerprint, timestamp(now)],
        )?;
        self.connection.execute(
            "DELETE FROM outstanding_feedback
             WHERE pull_request_id = ? AND instr(?, feedback_fingerprint) > 0",
            params![pull_request_id, newest_event_fingerprint],
        )?;
        Ok(())
    }

    /// Retain feedback discovered by a capture independently of the capture's
    /// bounded event window. Repeated observations are idempotent.
    pub fn record_feedback(
        &self,
        pull_request_id: &str,
        fingerprints: &[String],
        now: DateTime<Utc>,
    ) -> Result<()> {
        for fingerprint in fingerprints {
            self.connection.execute(
                "INSERT OR IGNORE INTO outstanding_feedback
                 (pull_request_id, feedback_fingerprint, detected_at)
                 VALUES (?, ?, ?)",
                params![pull_request_id, fingerprint, timestamp(now)],
            )?;
        }
        Ok(())
    }

    pub fn outstanding_feedback(
        &self,
    ) -> Result<std::collections::BTreeMap<String, std::collections::BTreeSet<String>>> {
        let mut statement = self.connection.prepare(
            "SELECT pull_request_id, feedback_fingerprint
             FROM outstanding_feedback ORDER BY pull_request_id, feedback_fingerprint",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut feedback = std::collections::BTreeMap::new();
        for row in rows {
            let (pull_request_id, fingerprint) = row?;
            feedback
                .entry(pull_request_id)
                .or_insert_with(std::collections::BTreeSet::new)
                .insert(fingerprint);
        }
        Ok(feedback)
    }

    pub fn snooze_until(
        &self,
        pull_request_id: &str,
        newest_event_fingerprint: &str,
        until: DateTime<Utc>,
    ) -> Result<()> {
        self.connection.execute(
            "INSERT INTO local_pr_state (pull_request_id, snoozed_fingerprint, snoozed_until)
             VALUES (?, ?, ?)
             ON CONFLICT(pull_request_id) DO UPDATE SET
               snoozed_fingerprint = excluded.snoozed_fingerprint,
               snoozed_until = excluded.snoozed_until",
            params![pull_request_id, newest_event_fingerprint, timestamp(until)],
        )?;
        Ok(())
    }

    pub fn clear_snooze(&self, pull_request_id: &str) -> Result<()> {
        self.connection.execute(
            "UPDATE local_pr_state SET snoozed_fingerprint = NULL, snoozed_until = NULL
             WHERE pull_request_id = ?",
            [pull_request_id],
        )?;
        Ok(())
    }

    pub fn is_suppressed(
        &self,
        pull_request_id: &str,
        newest_event_fingerprint: &str,
        now: DateTime<Utc>,
    ) -> Result<bool> {
        let state = self
            .connection
            .query_row(
                "SELECT acknowledged_fingerprint, snoozed_fingerprint, snoozed_until
                 FROM local_pr_state WHERE pull_request_id = ?",
                [pull_request_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()?;
        let Some((acknowledged, snoozed, snoozed_until)) = state else {
            return Ok(false);
        };
        let acknowledged = acknowledged.as_deref() == Some(newest_event_fingerprint);
        let snoozed = snoozed.as_deref() == Some(newest_event_fingerprint)
            && snoozed_until
                .as_deref()
                .is_some_and(|until| until > timestamp(now).as_str());
        Ok(acknowledged || snoozed)
    }

    pub fn observe_attention(
        &self,
        pull_request_id: &str,
        is_attention_required: bool,
        attention_fingerprint: &str,
        now: DateTime<Utc>,
    ) -> Result<AttentionObservation> {
        let previous = self
            .connection
            .query_row(
                "SELECT is_attention_required FROM attention_observations WHERE pull_request_id = ?",
                [pull_request_id],
                |row| row.get::<_, bool>(0),
            )
            .optional()?;
        let baselined = previous.is_none();
        let notification_due = previous == Some(false) && is_attention_required;
        let emitted = if notification_due {
            Some(attention_fingerprint)
        } else {
            None
        };
        self.connection.execute(
            "INSERT INTO attention_observations
             (pull_request_id, is_attention_required, observed_fingerprint, observed_at,
              last_emitted_fingerprint, last_emitted_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(pull_request_id) DO UPDATE SET
               is_attention_required = excluded.is_attention_required,
               observed_fingerprint = excluded.observed_fingerprint,
               observed_at = excluded.observed_at,
               last_emitted_fingerprint = COALESCE(excluded.last_emitted_fingerprint, attention_observations.last_emitted_fingerprint),
               last_emitted_at = COALESCE(excluded.last_emitted_at, attention_observations.last_emitted_at)",
            params![
                pull_request_id,
                is_attention_required,
                attention_fingerprint,
                timestamp(now),
                emitted,
                notification_due.then(|| timestamp(now)),
            ],
        )?;
        Ok(AttentionObservation {
            notification_due,
            baselined,
        })
    }
}

#[cfg(target_os = "linux")]
fn application_data_directory() -> Result<PathBuf> {
    linux_data_directory(env::var_os("XDG_DATA_HOME"), env::var_os("HOME"))
        .ok_or_else(|| anyhow!("cannot determine XDG data directory; set XDG_DATA_HOME or HOME"))
}

#[cfg(target_os = "linux")]
fn linux_data_directory(
    xdg_data_home: Option<OsString>,
    home: Option<OsString>,
) -> Option<PathBuf> {
    xdg_data_home
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            home.filter(|path| !path.is_empty())
                .map(|path| PathBuf::from(path).join(".local/share"))
        })
}

#[cfg(target_os = "macos")]
fn application_data_directory() -> Result<PathBuf> {
    env::var_os("HOME")
        .filter(|path| !path.is_empty())
        .map(|path| PathBuf::from(path).join("Library/Application Support"))
        .ok_or_else(|| anyhow!("cannot determine application data directory; set HOME"))
}

#[cfg(target_os = "windows")]
fn application_data_directory() -> Result<PathBuf> {
    env::var_os("APPDATA")
        .or_else(|| env::var_os("LOCALAPPDATA"))
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("cannot determine application data directory; set APPDATA"))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn application_data_directory() -> Result<PathBuf> {
    env::var_os("HOME")
        .filter(|path| !path.is_empty())
        .map(|path| PathBuf::from(path).join(".local/share"))
        .ok_or_else(|| anyhow!("cannot determine application data directory; set HOME"))
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use std::{env, fs, time::SystemTime};

    use chrono::{Duration, TimeZone};

    use super::*;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 21, 12, 0, 0).unwrap()
    }

    #[test]
    fn acknowledgement_and_snooze_reactivate_on_new_meaningful_event() {
        let store = StateStore::in_memory().unwrap();
        let initial = "review:node-001:2026-09-21T11:00:00Z";
        let newer = "review:node-002:2026-09-21T12:00:00Z";
        let newest = "review:node-003:2026-09-21T13:00:00Z";

        store.acknowledge("pr-1", initial, now()).unwrap();
        assert!(store.is_suppressed("pr-1", initial, now()).unwrap());
        assert!(!store.is_suppressed("pr-1", newer, now()).unwrap());

        store
            .snooze_until("pr-1", newer, now() + Duration::hours(4))
            .unwrap();
        assert!(store.is_suppressed("pr-1", newer, now()).unwrap());
        assert!(!store
            .is_suppressed("pr-1", newer, now() + Duration::hours(4))
            .unwrap());
        assert!(!store.is_suppressed("pr-1", newest, now()).unwrap());
    }

    #[test]
    fn attention_notifications_are_baselined_and_deduplicated() {
        let store = StateStore::in_memory().unwrap();
        let fingerprint = "changes-requested:review-1";

        assert_eq!(
            store
                .observe_attention("pr-1", true, fingerprint, now())
                .unwrap(),
            AttentionObservation {
                notification_due: false,
                baselined: true,
            }
        );
        assert!(
            !store
                .observe_attention("pr-1", true, fingerprint, now())
                .unwrap()
                .notification_due
        );
        assert!(
            !store
                .observe_attention("pr-1", false, "healthy", now())
                .unwrap()
                .notification_due
        );
        assert!(
            store
                .observe_attention("pr-1", true, "review-requested:review-2", now())
                .unwrap()
                .notification_due
        );
        assert!(
            !store
                .observe_attention("pr-1", true, "review-requested:review-2", now())
                .unwrap()
                .notification_due
        );
    }

    #[test]
    fn snooze_can_be_cleared_early() {
        let store = StateStore::in_memory().unwrap();
        store
            .snooze_until("pr-1", "event-1", now() + Duration::days(1))
            .unwrap();
        store.clear_snooze("pr-1").unwrap();
        assert!(!store.is_suppressed("pr-1", "event-1", now()).unwrap());
    }

    #[test]
    fn commands_require_the_projected_fingerprint_to_suppress_a_card() {
        let store = StateStore::in_memory().unwrap();
        store
            .apply(
                LocalStateCommand::Acknowledge {
                    pull_request_id: "pr-1".into(),
                    current_fingerprint: "event-1".into(),
                },
                now(),
            )
            .unwrap();
        assert!(store.is_suppressed("pr-1", "event-1", now()).unwrap());
        assert!(!store.is_suppressed("pr-1", "event-2", now()).unwrap());
    }

    #[test]
    fn outstanding_feedback_survives_refresh_and_is_cleared_by_acknowledgement() {
        let store = StateStore::in_memory().unwrap();
        store
            .record_feedback("pr-1", &["comment:1:v1".into()], now())
            .unwrap();
        assert!(store.outstanding_feedback().unwrap()["pr-1"].contains("comment:1:v1"));
        store
            .acknowledge("pr-1", "current:pr-1:comment:2:v2", now())
            .unwrap();
        assert!(store.outstanding_feedback().unwrap()["pr-1"].contains("comment:1:v1"));
        store
            .acknowledge("pr-1", "current:pr-1:comment:1:v1", now())
            .unwrap();
        assert!(store.outstanding_feedback().unwrap().is_empty());
    }

    #[test]
    fn local_state_survives_a_store_restart() {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("review-radar-state-{unique}.sqlite3"));
        {
            let store = StateStore::open(&path).unwrap();
            store.acknowledge("pr-1", "event-1", now()).unwrap();
        }
        let reopened = StateStore::open(&path).unwrap();
        assert!(reopened.is_suppressed("pr-1", "event-1", now()).unwrap());
        fs::remove_file(&path).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_state_path_uses_xdg_data_home_then_home_fallback() {
        assert_eq!(
            linux_data_directory(
                Some("/tmp/review-radar-data".into()),
                Some("/tmp/home".into())
            ),
            Some(PathBuf::from("/tmp/review-radar-data"))
        );
        assert_eq!(
            linux_data_directory(None, Some("/tmp/home".into())),
            Some(PathBuf::from("/tmp/home/.local/share"))
        );
    }
}
