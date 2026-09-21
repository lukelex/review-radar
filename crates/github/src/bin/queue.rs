use std::{env, path::PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use review_radar_domain::{
    NewestActivityRanking, PullRequestCard, RankingStrategy, Snapshot, TailoredRanking,
    WorkspaceView,
};
use review_radar_state::StateStore;
use rusqlite::{params, Connection, OpenFlags};
use serde_json::{json, Value};

#[derive(Debug)]
struct Config {
    database: PathBuf,
    capture_id: Option<i64>,
    view: WorkspaceView,
    ranking: String,
    state_database: PathBuf,
    record_attention: bool,
}

fn main() -> Result<()> {
    let config = parse_config(env::args().skip(1))?;
    let ranking = ranking(&config.ranking)?;
    let connection = Connection::open_with_flags(
        &config.database,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )
    .with_context(|| format!("cannot open {}", config.database.display()))?;
    let capture_id = config.capture_id.unwrap_or(latest_capture_id(&connection)?);
    let (captured_at, snapshot) = load_snapshot(&connection, capture_id)?;
    let cards = snapshot.view_with_ranking(config.view, ranking.as_ref());
    let state = StateStore::open(&config.state_database)
        .with_context(|| format!("cannot open {}", config.state_database.display()))?;
    let projection = apply_local_state(cards, &state, config.record_attention, Utc::now())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "captureId": capture_id,
            "capturedAt": captured_at,
            "view": config.view.as_str(),
            "ranking": ranking.id(),
            "sourceCount": projection.source_count,
            "suppressedCount": projection.suppressed_count,
            "count": projection.cards.len(),
            "notificationEligibleIds": projection.notification_eligible_ids,
            "pullRequests": projection.cards,
        }))?
    );
    Ok(())
}

fn parse_config(args: impl Iterator<Item = String>) -> Result<Config> {
    let mut config = Config {
        database: "/data/review-radar.sqlite3".into(),
        capture_id: None,
        view: WorkspaceView::Tailored,
        ranking: "tailored".into(),
        state_database: "/data/review-radar-state.sqlite3".into(),
        record_attention: false,
    };
    let mut args = args.peekable();
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| anyhow!("missing value for {flag}"))?;
        match flag.as_str() {
            "--database" => config.database = value.into(),
            "--capture-id" => {
                config.capture_id = Some(value.parse().context("invalid capture ID")?)
            }
            "--view" => {
                config.view = WorkspaceView::parse(&value).ok_or_else(|| {
                    anyhow!(
                        "invalid view {value:?}; use tailored, action, my-prs, following, or recent"
                    )
                })?
            }
            "--ranking" => config.ranking = value,
            "--state-database" => config.state_database = value.into(),
            "--record-attention" => {
                config.record_attention = match value.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => bail!("--record-attention must be true or false"),
                }
            }
            _ => bail!("unknown argument: {flag}"),
        }
    }
    Ok(config)
}

struct StateProjection {
    cards: Vec<PullRequestCard>,
    source_count: usize,
    suppressed_count: usize,
    notification_eligible_ids: Vec<String>,
}

fn apply_local_state(
    cards: Vec<PullRequestCard>,
    state: &StateStore,
    record_attention: bool,
    now: chrono::DateTime<Utc>,
) -> Result<StateProjection> {
    let source_count = cards.len();
    let mut visible = Vec::new();
    let mut notification_eligible_ids = Vec::new();
    for card in cards {
        if record_attention
            && state
                .observe_attention(
                    &card.id,
                    card.attention_required,
                    &card.current_fingerprint,
                    now,
                )?
                .notification_due
        {
            notification_eligible_ids.push(card.id.clone());
        }
        if !state.is_suppressed(&card.id, &card.current_fingerprint, now)? {
            visible.push(card);
        }
    }
    let suppressed_count = source_count - visible.len();
    Ok(StateProjection {
        cards: visible,
        source_count,
        suppressed_count,
        notification_eligible_ids,
    })
}

fn ranking(id: &str) -> Result<Box<dyn RankingStrategy>> {
    match id {
        "tailored" => Ok(Box::new(TailoredRanking)),
        "newest-activity" => Ok(Box::new(NewestActivityRanking)),
        _ => bail!("invalid ranking {id:?}; use tailored or newest-activity"),
    }
}

fn latest_capture_id(connection: &Connection) -> Result<i64> {
    connection
        .query_row(
            "SELECT id FROM captures ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .context("database contains no successful captures")
}

fn load_snapshot(connection: &Connection, capture_id: i64) -> Result<(String, Snapshot)> {
    let (captured_at, viewer_login): (String, String) = connection
        .query_row(
            "SELECT captured_at, viewer_login FROM captures WHERE id = ?",
            [capture_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .with_context(|| format!("capture {capture_id} does not exist"))?;
    let searches = load_searches(connection, capture_id)?;
    let pull_requests = load_pull_requests(connection, capture_id)?;
    let snapshot = Snapshot::from_json(&serde_json::to_string(&json!({
        "viewer": { "login": viewer_login },
        "searches": searches,
        "pullRequests": pull_requests,
    }))?)?;
    Ok((captured_at, snapshot))
}

fn load_searches(connection: &Connection, capture_id: i64) -> Result<Vec<Value>> {
    let mut statement = connection
        .prepare("SELECT category FROM searches WHERE capture_id = ? ORDER BY category")?;
    let categories = statement
        .query_map([capture_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut searches = Vec::new();
    for category in categories {
        let mut membership_statement = connection.prepare(
            "SELECT node_id FROM search_memberships
             WHERE capture_id = ? AND category = ? ORDER BY node_id",
        )?;
        let ids = membership_statement
            .query_map(params![capture_id, category], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        searches.push(json!({ "category": category, "ids": ids }));
    }
    Ok(searches)
}

fn load_pull_requests(connection: &Connection, capture_id: i64) -> Result<Vec<Value>> {
    let mut statement = connection
        .prepare("SELECT payload FROM pull_requests WHERE capture_id = ? ORDER BY node_id")?;
    let payloads = statement
        .query_map([capture_id], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    payloads
        .into_iter()
        .map(|payload| serde_json::from_str(&payload).context("invalid stored PR payload"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_supported_views() {
        let config = parse_config(
            [
                "--view".into(),
                "action".into(),
                "--capture-id".into(),
                "4".into(),
                "--ranking".into(),
                "newest-activity".into(),
                "--record-attention".into(),
                "true".into(),
            ]
            .into_iter(),
        )
        .unwrap();
        assert_eq!(config.view, WorkspaceView::Action);
        assert_eq!(config.capture_id, Some(4));
        assert_eq!(config.ranking, "newest-activity");
        assert!(config.record_attention);
        assert!(parse_config(["--view".into(), "other".into()].into_iter()).is_err());
        assert!(ranking("other").is_err());
    }

    #[test]
    fn local_state_suppresses_cards_and_observes_attention_on_request() {
        let cards = vec![
            card("pr-1", true, "event-1"),
            card("pr-2", false, "event-2"),
        ];
        let state = StateStore::in_memory().unwrap();
        let now = Utc::now();
        state.acknowledge("pr-1", "event-1", now).unwrap();
        let projection = apply_local_state(cards, &state, true, now).unwrap();

        assert_eq!(projection.source_count, 2);
        assert_eq!(projection.suppressed_count, 1);
        assert_eq!(projection.cards.len(), 1);
        assert!(projection.notification_eligible_ids.is_empty());
    }

    fn card(id: &str, attention_required: bool, current_fingerprint: &str) -> PullRequestCard {
        PullRequestCard {
            id: id.into(),
            repository: "example/repository-001".into(),
            number: 1,
            title: "Example pull request 001".into(),
            url: "https://github.com/example/repository-001/pull/1".into(),
            updated_at: "2026-09-21T12:00:00Z".into(),
            lifecycle: review_radar_domain::Lifecycle::Open,
            memberships: vec!["authored".into()],
            priority: review_radar_domain::Priority::AuthoredAction,
            relationship: review_radar_domain::Relationship::Authored,
            action: review_radar_domain::Action::ChangesRequested,
            action_label: "Changes requested",
            attention_required,
            explanation: review_radar_domain::attention::Explanation {
                heading: "Why this needs your attention",
                reasons: Vec::new(),
                health: review_radar_domain::attention::Health {
                    review_decision: None,
                    checks: None,
                    mergeable: "UNKNOWN".into(),
                    merge_state_status: "UNKNOWN".into(),
                    is_draft: false,
                },
            },
            current_fingerprint: current_fingerprint.into(),
            events: Vec::new(),
        }
    }
}
