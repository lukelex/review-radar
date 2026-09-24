//! Evidence-backed review-request response episodes. These measurements describe
//! elapsed handoff time, not reviewer effort or business value.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::friction::Coverage;

pub const POLICY_VERSION: &str = "handoff-v1";

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub policy_version: String,
    pub coverage: Coverage,
    pub started_at: u64,
    pub observed_until: u64,
    pub episodes: Vec<Episode>,
    pub force_pushes: Vec<HeadChange>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub request_event_id: String,
    pub reviewer: Option<Reviewer>,
    pub requested_at: u64,
    pub outcome: Outcome,
    pub resolved_at: Option<u64>,
    /// Exact wall-clock time between GitHub's request and matching submitted
    /// review. Suppressed for partial histories and non-user requests.
    pub response_seconds: Option<u64>,
    /// Exact elapsed time from the request to the capture cutoff for a still-open
    /// request. Suppressed for partial histories.
    pub pending_seconds: Option<u64>,
    pub resolution_event_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Reviewer {
    pub kind: ReviewerKind,
    /// GitHub login for users/mannequins or slug for teams, when present.
    pub identifier: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewerKind {
    User,
    Team,
    Mannequin,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    PendingAtCapture,
    Reviewed,
    Removed,
    Inconclusive,
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HeadChange {
    pub event_id: String,
    pub at: u64,
    pub before_oid: Option<String>,
    pub after_oid: Option<String>,
}

/// Pair only exact user requests with a review by that same user. Team requests
/// are retained but not assigned to a team member without an attribution source.
/// An incomplete connection cannot establish a final outcome or exact metric.
pub fn complete(history: &mut History, mut events: Vec<Event>) {
    let mut ids = BTreeSet::new();
    let mut valid = history.started_at <= history.observed_until;
    events.sort_by(|left, right| {
        left.at
            .cmp(&right.at)
            .then_with(|| event_order(&left.kind).cmp(&event_order(&right.kind)))
            .then_with(|| left.id.cmp(&right.id))
    });

    for event in &events {
        if event.id.is_empty()
            || !ids.insert(event.id.as_str())
            || event.at < history.started_at
            || event.at > history.observed_until
        {
            valid = false;
        }
    }

    let complete_coverage = history.coverage == Coverage::Complete && valid;
    if !valid {
        history.coverage = Coverage::Partial;
        history
            .limitations
            .push("invalid-handoff-evidence".to_owned());
    }
    if history.coverage == Coverage::Partial {
        history
            .limitations
            .push("partial-handoff-history".to_owned());
    }

    for event in events {
        match event.kind {
            EventKind::Requested(reviewer) => {
                let identified = reviewer.as_ref().is_some_and(|reviewer| {
                    reviewer.kind != ReviewerKind::Unknown && reviewer.identifier.is_some()
                });
                if !identified {
                    history
                        .limitations
                        .push("unattributed-review-request".to_owned());
                }
                let measurable = complete_coverage && identified;
                history.episodes.push(Episode {
                    request_event_id: event.id,
                    reviewer,
                    requested_at: event.at,
                    outcome: if measurable {
                        Outcome::PendingAtCapture
                    } else {
                        Outcome::Inconclusive
                    },
                    resolved_at: None,
                    response_seconds: None,
                    pending_seconds: measurable
                        .then(|| history.observed_until.checked_sub(event.at))
                        .flatten(),
                    resolution_event_id: None,
                });
            }
            EventKind::Removed(reviewer) => {
                if complete_coverage {
                    if let Some(episode) = history.episodes.iter_mut().rev().find(|episode| {
                        episode.outcome == Outcome::PendingAtCapture
                            && reviewer_matches(episode.reviewer.as_ref(), reviewer.as_ref())
                    }) {
                        episode.outcome = Outcome::Removed;
                        episode.resolved_at = Some(event.at);
                        episode.pending_seconds = None;
                        episode.resolution_event_id = Some(event.id);
                    }
                }
            }
            EventKind::Review { reviewer, .. } => {
                if complete_coverage {
                    if let Some(login) = reviewer.as_deref() {
                        if let Some(episode) = history.episodes.iter_mut().find(|episode| {
                            episode.outcome == Outcome::PendingAtCapture
                                && matches!(episode.reviewer.as_ref(), Some(requested)
                                    if requested.kind == ReviewerKind::User
                                        && requested.identifier.as_deref().is_some_and(|id| id.eq_ignore_ascii_case(login)))
                        }) {
                            episode.outcome = Outcome::Reviewed;
                            episode.resolved_at = Some(event.at);
                            episode.response_seconds = event.at.checked_sub(episode.requested_at);
                            episode.pending_seconds = None;
                            episode.resolution_event_id = Some(event.id);
                        }
                    }
                }
            }
            EventKind::ForcePushed {
                before_oid,
                after_oid,
            } => history.force_pushes.push(HeadChange {
                event_id: event.id,
                at: event.at,
                before_oid,
                after_oid,
            }),
        }
    }

    history.episodes.sort_by(|left, right| {
        left.requested_at
            .cmp(&right.requested_at)
            .then_with(|| left.request_event_id.cmp(&right.request_event_id))
    });
    history.force_pushes.sort_by(|left, right| {
        left.at
            .cmp(&right.at)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });
}

fn reviewer_matches(left: Option<&Reviewer>, right: Option<&Reviewer>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.kind == right.kind && left.identifier == right.identifier,
        _ => false,
    }
}

fn event_order(kind: &EventKind) -> u8 {
    match kind {
        EventKind::Requested(_) => 0,
        EventKind::Review { .. } => 1,
        EventKind::Removed(_) => 2,
        EventKind::ForcePushed { .. } => 3,
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub id: String,
    pub at: u64,
    pub kind: EventKind,
}

#[derive(Debug, Clone)]
pub enum EventKind {
    Requested(Option<Reviewer>),
    Removed(Option<Reviewer>),
    Review {
        reviewer: Option<String>,
    },
    ForcePushed {
        before_oid: Option<String>,
        after_oid: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(login: &str) -> Option<Reviewer> {
        Some(Reviewer {
            kind: ReviewerKind::User,
            identifier: Some(login.into()),
        })
    }

    fn history(coverage: Coverage) -> History {
        History {
            policy_version: POLICY_VERSION.into(),
            coverage,
            started_at: 10,
            observed_until: 100,
            episodes: Vec::new(),
            force_pushes: Vec::new(),
            limitations: Vec::new(),
        }
    }

    #[test]
    fn pairs_exact_user_request_with_review_and_keeps_head_changes_separate() {
        let mut history = history(Coverage::Complete);
        complete(
            &mut history,
            vec![
                Event {
                    id: "request-1".into(),
                    at: 20,
                    kind: EventKind::Requested(user("Reviewer")),
                },
                Event {
                    id: "review-1".into(),
                    at: 65,
                    kind: EventKind::Review {
                        reviewer: Some("reviewer".into()),
                    },
                },
                Event {
                    id: "push-1".into(),
                    at: 70,
                    kind: EventKind::ForcePushed {
                        before_oid: Some("head-a".into()),
                        after_oid: Some("head-b".into()),
                    },
                },
            ],
        );
        assert_eq!(history.episodes.len(), 1);
        assert_eq!(history.episodes[0].outcome, Outcome::Reviewed);
        assert_eq!(history.episodes[0].response_seconds, Some(45));
        assert_eq!(
            history.episodes[0].resolution_event_id.as_deref(),
            Some("review-1")
        );
        assert_eq!(history.force_pushes.len(), 1);
        assert_eq!(history.force_pushes[0].event_id, "push-1");
    }

    #[test]
    fn team_requests_and_partial_histories_do_not_fabricate_response_time() {
        let mut history = history(Coverage::Partial);
        let team = Some(Reviewer {
            kind: ReviewerKind::Team,
            identifier: Some("review-team".into()),
        });
        complete(
            &mut history,
            vec![
                Event {
                    id: "request-team".into(),
                    at: 20,
                    kind: EventKind::Requested(team),
                },
                Event {
                    id: "review-member".into(),
                    at: 65,
                    kind: EventKind::Review {
                        reviewer: Some("member".into()),
                    },
                },
            ],
        );
        assert_eq!(history.episodes[0].outcome, Outcome::Inconclusive);
        assert_eq!(history.episodes[0].response_seconds, None);
        assert!(history
            .limitations
            .iter()
            .any(|limitation| limitation == "partial-handoff-history"));
    }

    #[test]
    fn removed_request_closes_without_counting_as_review_response() {
        let mut history = history(Coverage::Complete);
        complete(
            &mut history,
            vec![
                Event {
                    id: "request-1".into(),
                    at: 20,
                    kind: EventKind::Requested(user("reviewer")),
                },
                Event {
                    id: "removed-1".into(),
                    at: 40,
                    kind: EventKind::Removed(user("reviewer")),
                },
            ],
        );
        assert_eq!(history.episodes[0].outcome, Outcome::Removed);
        assert_eq!(history.episodes[0].response_seconds, None);
    }

    #[test]
    fn pending_age_is_reported_only_from_complete_request_evidence() {
        let mut complete_history = history(Coverage::Complete);
        complete(
            &mut complete_history,
            vec![Event {
                id: "request-open".into(),
                at: 30,
                kind: EventKind::Requested(user("reviewer")),
            }],
        );
        assert_eq!(
            complete_history.episodes[0].outcome,
            Outcome::PendingAtCapture
        );
        assert_eq!(complete_history.episodes[0].pending_seconds, Some(70));

        let mut partial_history = history(Coverage::Partial);
        complete(
            &mut partial_history,
            vec![Event {
                id: "request-open".into(),
                at: 30,
                kind: EventKind::Requested(user("reviewer")),
            }],
        );
        assert_eq!(partial_history.episodes[0].pending_seconds, None);
    }
}
