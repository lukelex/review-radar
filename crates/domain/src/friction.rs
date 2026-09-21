//! Experimental, evidence-backed review friction. Snapshot age and comment
//! counts are deliberately not substitutes for complete review history.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::Lifecycle;

pub const POLICY_VERSION: &str = "review-friction-v1";
const MODERATE_REVIEW_SECONDS: u64 = 3 * 86_400;
const HIGH_REVIEW_SECONDS: u64 = 7 * 86_400;
const MODERATE_REWORK_BASIS_POINTS: u64 = 10_000;
const HIGH_REWORK_BASIS_POINTS: u64 = 50_000;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewHistory {
    pub coverage: Coverage,
    /// UTC epoch seconds. Complete coverage starts at PR creation.
    pub started_at: u64,
    pub observed_until: u64,
    pub initially_draft: bool,
    pub initial_review_diff_lines: Option<u64>,
    pub events: Vec<HistoryEvent>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Coverage {
    Complete,
    Partial,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HistoryEvent {
    pub id: String,
    pub at: u64,
    #[serde(flatten)]
    pub kind: HistoryEventKind,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum HistoryEventKind {
    Ready,
    Draft,
    Review,
    Revision {
        #[serde(rename = "changedLines")]
        changed_lines: Option<u64>,
    },
    Closed,
    Reopened,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "kebab-case")]
pub enum Level {
    Low,
    Moderate,
    High,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum AssessmentStatus {
    Assessed,
    LimitedHistory,
    NotAssessed,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Assessment {
    pub policy_version: &'static str,
    pub status: AssessmentStatus,
    pub level: Option<Level>,
    pub historical: bool,
    pub coverage: Option<Coverage>,
    pub started_at: Option<u64>,
    pub observed_until: Option<u64>,
    pub review_rounds: Option<u64>,
    pub review_seconds: Option<u64>,
    pub rework_lines: Option<u64>,
    pub initial_review_diff_lines: Option<u64>,
    pub contributors: Vec<Contributor>,
    pub limitations: Vec<&'static str>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Contributor {
    pub signal: &'static str,
    pub value: u64,
    pub unit: &'static str,
    pub level: Level,
    pub evidence_ids: Vec<String>,
}

impl Assessment {
    pub fn unavailable(lifecycle: Lifecycle) -> Self {
        Self {
            policy_version: POLICY_VERSION,
            status: AssessmentStatus::LimitedHistory,
            level: None,
            historical: lifecycle != Lifecycle::Open,
            coverage: None,
            started_at: None,
            observed_until: None,
            review_rounds: None,
            review_seconds: None,
            rework_lines: None,
            initial_review_diff_lines: None,
            contributors: Vec::new(),
            limitations: vec!["missing-review-history"],
        }
    }
}

/// Derive from normalized evidence only. Incomplete history never receives a
/// level, and assessment changes have no effect on meaningful-event fingerprints.
pub fn assess(history: Option<&ReviewHistory>, lifecycle: Lifecycle, is_draft: bool) -> Assessment {
    let mut result = Assessment::unavailable(lifecycle);
    let Some(history) = history else {
        return result;
    };
    result.coverage = Some(history.coverage);
    result.started_at = Some(history.started_at);
    result.observed_until = Some(history.observed_until);
    result.initial_review_diff_lines = history.initial_review_diff_lines;
    result.limitations.clear();
    if history.coverage == Coverage::Partial {
        result.limitations.push("partial-review-history");
        return result;
    }
    let mut ids = BTreeSet::new();
    let mut cursor = history.started_at;
    let mut draft = history.initially_draft;
    let mut closed = false;
    let mut ever_ready = !draft;
    let mut seconds = 0;
    let mut rounds = 0;
    let mut revised = false;
    let mut rework = Some(0_u64);
    let mut round_ids = Vec::new();
    let mut duration_ids = Vec::new();
    let mut rework_ids = Vec::new();
    if cursor > history.observed_until {
        result.limitations.push("invalid-history-bounds");
        return result;
    }
    for event in &history.events {
        if event.id.is_empty()
            || !ids.insert(&event.id)
            || event.at < cursor
            || event.at > history.observed_until
        {
            result.limitations.push("invalid-history-events");
            return result;
        }
        if matches!(event.kind, HistoryEventKind::Reopened) {
            result.limitations.push("unsupported-reopened-history");
            return result;
        }
        if closed {
            result.limitations.push("events-after-completion");
            return result;
        }
        if !draft {
            seconds += event.at - cursor;
        }
        cursor = event.at;
        match event.kind {
            HistoryEventKind::Ready => {
                if !draft {
                    result.limitations.push("invalid-ready-transition");
                    return result;
                }
                draft = false;
                ever_ready = true;
                duration_ids.push(event.id.clone());
            }
            HistoryEventKind::Draft => {
                if draft {
                    result.limitations.push("invalid-draft-transition");
                    return result;
                }
                draft = true;
                duration_ids.push(event.id.clone());
            }
            HistoryEventKind::Review if !draft => {
                if rounds == 0 || revised {
                    rounds += 1;
                    round_ids.push(event.id.clone());
                    revised = false;
                }
            }
            HistoryEventKind::Revision { changed_lines } if !draft && rounds > 0 => {
                revised = true;
                round_ids.push(event.id.clone());
                rework_ids.push(event.id.clone());
                rework = rework
                    .and_then(|total| changed_lines.and_then(|lines| total.checked_add(lines)));
            }
            HistoryEventKind::Closed => {
                closed = true;
                duration_ids.push(event.id.clone());
            }
            _ => {}
        }
    }
    if closed != (lifecycle != Lifecycle::Open) || draft != is_draft {
        result.limitations.push("history-snapshot-mismatch");
        return result;
    }
    if !closed && !draft {
        seconds += history.observed_until - cursor;
    }
    result.review_rounds = Some(rounds);
    result.review_seconds = Some(seconds);
    result.rework_lines = rework;
    if !ever_ready {
        result.status = AssessmentStatus::NotAssessed;
        result.limitations.push("review-not-started");
        return result;
    }
    let (Some(rework), Some(baseline)) = (rework, history.initial_review_diff_lines) else {
        result.limitations.push("unknown-code-churn");
        return result;
    };
    // Basis points retain useful precision without floating-point thresholds.
    let ratio = ((u128::from(rework) * 10_000) / u128::from(baseline.max(1)))
        .min(u128::from(u64::MAX)) as u64;
    result.contributors = vec![
        Contributor {
            signal: "review-rounds",
            value: rounds,
            unit: "rounds",
            level: band(rounds, 2, 4),
            evidence_ids: round_ids,
        },
        Contributor {
            signal: "time-in-review",
            value: seconds,
            unit: "seconds",
            level: band(seconds, MODERATE_REVIEW_SECONDS, HIGH_REVIEW_SECONDS),
            evidence_ids: duration_ids,
        },
        Contributor {
            signal: "code-rework",
            value: ratio,
            unit: "basis-points-of-initial-diff",
            level: band(
                ratio,
                MODERATE_REWORK_BASIS_POINTS,
                HIGH_REWORK_BASIS_POINTS,
            ),
            evidence_ids: rework_ids,
        },
    ];
    let high = result.contributors.iter().any(|c| c.level == Level::High);
    let moderate = result
        .contributors
        .iter()
        .filter(|c| c.level == Level::Moderate)
        .count();
    result.level = Some(if high || moderate >= 2 {
        Level::High
    } else if moderate == 1 {
        Level::Moderate
    } else {
        Level::Low
    });
    result.status = AssessmentStatus::Assessed;
    result
}

fn band(value: u64, moderate: u64, high: u64) -> Level {
    if value >= high {
        Level::High
    } else if value >= moderate {
        Level::Moderate
    } else {
        Level::Low
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        name: String,
        lifecycle: String,
        is_draft: bool,
        history: Option<ReviewHistory>,
        status: AssessmentStatus,
        level: Option<Level>,
        rounds: Option<u64>,
        seconds: Option<u64>,
    }

    #[test]
    fn representative_journeys_have_explainable_assessments() {
        let cases: Vec<Case> = serde_json::from_str(include_str!(
            "../../../tests/fixtures/domain/review-histories.json"
        ))
        .unwrap();
        for case in cases {
            let result = assess(
                case.history.as_ref(),
                crate::lifecycle(&case.lifecycle),
                case.is_draft,
            );
            assert_eq!(result.status, case.status, "{}", case.name);
            assert_eq!(result.level, case.level, "{}", case.name);
            assert_eq!(result.review_rounds, case.rounds, "{}", case.name);
            assert_eq!(result.review_seconds, case.seconds, "{}", case.name);
            if result.status == AssessmentStatus::Assessed {
                assert_eq!(result.contributors.len(), 3, "{}", case.name);
                assert!(result.limitations.is_empty(), "{}", case.name);
            }
        }
    }

    #[test]
    fn completion_freezes_duration_and_invalid_or_unknown_evidence_is_not_low() {
        let mut history = ReviewHistory {
            coverage: Coverage::Complete,
            started_at: 0,
            observed_until: 100,
            initially_draft: false,
            initial_review_diff_lines: Some(100),
            events: vec![HistoryEvent {
                id: "closed-1".into(),
                at: 50,
                kind: HistoryEventKind::Closed,
            }],
        };
        let completed = assess(Some(&history), Lifecycle::Merged, false);
        history.observed_until = 100_000;
        assert_eq!(
            completed.review_seconds,
            assess(Some(&history), Lifecycle::Merged, false).review_seconds
        );
        assert!(completed.historical);
        assert_eq!(assess(Some(&history), Lifecycle::Open, false).level, None);
        history.initial_review_diff_lines = None;
        assert_eq!(assess(Some(&history), Lifecycle::Merged, false).level, None);
        history.events.push(history.events[0].clone());
        assert!(assess(Some(&history), Lifecycle::Merged, false)
            .limitations
            .contains(&"invalid-history-events"));
    }

    #[test]
    fn history_and_elapsed_time_change_friction_without_reactivating_attention() {
        let mut snapshot = crate::Snapshot::from_json(include_str!(
            "../../../tests/fixtures/github/github-snapshot.json"
        ))
        .unwrap();
        let id = snapshot
            .pull_requests
            .iter()
            .find(|pr| pr.state == "OPEN" && !pr.is_draft)
            .unwrap()
            .id
            .clone();
        let card = |snapshot: &crate::Snapshot| {
            snapshot
                .tailored_queue()
                .into_iter()
                .find(|card| card.id.as_str() == id.as_str())
                .unwrap()
        };
        let before = card(&snapshot);
        assert_eq!(
            before.review_friction.status,
            AssessmentStatus::LimitedHistory
        );
        snapshot.review_histories.insert(
            id.to_string(),
            ReviewHistory {
                coverage: Coverage::Complete,
                started_at: 0,
                observed_until: 86400,
                initially_draft: false,
                initial_review_diff_lines: Some(100),
                events: Vec::new(),
            },
        );
        let low = card(&snapshot);
        snapshot
            .review_histories
            .get_mut(id.as_str())
            .unwrap()
            .observed_until = 14 * 86400;
        let high = card(&snapshot);
        assert_eq!(low.review_friction.level, Some(Level::Low));
        assert_eq!(high.review_friction.level, Some(Level::High));
        assert_eq!(before.current_fingerprint, high.current_fingerprint);
        assert_eq!(before.attention_required, high.attention_required);
        assert_eq!(before.explanation, high.explanation);
        assert_eq!(before.events, high.events);
    }

    #[test]
    fn generalized_calibration_traversal_is_structurally_useful_and_identity_safe() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/domain/simplerqms-merged-calibration.json"
        ))
        .unwrap();
        let pull_requests = fixture["pullRequests"].as_array().unwrap();
        assert_eq!(pull_requests.len(), 100);
        assert!(pull_requests.iter().all(|pr| {
            pr["id"].as_str().is_some_and(|id| id.starts_with("pr-"))
                && pr["reviews"].as_array().is_some()
                && pr["commits"].as_array().is_some()
        }));
        let serialized = fixture.to_string();
        assert!(!serialized.contains("SimplerQMS"));
        assert!(!serialized.contains("github.com"));
    }
}
