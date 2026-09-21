//! Queue ordering policies. Policies only order projected cards; they do not
//! classify GitHub data or decide which cards a workspace view includes.

use crate::PullRequestCard;

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
