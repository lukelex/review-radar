//! Queue ordering policies. Policies only order projected cards; they do not
//! classify GitHub data or decide which cards a workspace view includes.

use crate::PullRequestCard;

/// Resolve stable IDs in the core, so clients do not implement policy selection.
pub fn by_id(id: &str) -> Option<Box<dyn RankingStrategy>> {
    match id {
        "tailored" => Some(Box::new(TailoredRanking)),
        "newest-activity" => Some(Box::new(NewestActivityRanking)),
        "highest-friction" => Some(Box::new(HighestFrictionRanking)),
        _ => None,
    }
}

/// Orders already-projected cards for a workspace view.
///
/// New policies can be supplied without changing snapshot normalization,
/// classification, or view filtering.
pub trait RankingStrategy: Send + Sync {
    /// Stable identifier emitted in client view models and persisted preferences.
    fn id(&self) -> &'static str;

    /// Reorder the supplied cards in place.
    fn rank(&self, cards: &mut [PullRequestCard]);
}

/// Product-default ordering: documented priority bands, newest activity first
/// inside each band, then a stable node-ID tiebreaker.
#[derive(Debug, Default)]
pub struct TailoredRanking;

impl RankingStrategy for TailoredRanking {
    fn id(&self) -> &'static str {
        "tailored"
    }

    fn rank(&self, cards: &mut [PullRequestCard]) {
        cards.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| right.updated_at.cmp(&left.updated_at))
                .then_with(|| left.id.cmp(&right.id))
        });
    }
}

/// Alternative chronological ordering for a future activity-oriented workspace.
#[derive(Debug, Default)]
pub struct NewestActivityRanking;

/// Explicit alternative: assessed levels first, then limited and not assessed.
/// This crosses priority bands but never changes workspace membership.
#[derive(Debug, Default)]
pub struct HighestFrictionRanking;

impl RankingStrategy for HighestFrictionRanking {
    fn id(&self) -> &'static str {
        "highest-friction"
    }

    fn rank(&self, cards: &mut [PullRequestCard]) {
        use crate::friction::{AssessmentStatus, Level};
        let bucket = |card: &PullRequestCard| match (
            card.review_friction.status,
            card.review_friction.level,
        ) {
            (AssessmentStatus::Assessed, Some(Level::High)) => 0,
            (AssessmentStatus::Assessed, Some(Level::Moderate)) => 1,
            (AssessmentStatus::Assessed, Some(Level::Low)) => 2,
            (AssessmentStatus::NotAssessed, _) => 4,
            _ => 3,
        };
        cards.sort_by(|left, right| {
            bucket(left)
                .cmp(&bucket(right))
                .then_with(|| {
                    if bucket(left) < 3 && bucket(right) < 3 {
                        right
                            .review_friction
                            .review_seconds
                            .cmp(&left.review_friction.review_seconds)
                    } else {
                        std::cmp::Ordering::Equal
                    }
                })
                .then_with(|| right.updated_at.cmp(&left.updated_at))
                .then_with(|| left.id.cmp(&right.id))
        });
    }
}

impl RankingStrategy for NewestActivityRanking {
    fn id(&self) -> &'static str {
        "newest-activity"
    }

    fn rank(&self, cards: &mut [PullRequestCard]) {
        cards.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.id.cmp(&right.id))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        friction::{AssessmentStatus, Level},
        Snapshot, WorkspaceView,
    };
    use std::collections::BTreeSet;

    #[test]
    fn friction_order_crosses_bands_and_handles_unknowns_and_stable_ties() {
        let snapshot = Snapshot::from_json(include_str!(
            "../../../tests/fixtures/github/github-snapshot.json"
        ))
        .unwrap();
        let template = snapshot.tailored_queue().remove(0);
        let mut cards = Vec::new();
        for (id, status, level, seconds) in [
            ("not-started", AssessmentStatus::NotAssessed, None, Some(0)),
            (
                "unknown",
                AssessmentStatus::LimitedHistory,
                None,
                Some(9_000_000),
            ),
            ("low", AssessmentStatus::Assessed, Some(Level::Low), Some(1)),
            (
                "moderate",
                AssessmentStatus::Assessed,
                Some(Level::Moderate),
                Some(10),
            ),
            (
                "high-b",
                AssessmentStatus::Assessed,
                Some(Level::High),
                Some(20),
            ),
            (
                "high-a",
                AssessmentStatus::Assessed,
                Some(Level::High),
                Some(20),
            ),
            (
                "high-long",
                AssessmentStatus::Assessed,
                Some(Level::High),
                Some(30),
            ),
            (
                "high-unknown-time",
                AssessmentStatus::Assessed,
                Some(Level::High),
                None,
            ),
        ] {
            let mut card = template.clone();
            card.id = id.into();
            card.review_friction.status = status;
            card.review_friction.level = level;
            card.review_friction.review_seconds = seconds;
            if id == "high-long" {
                card.priority = crate::Priority::AuthoredWaiting;
                card.attention_required = false;
            }
            cards.push(card);
        }
        HighestFrictionRanking.rank(&mut cards);
        assert_eq!(
            cards.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
            [
                "high-long",
                "high-a",
                "high-b",
                "high-unknown-time",
                "moderate",
                "low",
                "unknown",
                "not-started"
            ]
        );
        assert!(!cards[0].attention_required);
    }

    #[test]
    fn choosing_friction_preserves_every_view_membership() {
        let snapshot = Snapshot::from_json(include_str!(
            "../../../tests/fixtures/github/github-snapshot.json"
        ))
        .unwrap();
        for view in [
            WorkspaceView::Tailored,
            WorkspaceView::Action,
            WorkspaceView::MyPrs,
            WorkspaceView::Following,
            WorkspaceView::Recent,
        ] {
            let ids = |ranking: &dyn RankingStrategy| {
                snapshot
                    .view_with_ranking(view, ranking)
                    .into_iter()
                    .map(|c| c.id)
                    .collect::<BTreeSet<_>>()
            };
            assert_eq!(ids(&TailoredRanking), ids(&HighestFrictionRanking));
        }
        assert_eq!(by_id("highest-friction").unwrap().id(), "highest-friction");
        assert!(by_id("unknown").is_none());
    }
}
