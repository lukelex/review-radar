use std::{env, path::PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use review_radar_state::{LocalStateCommand, StateStore};
use serde_json::json;

#[derive(Debug, Eq, PartialEq)]
struct Config {
    database: PathBuf,
    command: LocalStateCommand,
}

fn main() -> Result<()> {
    let config = parse_config(env::args().skip(1))?;
    let store = StateStore::open(&config.database)
        .with_context(|| format!("cannot open {}", config.database.display()))?;
    store.apply(config.command, Utc::now())?;
    println!("{}", serde_json::to_string(&json!({ "ok": true }))?);
    Ok(())
}

fn parse_config(args: impl Iterator<Item = String>) -> Result<Config> {
    let mut args = args.peekable();
    let mut database: PathBuf = "/data/review-radar-state.sqlite3".into();
    if args.peek().is_some_and(|arg| arg == "--database") {
        args.next();
        database = next_value(&mut args, "--database")?.into();
    }
    let operation = args.next().ok_or_else(|| anyhow!("missing command"))?;
    let mut pull_request_id = None;
    let mut fingerprint = None;
    let mut until = None;
    while let Some(flag) = args.next() {
        let value = next_value(&mut args, &flag)?;
        match flag.as_str() {
            "--pull-request-id" => pull_request_id = Some(value),
            "--fingerprint" => fingerprint = Some(value),
            "--until" => until = Some(parse_timestamp(&value)?),
            _ => bail!("unknown argument: {flag}"),
        }
    }
    let pull_request_id = pull_request_id.ok_or_else(|| anyhow!("missing --pull-request-id"))?;
    let command = match operation.as_str() {
        "acknowledge" => LocalStateCommand::Acknowledge {
            pull_request_id,
            current_fingerprint: fingerprint.ok_or_else(|| anyhow!("missing --fingerprint"))?,
        },
        "snooze" => LocalStateCommand::SnoozeUntil {
            pull_request_id,
            current_fingerprint: fingerprint.ok_or_else(|| anyhow!("missing --fingerprint"))?,
            until: until.ok_or_else(|| anyhow!("missing --until"))?,
        },
        "clear-snooze" => {
            if fingerprint.is_some() || until.is_some() {
                bail!("clear-snooze does not accept --fingerprint or --until");
            }
            LocalStateCommand::ClearSnooze { pull_request_id }
        }
        _ => bail!("invalid command {operation:?}; use acknowledge, snooze, or clear-snooze"),
    };
    Ok(Config { database, command })
}

fn next_value(
    args: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    flag: &str,
) -> Result<String> {
    args.next()
        .ok_or_else(|| anyhow!("missing value for {flag}"))
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("invalid RFC 3339 timestamp {value:?}"))?
        .with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_state_commands_with_required_current_fingerprints() {
        let config = parse_config(
            [
                "--database".into(),
                "/tmp/state.sqlite3".into(),
                "snooze".into(),
                "--pull-request-id".into(),
                "pr-1".into(),
                "--fingerprint".into(),
                "current:pr-1".into(),
                "--until".into(),
                "2026-09-22T12:00:00Z".into(),
            ]
            .into_iter(),
        )
        .unwrap();
        assert_eq!(config.database, PathBuf::from("/tmp/state.sqlite3"));
        assert!(matches!(
            config.command,
            LocalStateCommand::SnoozeUntil { .. }
        ));
        assert!(parse_config(["acknowledge".into()].into_iter()).is_err());
        assert!(parse_config(
            [
                "acknowledge".into(),
                "--pull-request-id".into(),
                "pr-1".into(),
            ]
            .into_iter()
        )
        .is_err());
    }
}
