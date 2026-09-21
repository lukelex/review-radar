use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub viewer: Actor,
    pub searches: Vec<Search>,
    pub pull_requests: Vec<PullRequest>,
}

impl Snapshot {
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("invalid GitHub snapshot")
    }

    pub fn tailored_queue(&self) -> Vec<PullRequestCard> {
        let memberships = self.memberships();
        let mut cards = self
            .pull_requests
            .iter()
            .map(|pr| project(pr, memberships.get(&pr.id)))
            .collect::<Vec<_>>();
        cards.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| right.updated_at.cmp(&left.updated_at))
                .then_with(|| left.id.cmp(&right.id))
        });
        cards
    }

    fn memberships(&self) -> BTreeMap<String, BTreeSet<String>> {
        let mut memberships = BTreeMap::<String, BTreeSet<String>>::new();
        for search in &self.searches {
            for id in &search.ids {
                memberships
                    .entry(id.clone())
                    .or_default()
                    .insert(search.category.clone());
            }
        }
        memberships
    }
}

#[derive(Debug, Deserialize)]
pub struct Search {
    pub category: String,
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub id: String,
    pub number: u64,
    pub title: String,
    pub url: String,
    pub state: String,
    pub is_draft: bool,
    pub author: Option<Actor>,
    pub repository: Repository,
    pub review_decision: Option<String>,
    pub mergeable: String,
    pub merge_state_status: String,
    pub updated_at: String,
    #[serde(default)]
    pub reviews: Connection<Review>,
    #[serde(default)]
    pub comments: Connection<Comment>,
    #[serde(default)]
    pub review_threads: Connection<ReviewThread>,
    #[serde(default)]
    pub commits: Connection<CommitNode>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Actor {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub name_with_owner: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct Connection<T> {
    #[serde(default)]
    pub nodes: Vec<T>,
}

impl<T> Default for Connection<T> {
    fn default() -> Self {
        Self { nodes: Vec::new() }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub id: String,
    pub author: Option<Actor>,
    pub state: String,
    pub submitted_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: String,
    pub author: Option<Actor>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewThread {
    pub id: String,
    pub is_resolved: bool,
    #[serde(default)]
    pub comments: Connection<Comment>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitNode {
    pub commit: Commit,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub status_check_rollup: Option<StatusCheckRollup>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatusCheckRollup {
    pub state: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Priority {
    ReviewRequested,
    AuthoredAction,
    AuthoredWaiting,
    Following,
    Recent,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Relationship {
    ReviewRequested,
    Authored,
    Following,
    Recent,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    ReviewRequested,
    ChangesRequested,
    ChecksFailing,
    MergeConflict,
    ReadyToMerge,
    AwaitingReview,
    Draft,
    Following,
    Merged,
    Closed,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Self::ReviewRequested => "Review requested",
            Self::ChangesRequested => "Changes requested",
            Self::ChecksFailing => "Checks failing",
            Self::MergeConflict => "Merge conflict",
            Self::ReadyToMerge => "Ready to merge",
            Self::AwaitingReview => "Awaiting review",
            Self::Draft => "Draft",
            Self::Following => "Following",
            Self::Merged => "Merged",
            Self::Closed => "Closed",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestCard {
    pub id: String,
    pub repository: String,
    pub number: u64,
    pub title: String,
    pub url: String,
    pub updated_at: String,
    pub priority: Priority,
    pub relationship: Relationship,
    pub action: Action,
    pub action_label: &'static str,
    pub attention_required: bool,
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventKind {
    Review,
    Comment,
    ReviewComment,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub fingerprint: String,
    pub kind: EventKind,
    pub actor: Option<String>,
    pub state: Option<String>,
    pub occurred_at: String,
}

fn project(pr: &PullRequest, membership: Option<&BTreeSet<String>>) -> PullRequestCard {
    let membership = membership.cloned().unwrap_or_default();
    let authored = membership.contains("authored");
    let review_requested = membership.contains("review_requested");
    let recent = matches!(pr.state.as_str(), "MERGED" | "CLOSED");
    let checks = pr
        .commits
        .nodes
        .last()
        .and_then(|node| node.commit.status_check_rollup.as_ref())
        .map(|rollup| rollup.state.as_str());

    let (priority, relationship, action) = if recent {
        (
            Priority::Recent,
            Relationship::Recent,
            if pr.state == "MERGED" {
                Action::Merged
            } else {
                Action::Closed
            },
        )
    } else if review_requested {
        (
            Priority::ReviewRequested,
            Relationship::ReviewRequested,
            Action::ReviewRequested,
        )
    } else if authored {
        let action = if pr.review_decision.as_deref() == Some("CHANGES_REQUESTED") {
            Action::ChangesRequested
        } else if checks == Some("FAILURE") || checks == Some("ERROR") {
            Action::ChecksFailing
        } else if pr.mergeable == "CONFLICTING" || pr.merge_state_status == "DIRTY" {
            Action::MergeConflict
        } else if !pr.is_draft
            && pr.review_decision.as_deref() == Some("APPROVED")
            && checks != Some("PENDING")
            && checks != Some("EXPECTED")
            && pr.mergeable == "MERGEABLE"
        {
            Action::ReadyToMerge
        } else if pr.is_draft {
            Action::Draft
        } else {
            Action::AwaitingReview
        };
        let requires_action = matches!(
            action,
            Action::ChangesRequested
                | Action::ChecksFailing
                | Action::MergeConflict
                | Action::ReadyToMerge
        );
        (
            if requires_action {
                Priority::AuthoredAction
            } else {
                Priority::AuthoredWaiting
            },
            Relationship::Authored,
            action,
        )
    } else {
        (
            Priority::Following,
            Relationship::Following,
            Action::Following,
        )
    };
    let attention_required = matches!(
        action,
        Action::ReviewRequested
            | Action::ChangesRequested
            | Action::ChecksFailing
            | Action::MergeConflict
            | Action::ReadyToMerge
    );
    let events = events(pr);
    PullRequestCard {
        id: pr.id.clone(),
        repository: pr.repository.name_with_owner.clone(),
        number: pr.number,
        title: pr.title.clone(),
        url: pr.url.clone(),
        updated_at: pr.updated_at.clone(),
        priority,
        relationship,
        action,
        action_label: action.label(),
        attention_required,
        events,
    }
}

fn events(pr: &PullRequest) -> Vec<Event> {
    let mut events = Vec::new();
    for review in &pr.reviews.nodes {
        let occurred_at = review
            .submitted_at
            .as_ref()
            .or(review.updated_at.as_ref())
            .cloned()
            .unwrap_or_else(|| pr.updated_at.clone());
        let version = review.updated_at.as_ref().unwrap_or(&occurred_at);
        events.push(Event {
            fingerprint: fingerprint(EventKind::Review, &review.id, version),
            kind: EventKind::Review,
            actor: review.author.as_ref().map(|actor| actor.login.clone()),
            state: Some(review.state.clone()),
            occurred_at,
        });
    }
    for comment in &pr.comments.nodes {
        events.push(comment_event(comment, EventKind::Comment));
    }
    for thread in &pr.review_threads.nodes {
        for comment in &thread.comments.nodes {
            events.push(comment_event(comment, EventKind::ReviewComment));
        }
    }
    events.sort_by(|left, right| {
        left.occurred_at
            .cmp(&right.occurred_at)
            .then_with(|| left.fingerprint.cmp(&right.fingerprint))
    });
    events
}

fn comment_event(comment: &Comment, kind: EventKind) -> Event {
    Event {
        fingerprint: fingerprint(kind, &comment.id, &comment.updated_at),
        kind,
        actor: comment.author.as_ref().map(|actor| actor.login.clone()),
        state: None,
        occurred_at: comment.created_at.clone(),
    }
}

fn fingerprint(kind: EventKind, id: &str, version: &str) -> String {
    let kind = match kind {
        EventKind::Review => "review",
        EventKind::Comment => "comment",
        EventKind::ReviewComment => "review-comment",
    };
    format!("{kind}:{id}:{version}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: &str = include_str!("../../../tests/fixtures/github/github-snapshot.json");

    #[test]
    fn fixture_projects_to_ranked_cards() {
        let snapshot = Snapshot::from_json(SEED).unwrap();
        let queue = snapshot.tailored_queue();
        assert_eq!(queue.len(), 97);
        assert!(queue.windows(2).all(|pair| {
            pair[0].priority < pair[1].priority
                || pair[0].priority == pair[1].priority && pair[0].updated_at >= pair[1].updated_at
        }));
        assert!(queue
            .iter()
            .any(|card| card.action == Action::ChecksFailing));
        assert!(queue
            .iter()
            .any(|card| card.action == Action::MergeConflict));
        assert!(queue
            .iter()
            .any(|card| card.action == Action::ChangesRequested));
        assert!(queue.iter().any(|card| card.action == Action::Draft));
        assert!(queue.iter().any(|card| card.action == Action::Merged));
        assert!(queue.iter().any(|card| card.action == Action::Closed));
    }

    #[test]
    fn relationship_precedence_matches_product_queue() {
        let snapshot = Snapshot::from_json(SEED).unwrap();
        let queue = snapshot.tailored_queue();
        let memberships = snapshot.memberships();
        for card in queue {
            let membership = &memberships[&card.id];
            if card.priority == Priority::Recent {
                continue;
            }
            if membership.contains("review_requested") {
                assert_eq!(card.relationship, Relationship::ReviewRequested);
                assert_eq!(card.priority, Priority::ReviewRequested);
            } else if membership.contains("authored") {
                assert_eq!(card.relationship, Relationship::Authored);
            } else {
                assert_eq!(card.relationship, Relationship::Following);
            }
        }
    }

    #[test]
    fn events_are_ordered_and_have_stable_source_fingerprints() {
        let snapshot = Snapshot::from_json(SEED).unwrap();
        let queue = snapshot.tailored_queue();
        let events = queue
            .iter()
            .flat_map(|card| card.events.iter())
            .collect::<Vec<_>>();
        assert!(!events.is_empty());
        for card in &queue {
            assert!(card
                .events
                .windows(2)
                .all(|pair| pair[0].occurred_at <= pair[1].occurred_at));
        }
        assert!(events
            .iter()
            .all(|event| event.fingerprint.split(':').count() >= 3));
    }
}
