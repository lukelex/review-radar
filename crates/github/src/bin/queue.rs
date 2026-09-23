use std::{collections::BTreeMap, env, path::PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
#[cfg(test)]
use review_radar_domain::PullRequestCard;
use review_radar_domain::{
    friction::{Coverage, HistoryEvent, HistoryEventKind, ReviewHistory},
    RankingStrategy, Snapshot, WorkspaceView,
};
use review_radar_github::projection::apply_local_state;
use review_radar_state::StateStore;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde_json::{json, Value};
use tracing::{info, info_span};
use tracing_subscriber::EnvFilter;

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
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("review_radar_queue=info")),
        )
        .with_target(false)
        .init();
    let _span = info_span!("queue.projection").entered();
    let config = parse_config(env::args().skip(1))?;
    let ranking = ranking(&config.ranking)?;
    let connection = Connection::open_with_flags(
        &config.database,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )
    .with_context(|| format!("cannot open {}", config.database.display()))?;
    let capture_id = config.capture_id.unwrap_or(latest_capture_id(&connection)?);
    let (captured_at, snapshot) = load_snapshot(&connection, capture_id, true)?;
    let predecessor = load_predecessor_snapshot(&connection, capture_id)?;
    let state = StateStore::open(&config.state_database)
        .with_context(|| format!("cannot open {}", config.state_database.display()))?;
    let retained_feedback = state.outstanding_feedback()?;
    let cards = snapshot.view_with_ranking_since_and_feedback(
        config.view,
        ranking.as_ref(),
        predecessor.as_ref(),
        &retained_feedback,
    );
    for card in &cards {
        if !card.feedback_fingerprints.is_empty() {
            state.record_feedback(
                card.id.as_str(),
                &card
                    .feedback_fingerprints
                    .iter()
                    .map(|value| value.as_str().to_owned())
                    .collect::<Vec<_>>(),
                Utc::now(),
            )?;
        }
    }
    let projection = apply_local_state(cards, &state, config.record_attention, Utc::now())?;
    info!(
        capture_id,
        source_count = projection.source_count,
        visible_count = projection.cards.len(),
        suppressed_count = projection.suppressed_count,
        "projection completed"
    );
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

fn ranking(id: &str) -> Result<Box<dyn RankingStrategy>> {
    review_radar_domain::ranking::by_id(id).ok_or_else(|| {
        anyhow!("invalid ranking {id:?}; use tailored, newest-activity, or highest-friction")
    })
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

fn load_snapshot(
    connection: &Connection,
    capture_id: i64,
    include_review_history: bool,
) -> Result<(String, Snapshot)> {
    let (captured_at, viewer_login): (String, String) = connection
        .query_row(
            "SELECT captured_at, viewer_login FROM captures WHERE id = ?",
            [capture_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .with_context(|| format!("capture {capture_id} does not exist"))?;
    let searches = load_searches(connection, capture_id)?;
    let pull_requests = load_pull_requests(connection, capture_id)?;
    let review_histories = if include_review_history {
        normalize_review_histories(&pull_requests, &viewer_login, &captured_at)?
    } else {
        BTreeMap::new()
    };
    let snapshot = Snapshot::from_json(&serde_json::to_string(&json!({
        "capturedAt": captured_at.clone(),
        "viewer": { "login": viewer_login },
        "searches": searches,
        "pullRequests": pull_requests,
        "reviewHistories": review_histories,
    }))?)?;
    Ok((captured_at, snapshot))
}

fn load_predecessor_snapshot(connection: &Connection, capture_id: i64) -> Result<Option<Snapshot>> {
    let predecessor_id = connection
        .query_row(
            "SELECT id FROM captures WHERE id < ? ORDER BY id DESC LIMIT 1",
            [capture_id],
            |row| row.get(0),
        )
        .optional()?;
    predecessor_id
        .map(|id| load_snapshot(connection, id, false).map(|(_, snapshot)| snapshot))
        .transpose()
}

/// Convert bounded collector evidence into the domain history contract. Missing
/// fields and pagination become partial coverage; commit totals are never used as
/// revision churn, so the resulting assessment remains Limited history for now.
fn normalize_review_histories(
    pull_requests: &[Value],
    viewer_login: &str,
    captured_at: &str,
) -> Result<BTreeMap<String, ReviewHistory>> {
    let observed_until = unix_seconds(captured_at)?;
    let mut histories = BTreeMap::new();
    for pull_request in pull_requests {
        let Some((id, history)) =
            normalize_review_history(pull_request, viewer_login, observed_until)
        else {
            continue;
        };
        histories.insert(id, history);
    }
    Ok(histories)
}

fn normalize_review_history(
    pull_request: &Value,
    viewer_login: &str,
    observed_until: u64,
) -> Option<(String, ReviewHistory)> {
    let id = pull_request.get("id")?.as_str()?.to_owned();
    let started_at = unix_seconds(pull_request.get("createdAt")?.as_str()?).ok()?;
    let current_draft = pull_request.get("isDraft")?.as_bool()?;
    let timeline = pull_request.get("timelineItems")?;
    let reviews = pull_request.get("reviews")?;
    let commits = pull_request.get("commits")?;
    let mut complete = connection_complete(timeline)?
        && connection_complete(reviews)?
        && connection_complete(commits)?;
    let mut events = Vec::new();
    let mut valid = started_at <= observed_until;

    for node in connection_nodes(timeline)? {
        let (kind, timestamp) = match node.get("__typename").and_then(Value::as_str)? {
            "ReadyForReviewEvent" => (HistoryEventKind::Ready, node.get("createdAt")),
            "ConvertToDraftEvent" => (HistoryEventKind::Draft, node.get("createdAt")),
            "ClosedEvent" => (HistoryEventKind::Closed, node.get("createdAt")),
            "ReopenedEvent" => (HistoryEventKind::Reopened, node.get("createdAt")),
            _ => {
                valid = false;
                continue;
            }
        };
        let Some(event) = source_event(
            format!(
                "timeline:{}",
                node.get("id").and_then(Value::as_str).unwrap_or_default()
            ),
            timestamp.and_then(Value::as_str),
            kind,
        ) else {
            valid = false;
            continue;
        };
        events.push(event);
    }
    for node in connection_nodes(reviews)? {
        let author = node
            .get("author")
            .and_then(Value::as_object)
            .and_then(|author| {
                author
                    .get("login")
                    .and_then(Value::as_str)
                    .zip(author.get("__typename").and_then(Value::as_str))
            });
        let substantive = node.get("state").and_then(Value::as_str) != Some("PENDING");
        let Some((login, typename)) = author else {
            continue;
        };
        if !substantive || login == viewer_login || typename == "Bot" {
            continue;
        }
        let Some(event) = source_event(
            format!(
                "review:{}",
                node.get("id").and_then(Value::as_str).unwrap_or_default()
            ),
            node.get("submittedAt")
                .or_else(|| node.get("updatedAt"))
                .and_then(Value::as_str),
            HistoryEventKind::Review,
        ) else {
            valid = false;
            continue;
        };
        events.push(event);
    }
    for node in connection_nodes(commits)? {
        let commit = node.get("commit")?;
        let changed_lines = parent_diff_lines(commit);
        let Some(event) = source_event(
            format!(
                "commit:{}",
                commit
                    .get("oid")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            ),
            commit.get("committedDate").and_then(Value::as_str),
            HistoryEventKind::Revision { changed_lines },
        ) else {
            valid = false;
            continue;
        };
        events.push(event);
    }
    events.sort_by(|left, right| left.at.cmp(&right.at).then_with(|| left.id.cmp(&right.id)));
    let initially_draft = events
        .iter()
        .find_map(|event| match event.kind {
            HistoryEventKind::Ready => Some(true),
            HistoryEventKind::Draft => Some(false),
            _ => None,
        })
        .unwrap_or(current_draft);
    if events.iter().any(|event| event.id.ends_with(':')) {
        valid = false;
    }
    if !valid {
        complete = false;
    }
    let initial_review_diff_lines = initial_review_change_volume(&events);
    Some((
        id,
        ReviewHistory {
            coverage: if complete {
                Coverage::Complete
            } else {
                Coverage::Partial
            },
            started_at,
            observed_until,
            initially_draft,
            initial_review_diff_lines,
            events,
        },
    ))
}

/// Sum complete first-parent commit deltas through the first substantive review.
/// This is an auditable initial-review *change-volume* baseline, not a net PR
/// diff: repeated pre-review edits remain visible rather than being cancelled.
fn initial_review_change_volume(events: &[HistoryEvent]) -> Option<u64> {
    let first_review_at = events
        .iter()
        .find(|event| matches!(event.kind, HistoryEventKind::Review))?
        .at;
    let revisions = events
        .iter()
        .filter(|event| {
            event.at <= first_review_at && matches!(event.kind, HistoryEventKind::Revision { .. })
        })
        .collect::<Vec<_>>();
    (!revisions.is_empty()).then_some(())?;
    revisions
        .into_iter()
        .try_fold(0_u64, |total, event| match event.kind {
            HistoryEventKind::Revision {
                changed_lines: Some(lines),
            } => total.checked_add(lines),
            _ => None,
        })
}

/// GitHub's Commit additions/deletions are calculated against the commit's
/// first parent. Missing parent or counters mean this is not a reproducible
/// parent diff and must remain unknown rather than be counted as zero.
fn parent_diff_lines(commit: &Value) -> Option<u64> {
    let parent = commit.get("parents")?.get("nodes")?.as_array()?.first()?;
    parent.get("oid")?.as_str()?;
    let additions = commit.get("additions")?.as_u64()?;
    let deletions = commit.get("deletions")?.as_u64()?;
    additions.checked_add(deletions)
}

fn connection_nodes(connection: &Value) -> Option<&Vec<Value>> {
    connection.get("nodes")?.as_array()
}

fn connection_complete(connection: &Value) -> Option<bool> {
    Some(
        !connection
            .get("pageInfo")?
            .get("hasPreviousPage")?
            .as_bool()?,
    )
}

fn source_event(
    id: String,
    timestamp: Option<&str>,
    kind: HistoryEventKind,
) -> Option<HistoryEvent> {
    (!id.ends_with(':')).then_some(HistoryEvent {
        id,
        at: unix_seconds(timestamp?).ok()?,
        kind,
    })
}

fn unix_seconds(value: &str) -> Result<u64> {
    let seconds = DateTime::parse_from_rfc3339(value)?.timestamp();
    u64::try_from(seconds).context("timestamp before Unix epoch")
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
    let mut statement = connection.prepare(
        "SELECT node_id, payload FROM pull_requests WHERE capture_id = ? ORDER BY node_id",
    )?;
    let payloads = statement
        .query_map([capture_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    payloads
        .into_iter()
        .map(|(node_id, payload)| {
            let mut payload: Value =
                serde_json::from_str(&payload).context("invalid stored PR payload")?;
            let object = payload
                .as_object_mut()
                .context("stored PR payload is not an object")?;
            // The database primary key is canonical. Restore it for captures
            // written by older collectors that did not retain `id` in JSON.
            object.insert("id".into(), Value::String(node_id));
            Ok(payload)
        })
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
        assert_eq!(
            ranking("highest-friction").unwrap().id(),
            "highest-friction"
        );
    }

    #[test]
    fn restores_id_from_database_key_for_legacy_payloads() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE pull_requests (capture_id INTEGER, node_id TEXT, payload TEXT);",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO pull_requests VALUES (1, 'PR_1', '{\"title\":\"Legacy PR\"}')",
                [],
            )
            .unwrap();

        let pull_requests = load_pull_requests(&connection, 1).unwrap();

        assert_eq!(pull_requests[0]["id"], "PR_1");
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

    #[test]
    fn projection_notifications_are_baselined_and_deduplicated() {
        let state = StateStore::in_memory().unwrap();
        let now = Utc::now();

        let first =
            apply_local_state(vec![card("pr-1", true, "event-1")], &state, true, now).unwrap();
        assert!(first.notification_eligible_ids.is_empty());

        let unchanged =
            apply_local_state(vec![card("pr-1", true, "event-1")], &state, true, now).unwrap();
        assert!(unchanged.notification_eligible_ids.is_empty());

        let recovered =
            apply_local_state(vec![card("pr-1", false, "healthy")], &state, true, now).unwrap();
        assert!(recovered.notification_eligible_ids.is_empty());

        let reactivated =
            apply_local_state(vec![card("pr-1", true, "event-2")], &state, true, now).unwrap();
        assert_eq!(reactivated.notification_eligible_ids, vec!["pr-1"]);

        let duplicate =
            apply_local_state(vec![card("pr-1", true, "event-2")], &state, true, now).unwrap();
        assert!(duplicate.notification_eligible_ids.is_empty());
    }

    #[test]
    fn normalizes_complete_collector_history_without_claiming_churn() {
        let payload = json!({
            "id": "pr-001", "createdAt": "2026-09-01T00:00:00Z", "isDraft": false,
            "timelineItems": { "pageInfo": { "hasPreviousPage": false }, "nodes": [
                { "__typename": "ReadyForReviewEvent", "id": "ready-1", "createdAt": "2026-09-03T00:00:00Z" },
                { "__typename": "ConvertToDraftEvent", "id": "draft-1", "createdAt": "2026-09-04T00:00:00Z" },
                { "__typename": "ReadyForReviewEvent", "id": "ready-2", "createdAt": "2026-09-05T00:00:00Z" }
            ] },
            "reviews": { "pageInfo": { "hasPreviousPage": false }, "nodes": [
                { "id": "review-1", "author": { "login": "reviewer-001", "__typename": "User" }, "state": "COMMENTED", "submittedAt": "2026-09-06T00:00:00Z", "updatedAt": "2026-09-06T00:00:00Z" }
            ] },
            "commits": { "pageInfo": { "hasPreviousPage": false }, "nodes": [
                { "commit": { "oid": "commit-1", "committedDate": "2026-09-02T00:00:00Z", "additions": 40, "deletions": 10, "parents": { "nodes": [{ "oid": "base" }] } } },
                { "commit": { "oid": "commit-2", "committedDate": "2026-09-07T00:00:00Z", "additions": 30, "deletions": 20, "parents": { "nodes": [{ "oid": "commit-1" }] } } }
            ] }
        });
        let (_, history) = normalize_review_history(
            &payload,
            "viewer-001",
            unix_seconds("2026-09-08T00:00:00Z").unwrap(),
        )
        .unwrap();
        assert_eq!(history.coverage, Coverage::Complete);
        assert!(history.initially_draft);
        assert_eq!(history.initial_review_diff_lines, Some(50));
        assert!(history
            .events
            .iter()
            .any(|event| matches!(event.kind, HistoryEventKind::Review)));
        assert!(history.events.iter().any(|event| matches!(
            event.kind,
            HistoryEventKind::Revision {
                changed_lines: Some(50)
            }
        )));
        let assessment = review_radar_domain::friction::assess(
            Some(&history),
            review_radar_domain::Lifecycle::Open,
            false,
        );
        assert_eq!(
            assessment.status,
            review_radar_domain::friction::AssessmentStatus::Assessed
        );
        assert_eq!(
            assessment.level,
            // One moderate duration signal plus one moderate rework signal
            // combines to a high overall assessment under review-friction-v1.
            Some(review_radar_domain::friction::Level::High)
        );
    }

    #[test]
    fn paginated_or_unattributed_evidence_stays_partial_and_does_not_add_review_events() {
        let payload = json!({
            "id": "pr-001", "createdAt": "2026-09-01T00:00:00Z", "isDraft": false,
            "timelineItems": { "pageInfo": { "hasPreviousPage": true }, "nodes": [] },
            "reviews": { "pageInfo": { "hasPreviousPage": false }, "nodes": [
                { "id": "self", "author": { "login": "viewer-001", "__typename": "User" }, "state": "COMMENTED", "submittedAt": "2026-09-02T00:00:00Z" },
                { "id": "bot", "author": { "login": "bot-001", "__typename": "Bot" }, "state": "COMMENTED", "submittedAt": "2026-09-02T00:00:00Z" },
                { "id": "pending", "author": { "login": "reviewer-001", "__typename": "User" }, "state": "PENDING", "updatedAt": "2026-09-02T00:00:00Z" },
                { "id": "unknown", "author": null, "state": "COMMENTED", "submittedAt": "2026-09-02T00:00:00Z" }
            ] },
            "commits": { "pageInfo": { "hasPreviousPage": false }, "nodes": [] }
        });
        let (_, history) = normalize_review_history(
            &payload,
            "viewer-001",
            unix_seconds("2026-09-08T00:00:00Z").unwrap(),
        )
        .unwrap();
        assert_eq!(history.coverage, Coverage::Partial);
        assert!(history.events.is_empty());
    }

    #[test]
    fn parent_diff_requires_a_parent_and_explicit_counters() {
        assert_eq!(
            parent_diff_lines(&json!({
                "parents": { "nodes": [{ "oid": "parent-1" }] }, "additions": 12, "deletions": 8
            })),
            Some(20)
        );
        assert_eq!(
            parent_diff_lines(&json!({ "additions": 12, "deletions": 8 })),
            None
        );
        assert_eq!(
            parent_diff_lines(&json!({
                "parents": { "nodes": [{ "oid": "parent-1" }] }, "additions": 12
            })),
            None
        );
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
            review_friction: review_radar_domain::friction::Assessment::unavailable(
                review_radar_domain::Lifecycle::Open,
            ),
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
            feedback_fingerprints: Vec::new(),
            events: Vec::new(),
        }
    }
}
