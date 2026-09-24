use std::{collections::BTreeMap, env, path::PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
#[cfg(test)]
use review_radar_domain::PullRequestCard;
use review_radar_domain::{
    friction::{Coverage, HistoryEvent, HistoryEventKind, ReviewHistory},
    handoff::{self, Event as HandoffEvent, EventKind as HandoffEventKind},
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
    let handoff_histories = if include_review_history {
        normalize_handoff_histories(&pull_requests, &captured_at)?
    } else {
        BTreeMap::new()
    };
    let snapshot = Snapshot::from_json(&serde_json::to_string(&json!({
        "capturedAt": captured_at.clone(),
        "viewer": { "login": viewer_login },
        "searches": searches,
        "pullRequests": pull_requests,
        "reviewHistories": review_histories,
        "handoffHistories": handoff_histories,
    }))?)?;
    Ok((captured_at, snapshot))
}

/// Normalize the bounded request/review/head-change evidence into explicit
/// response episodes. Exact response durations are available only when both
/// request and review connections are complete and the reviewer is an exact user.
fn normalize_handoff_histories(
    pull_requests: &[Value],
    captured_at: &str,
) -> Result<BTreeMap<String, handoff::History>> {
    let observed_until = unix_seconds(captured_at)?;
    let mut histories = BTreeMap::new();
    for pull_request in pull_requests {
        let Some(id) = pull_request.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(started_at) = pull_request
            .get("createdAt")
            .and_then(Value::as_str)
            .and_then(|at| unix_seconds(at).ok())
        else {
            continue;
        };
        let (Some(timeline), Some(reviews)) = (
            pull_request.get("handoffItems"),
            pull_request.get("reviews"),
        ) else {
            continue;
        };
        let mut complete = connection_complete(timeline).unwrap_or(false)
            && connection_nodes(timeline).is_some()
            && connection_complete(reviews).unwrap_or(false)
            && connection_nodes(reviews).is_some();
        let mut events = Vec::new();
        let mut valid = started_at <= observed_until;
        let pull_request_author = pull_request.pointer("author.login").and_then(Value::as_str);

        for node in connection_nodes(timeline).into_iter().flatten() {
            let typename = node.get("__typename").and_then(Value::as_str);
            let kind = match typename {
                Some("ReviewRequestedEvent") => Some(HandoffEventKind::Requested(
                    requested_reviewer(node.get("requestedReviewer")),
                )),
                Some("ReviewRequestRemovedEvent") => Some(HandoffEventKind::Removed(
                    requested_reviewer(node.get("requestedReviewer")),
                )),
                Some("HeadRefForcePushedEvent") => Some(HandoffEventKind::ForcePushed {
                    before_oid: node
                        .get("beforeCommit")
                        .and_then(|value| value.get("oid"))
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    after_oid: node
                        .get("afterCommit")
                        .and_then(|value| value.get("oid"))
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                }),
                _ => {
                    valid = false;
                    None
                }
            };
            if let Some(kind) = kind {
                let Some(event) = handoff_source_event(node, kind) else {
                    valid = false;
                    continue;
                };
                events.push(event);
            }
        }

        for node in connection_nodes(reviews).into_iter().flatten() {
            let Some(state) = node.get("state").and_then(Value::as_str) else {
                valid = false;
                continue;
            };
            if state == "PENDING" {
                continue;
            }
            let author = node.get("author").and_then(Value::as_object);
            let (Some(login), Some(typename)) = (
                author
                    .and_then(|actor| actor.get("login"))
                    .and_then(Value::as_str),
                author
                    .and_then(|actor| actor.get("__typename"))
                    .and_then(Value::as_str),
            ) else {
                valid = false;
                continue;
            };
            if pull_request_author.is_some_and(|author| author.eq_ignore_ascii_case(login))
                || typename == "Bot"
            {
                continue;
            }
            let timestamp = node.get("submittedAt").and_then(Value::as_str);
            let Some(at) = timestamp.and_then(|value| unix_seconds(value).ok()) else {
                valid = false;
                continue;
            };
            events.push(HandoffEvent {
                id: node
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                at,
                kind: HandoffEventKind::Review {
                    reviewer: Some(login.to_owned()),
                },
            });
        }

        if !valid {
            complete = false;
        }
        let mut history = handoff::History {
            policy_version: handoff::POLICY_VERSION.into(),
            coverage: if complete {
                Coverage::Complete
            } else {
                Coverage::Partial
            },
            started_at,
            observed_until,
            episodes: Vec::new(),
            force_pushes: Vec::new(),
            limitations: Vec::new(),
        };
        handoff::complete(&mut history, events);
        histories.insert(id.to_owned(), history);
    }
    Ok(histories)
}

fn handoff_source_event(node: &Value, kind: HandoffEventKind) -> Option<HandoffEvent> {
    Some(HandoffEvent {
        id: node.get("id")?.as_str()?.to_owned(),
        at: unix_seconds(node.get("createdAt")?.as_str()?).ok()?,
        kind,
    })
}

fn requested_reviewer(value: Option<&Value>) -> Option<handoff::Reviewer> {
    let reviewer = value?;
    let kind = match reviewer.get("__typename").and_then(Value::as_str) {
        Some("User") => handoff::ReviewerKind::User,
        Some("Team") => handoff::ReviewerKind::Team,
        Some("Mannequin") => handoff::ReviewerKind::Mannequin,
        _ => handoff::ReviewerKind::Unknown,
    };
    let identifier = match kind {
        handoff::ReviewerKind::Team => reviewer.get("slug"),
        _ => reviewer.get("login"),
    }
    .and_then(Value::as_str)
    .map(str::to_owned);
    Some(handoff::Reviewer { kind, identifier })
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
                { "__typename": "ReadyForReviewEvent", "id": "ready-2", "createdAt": "2026-09-05T00:00:00Z" },
            ] },
            "handoffItems": { "pageInfo": { "hasPreviousPage": false }, "nodes": [
                { "__typename": "ReviewRequestedEvent", "id": "request-1", "createdAt": "2026-09-05T12:00:00Z", "requestedReviewer": { "__typename": "User", "login": "reviewer-001" } },
                { "__typename": "ReviewRequestRemovedEvent", "id": "request-removed-1", "createdAt": "2026-09-06T01:00:00Z", "requestedReviewer": { "__typename": "Team", "slug": "team-001" } },
                { "__typename": "HeadRefForcePushedEvent", "id": "push-1", "createdAt": "2026-09-06T02:00:00Z", "beforeCommit": { "oid": "old-head" }, "afterCommit": { "oid": "new-head" } }
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
    fn normalizes_request_response_episodes_and_requires_complete_connections() {
        let pull_request = json!({
            "id": "pr-episode", "createdAt": "2026-09-01T00:00:00Z",
            "author": { "login": "author-001" },
            "handoffItems": { "pageInfo": { "hasPreviousPage": false }, "nodes": [
                { "__typename": "ReviewRequestedEvent", "id": "request-001", "createdAt": "2026-09-02T00:00:00Z", "requestedReviewer": { "__typename": "User", "login": "reviewer-001" } },
                { "__typename": "ReviewRequestedEvent", "id": "request-team", "createdAt": "2026-09-02T01:00:00Z", "requestedReviewer": { "__typename": "Team", "slug": "team-001" } },
                { "__typename": "ReviewRequestRemovedEvent", "id": "removed-team", "createdAt": "2026-09-04T00:00:00Z", "requestedReviewer": { "__typename": "Team", "slug": "team-001" } },
                { "__typename": "HeadRefForcePushedEvent", "id": "push-001", "createdAt": "2026-09-05T00:00:00Z", "beforeCommit": { "oid": "head-a" }, "afterCommit": { "oid": "head-b" } }
            ] },
            "reviews": { "pageInfo": { "hasPreviousPage": false }, "nodes": [
                { "id": "review-001", "author": { "login": "reviewer-001", "__typename": "User" }, "state": "COMMENTED", "submittedAt": "2026-09-03T00:00:00Z" },
                { "id": "self-review", "author": { "login": "author-001", "__typename": "User" }, "state": "APPROVED", "submittedAt": "2026-09-03T01:00:00Z" },
                { "id": "bot-review", "author": { "login": "bot-001", "__typename": "Bot" }, "state": "APPROVED", "submittedAt": "2026-09-03T02:00:00Z" }
            ] }
        });
        let histories = normalize_handoff_histories(
            std::slice::from_ref(&pull_request),
            "2026-09-06T00:00:00Z",
        )
        .unwrap();
        let history = &histories["pr-episode"];
        assert_eq!(history.coverage, Coverage::Complete);
        assert_eq!(history.episodes.len(), 2);
        assert_eq!(history.episodes[0].outcome, handoff::Outcome::Reviewed);
        assert_eq!(history.episodes[0].response_seconds, Some(86_400));
        assert_eq!(
            history.episodes[0].resolution_event_id.as_deref(),
            Some("review-001")
        );
        assert_eq!(history.episodes[1].outcome, handoff::Outcome::Removed);
        assert_eq!(history.episodes[1].response_seconds, None);
        assert_eq!(history.force_pushes.len(), 1);
        assert_eq!(
            history.force_pushes[0].before_oid.as_deref(),
            Some("head-a")
        );
        assert_eq!(history.force_pushes[0].after_oid.as_deref(), Some("head-b"));

        let mut partial = pull_request;
        partial["reviews"]["pageInfo"]["hasPreviousPage"] = json!(true);
        let partial_history =
            normalize_handoff_histories(&[partial], "2026-09-06T00:00:00Z").unwrap();
        assert_eq!(partial_history["pr-episode"].coverage, Coverage::Partial);
        assert_eq!(
            partial_history["pr-episode"].episodes[0].outcome,
            handoff::Outcome::Inconclusive
        );
        assert_eq!(
            partial_history["pr-episode"].episodes[0].response_seconds,
            None
        );
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
            handoff_history: None,
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
