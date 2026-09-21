use anyhow::Result;
use chrono::{DateTime, Utc};
use review_radar_domain::PullRequestCard;
use review_radar_state::StateStore;

/// The queue's local-state result, ready for serialization by a client.
#[derive(Debug)]
pub struct StateProjection {
    pub cards: Vec<PullRequestCard>,
    pub source_count: usize,
    pub suppressed_count: usize,
    pub notification_eligible_ids: Vec<String>,
}

/// Apply per-device acknowledgement, snooze, and notification state without
/// changing the shared domain projection.
pub fn apply_local_state(
    cards: Vec<PullRequestCard>,
    state: &StateStore,
    record_attention: bool,
    now: DateTime<Utc>,
) -> Result<StateProjection> {
    let source_count = cards.len();
    let mut visible = Vec::new();
    let mut notification_eligible_ids = Vec::new();
    for card in cards {
        if record_attention
            && state
                .observe_attention(
                    card.id.as_str(),
                    card.attention_required,
                    card.current_fingerprint.as_str(),
                    now,
                )?
                .notification_due
        {
            notification_eligible_ids.push(card.id.to_string());
        }
        if !state.is_suppressed(card.id.as_str(), card.current_fingerprint.as_str(), now)? {
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
