use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

macro_rules! stable_string_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

stable_string_id!(PullRequestId);
stable_string_id!(EventFingerprint);

pub mod attention;
pub mod friction;
pub mod ranking;

pub use ranking::{
    HighestFrictionRanking, NewestActivityRanking, RankingStrategy, TailoredRanking,
};

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("invalid GitHub snapshot: {0}")]
    InvalidSnapshot(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub viewer: Actor,
    pub searches: Vec<Search>,
    pub pull_requests: Vec<PullRequest>,
    /// Optional normalized history; raw bounded GitHub snapshots omit it.
    #[serde(default)]
    pub review_histories: BTreeMap<String, friction::ReviewHistory>,
    /// Capture completion time used only to compare with a predecessor. It is
    /// absent from fixture-only snapshots, which deliberately yields no delta.
    #[serde(default)]
    pub captured_at: Option<String>,
}

impl Snapshot {
    pub fn from_json(json: &str) -> std::result::Result<Self, DomainError> {
        serde_json::from_str(json).map_err(DomainError::InvalidSnapshot)
    }

    pub fn tailored_queue(&self) -> Vec<PullRequestCard> {
        self.ranked(&TailoredRanking)
    }

    pub fn ranked(&self, ranking: &dyn RankingStrategy) -> Vec<PullRequestCard> {
        self.ranked_since(ranking, None)
    }

    /// Project against the immediately preceding successful capture. Feedback
    /// must be absent from that predecessor and timestamped after it completed;
    /// a first capture establishes a baseline instead of flagging its bounded
    /// event tail as new.
    pub fn ranked_since(
        &self,
        ranking: &dyn RankingStrategy,
        predecessor: Option<&Snapshot>,
    ) -> Vec<PullRequestCard> {
        self.ranked_since_with_feedback(ranking, predecessor, &BTreeMap::new())
    }

    /// Project a capture while retaining feedback that a local client has not
    /// acknowledged yet. Local feedback is deliberately separate from the
    /// capture delta: bounded event windows may stop reporting an event after
    /// it was first observed.
    pub fn ranked_since_with_feedback(
        &self,
        ranking: &dyn RankingStrategy,
        predecessor: Option<&Snapshot>,
        retained_feedback: &BTreeMap<String, BTreeSet<String>>,
    ) -> Vec<PullRequestCard> {
        let memberships = self.memberships();
        let detected_feedback = predecessor
            .and_then(|previous| {
                previous
                    .captured_at
                    .as_deref()
                    .map(|captured_at| self.new_feedback_since(previous, captured_at))
            })
            .unwrap_or_default();
        let mut cards =
            self.pull_requests
                .iter()
                .map(|pr| {
                    let mut feedback = detected_feedback
                        .get(pr.id.as_str())
                        .cloned()
                        .unwrap_or_default();
                    let retained = retained_feedback.get(pr.id.as_str()).into_iter().flat_map(
                        |fingerprints| {
                            events(pr)
                                .into_iter()
                                .filter(|event| fingerprints.contains(&event.fingerprint))
                                .collect::<Vec<_>>()
                        },
                    );
                    for event in retained {
                        if !feedback
                            .iter()
                            .any(|existing| existing.fingerprint == event.fingerprint)
                        {
                            feedback.push(event);
                        }
                    }
                    // Preserve the reason even when a bounded current event window
                    // no longer contains the event. Its fingerprint remains the
                    // local evidence and the current PR timestamp is conservative.
                    for fingerprint in retained_feedback
                        .get(pr.id.as_str())
                        .into_iter()
                        .flat_map(|set| set.iter())
                    {
                        if !feedback
                            .iter()
                            .any(|event| &event.fingerprint == fingerprint)
                        {
                            feedback.push(Event {
                                fingerprint: fingerprint.clone(),
                                kind: EventKind::Comment,
                                actor: None,
                                state: None,
                                occurred_at: pr.updated_at.clone(),
                                is_bot: false,
                            });
                        }
                    }
                    project(
                        pr,
                        memberships.get(pr.id.as_str()),
                        self.review_histories.get(pr.id.as_str()),
                        &feedback,
                    )
                })
                .collect::<Vec<_>>();
        ranking.rank(&mut cards);
        cards
    }

    pub fn view(&self, view: WorkspaceView) -> Vec<PullRequestCard> {
        self.view_with_ranking(view, &TailoredRanking)
    }

    pub fn view_with_ranking(
        &self,
        view: WorkspaceView,
        ranking: &dyn RankingStrategy,
    ) -> Vec<PullRequestCard> {
        self.view_with_ranking_since(view, ranking, None)
    }

    pub fn view_with_ranking_since(
        &self,
        view: WorkspaceView,
        ranking: &dyn RankingStrategy,
        predecessor: Option<&Snapshot>,
    ) -> Vec<PullRequestCard> {
        self.view_with_ranking_since_and_feedback(view, ranking, predecessor, &BTreeMap::new())
    }

    pub fn view_with_ranking_since_and_feedback(
        &self,
        view: WorkspaceView,
        ranking: &dyn RankingStrategy,
        predecessor: Option<&Snapshot>,
        retained_feedback: &BTreeMap<String, BTreeSet<String>>,
    ) -> Vec<PullRequestCard> {
        self.ranked_since_with_feedback(ranking, predecessor, retained_feedback)
            .into_iter()
            .filter(|card| match view {
                // Completed work belongs in History/Recent, not in the
                // actionable default queue. Keep closed PRs eligible for
                // now, since they can still represent unresolved follow-up;
                // merged PRs have a dedicated historical view.
                WorkspaceView::Tailored => card.lifecycle != Lifecycle::Merged,
                WorkspaceView::Action => card.attention_required,
                WorkspaceView::MyPrs => {
                    card.memberships.contains(&"authored".to_owned())
                        && card.lifecycle == Lifecycle::Open
                }
                WorkspaceView::Following => {
                    card.lifecycle == Lifecycle::Open
                        && !card.memberships.contains(&"authored".to_owned())
                        && (card.memberships.contains(&"involved".to_owned())
                            || card.memberships.contains(&"review_involved".to_owned()))
                }
                WorkspaceView::Recent => {
                    (card.lifecycle == Lifecycle::Merged || card.lifecycle == Lifecycle::Closed)
                        && (card.memberships.contains(&"recent".to_owned())
                            || card
                                .memberships
                                .contains(&"recent_review_involved".to_owned()))
                }
            })
            .collect()
    }

    fn new_feedback_since(
        &self,
        predecessor: &Snapshot,
        predecessor_captured_at: &str,
    ) -> BTreeMap<String, Vec<Event>> {
        let prior_events = predecessor
            .pull_requests
            .iter()
            .map(|pr| {
                (
                    pr.id.as_str(),
                    events(pr)
                        .into_iter()
                        .map(|event| event.fingerprint)
                        .collect::<BTreeSet<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        self.pull_requests
            .iter()
            .filter_map(|pr| {
                let prior = prior_events.get(pr.id.as_str())?;
                let feedback = events(pr)
                    .into_iter()
                    .filter(|event| {
                        event.actor.is_some()
                            && event.actor.as_deref() != Some(self.viewer.login.as_str())
                            && !event.is_bot
                            && is_substantive_feedback(event)
                            && event.occurred_at.as_str() > predecessor_captured_at
                            && !prior.contains(&event.fingerprint)
                    })
                    .collect::<Vec<_>>();
                (!feedback.is_empty()).then(|| (pr.id.as_str().to_owned(), feedback))
            })
            .collect()
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
    pub id: PullRequestId,
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
    #[serde(default, rename = "__typename")]
    pub typename: Option<String>,
}

impl Actor {
    fn is_bot(&self) -> bool {
        self.typename.as_deref() == Some("Bot")
    }
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum WorkspaceView {
    Tailored,
    Action,
    MyPrs,
    Following,
    Recent,
}

impl WorkspaceView {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "tailored" => Some(Self::Tailored),
            "action" => Some(Self::Action),
            "my-prs" => Some(Self::MyPrs),
            "following" => Some(Self::Following),
            "recent" => Some(Self::Recent),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tailored => "tailored",
            Self::Action => "action",
            Self::MyPrs => "my-prs",
            Self::Following => "following",
            Self::Recent => "recent",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Lifecycle {
    Open,
    Merged,
    Closed,
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
    NewFeedback,
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
            Self::NewFeedback => "New feedback",
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
    pub id: PullRequestId,
    pub repository: String,
    pub number: u64,
    pub title: String,
    pub url: String,
    pub updated_at: String,
    pub lifecycle: Lifecycle,
    pub memberships: Vec<String>,
    pub priority: Priority,
    pub relationship: Relationship,
    pub action: Action,
    pub action_label: &'static str,
    pub attention_required: bool,
    pub explanation: attention::Explanation,
    pub review_friction: friction::Assessment,
    /// Stable for unchanged captured signals; changes when the current action,
    /// review, check, merge, or latest-activity signal changes.
    pub current_fingerprint: EventFingerprint,
    #[serde(skip)]
    pub feedback_fingerprints: Vec<EventFingerprint>,
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
    #[serde(skip)]
    is_bot: bool,
}

fn project(
    pr: &PullRequest,
    membership: Option<&BTreeSet<String>>,
    history: Option<&friction::ReviewHistory>,
    new_feedback: &[Event],
) -> PullRequestCard {
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
        } else if !new_feedback.is_empty() {
            Action::NewFeedback
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
                | Action::NewFeedback
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
            | Action::NewFeedback
            | Action::ChecksFailing
            | Action::MergeConflict
            | Action::ReadyToMerge
    );
    let events = events(pr);
    let current_fingerprint = current_fingerprint(pr, action, checks, new_feedback);
    PullRequestCard {
        id: pr.id.clone(),
        repository: pr.repository.name_with_owner.clone(),
        number: pr.number,
        title: pr.title.clone(),
        url: pr.url.clone(),
        updated_at: pr.updated_at.clone(),
        lifecycle: lifecycle(&pr.state),
        memberships: membership.into_iter().collect(),
        priority,
        relationship,
        action,
        action_label: action.label(),
        attention_required,
        explanation: attention::explain(pr, action, authored, attention_required, checks),
        review_friction: friction::assess(history, lifecycle(&pr.state), pr.is_draft),
        current_fingerprint: current_fingerprint.into(),
        feedback_fingerprints: new_feedback
            .iter()
            .map(|event| event.fingerprint.clone().into())
            .collect(),
        events,
    }
}

fn current_fingerprint(
    pr: &PullRequest,
    action: Action,
    checks: Option<&str>,
    new_feedback: &[Event],
) -> String {
    let newest_feedback = new_feedback
        .iter()
        .max_by(|left, right| {
            left.occurred_at
                .cmp(&right.occurred_at)
                .then_with(|| left.fingerprint.cmp(&right.fingerprint))
        })
        .map(|event| event.fingerprint.as_str())
        .unwrap_or("none");
    format!(
        "current:{}:{:?}:{:?}:{}:{}:{}:{}",
        pr.id,
        action,
        pr.review_decision.as_deref().unwrap_or("none"),
        checks.unwrap_or("none"),
        pr.mergeable,
        pr.merge_state_status,
        newest_feedback,
    )
}

fn lifecycle(state: &str) -> Lifecycle {
    match state {
        "MERGED" => Lifecycle::Merged,
        "CLOSED" => Lifecycle::Closed,
        _ => Lifecycle::Open,
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
            is_bot: review.author.as_ref().is_some_and(Actor::is_bot),
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

fn is_substantive_feedback(event: &Event) -> bool {
    match event.kind {
        EventKind::Review => event.state.as_deref() != Some("PENDING"),
        EventKind::Comment | EventKind::ReviewComment => true,
    }
}

fn comment_event(comment: &Comment, kind: EventKind) -> Event {
    Event {
        fingerprint: fingerprint(kind, &comment.id, &comment.updated_at),
        kind,
        actor: comment.author.as_ref().map(|actor| actor.login.clone()),
        state: None,
        occurred_at: comment.created_at.clone(),
        is_bot: comment.author.as_ref().is_some_and(Actor::is_bot),
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
    fn snapshot_accepts_hydrated_review_threads_without_unused_fields() {
        let snapshot = serde_json::json!({
            "viewer": { "login": "viewer" },
            "searches": [],
            "pullRequests": [{
                "id": "PR_1", "number": 1, "title": "Example",
                "url": "https://github.com/example/repository/pull/1",
                "state": "OPEN", "isDraft": false,
                "repository": { "nameWithOwner": "example/repository" },
                "mergeable": "MERGEABLE", "mergeStateStatus": "CLEAN",
                "updatedAt": "2026-09-23T12:00:00Z",
                "reviewThreads": { "nodes": [{ "comments": { "nodes": [] } }] }
            }]
        });

        Snapshot::from_json(&snapshot.to_string()).unwrap();
    }

    #[test]
    fn relationship_precedence_matches_product_queue() {
        let snapshot = Snapshot::from_json(SEED).unwrap();
        let queue = snapshot.tailored_queue();
        let memberships = snapshot.memberships();
        for card in queue {
            let membership = &memberships[card.id.as_str()];
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
    fn workspace_views_are_filtered_from_membership_not_top_relationship() {
        let snapshot = Snapshot::from_json(SEED).unwrap();
        let action = snapshot.view(WorkspaceView::Action);
        let tailored = snapshot.view(WorkspaceView::Tailored);
        let authored = snapshot.view(WorkspaceView::MyPrs);
        let following = snapshot.view(WorkspaceView::Following);
        let recent = snapshot.view(WorkspaceView::Recent);

        assert!(action.iter().all(|card| card.attention_required));
        assert!(tailored
            .iter()
            .all(|card| card.lifecycle != Lifecycle::Merged));
        assert!(authored.iter().all(|card| card.lifecycle == Lifecycle::Open
            && card.memberships.contains(&"authored".into())));
        assert!(following.iter().all(|card| {
            card.lifecycle == Lifecycle::Open
                && !card.memberships.contains(&"authored".into())
                && (card.memberships.contains(&"involved".into())
                    || card.memberships.contains(&"review_involved".into()))
        }));
        assert!(recent.iter().all(|card| {
            matches!(card.lifecycle, Lifecycle::Merged | Lifecycle::Closed)
                && (card.memberships.contains(&"recent".into())
                    || card.memberships.contains(&"recent_review_involved".into()))
        }));
        assert!(recent
            .iter()
            .any(|card| card.lifecycle == Lifecycle::Merged));
        assert!(WorkspaceView::parse("my-prs").is_some());
        assert!(WorkspaceView::parse("unknown").is_none());
    }

    #[test]
    fn fixture_projects_every_supported_queue_view() {
        let snapshot = Snapshot::from_json(SEED).unwrap();
        for view in [
            WorkspaceView::Tailored,
            WorkspaceView::Action,
            WorkspaceView::MyPrs,
            WorkspaceView::Following,
            WorkspaceView::Recent,
        ] {
            let cards = snapshot.view(view);
            assert!(!cards.is_empty(), "fixture view {view:?} is empty");
            let json = serde_json::to_value(&cards).unwrap();
            assert!(json.is_array());
            assert!(cards
                .iter()
                .all(|card| !card.current_fingerprint.is_empty()));
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
        assert!(queue
            .iter()
            .all(|card| card.current_fingerprint.as_str().starts_with("current:")));
    }

    #[test]
    fn ranking_strategies_can_be_swapped_without_changing_view_membership() {
        let snapshot = Snapshot::from_json(SEED).unwrap();
        let tailored = snapshot.view_with_ranking(WorkspaceView::Action, &TailoredRanking);
        let chronological =
            snapshot.view_with_ranking(WorkspaceView::Action, &NewestActivityRanking);

        assert_eq!(TailoredRanking.id(), "tailored");
        assert_eq!(NewestActivityRanking.id(), "newest-activity");
        assert_eq!(
            tailored
                .iter()
                .map(|card| &card.id)
                .collect::<BTreeSet<_>>(),
            chronological
                .iter()
                .map(|card| &card.id)
                .collect::<BTreeSet<_>>()
        );
        assert!(chronological
            .windows(2)
            .all(|pair| pair[0].updated_at >= pair[1].updated_at));
    }

    #[test]
    fn new_reviewer_feedback_is_compared_with_the_predecessor_not_inferred_from_a_tail() {
        let previous = feedback_snapshot("2026-09-21T12:00:00Z", vec![], vec![]);
        let current = feedback_snapshot(
            "2026-09-21T12:05:00Z",
            vec![serde_json::json!({
                "id": "comment-1", "author": { "login": "reviewer-001" },
                "createdAt": "2026-09-21T12:01:00Z", "updatedAt": "2026-09-21T12:01:00Z"
            })],
            vec![],
        );
        let card = current
            .view_with_ranking_since(WorkspaceView::Action, &TailoredRanking, Some(&previous))
            .pop()
            .unwrap();
        assert_eq!(card.action, Action::NewFeedback);
        assert!(card.attention_required);
        assert_eq!(
            card.explanation.reasons[0].evidence,
            ["search:authored", "capture-delta"]
        );
        assert!(card
            .current_fingerprint
            .as_str()
            .contains("comment:comment-1"));

        // The same bounded event tail is a baseline when there is no prior
        // capture, and is not repeatedly classified after it was observed.
        assert!(current.view(WorkspaceView::Action).is_empty());
        let observed = feedback_snapshot(
            "2026-09-21T12:03:00Z",
            vec![serde_json::json!({
                "id": "comment-1", "author": { "login": "reviewer-001" },
                "createdAt": "2026-09-21T12:01:00Z", "updatedAt": "2026-09-21T12:01:00Z"
            })],
            vec![],
        );
        assert!(current
            .view_with_ranking_since(WorkspaceView::Action, &TailoredRanking, Some(&observed))
            .is_empty());
    }

    #[test]
    fn locally_retained_feedback_survives_a_bounded_event_window() {
        let current = feedback_snapshot("2026-09-21T12:05:00Z", vec![], vec![]);
        let mut retained = BTreeMap::new();
        retained.insert(
            "pr-001".into(),
            ["comment:comment-1:2026-09-21T12:01:00Z".into()]
                .into_iter()
                .collect(),
        );
        let card = current
            .view_with_ranking_since_and_feedback(
                WorkspaceView::Action,
                &TailoredRanking,
                None,
                &retained,
            )
            .pop()
            .unwrap();
        assert_eq!(card.action, Action::NewFeedback);
        assert_eq!(card.feedback_fingerprints.len(), 1);
        assert!(card
            .current_fingerprint
            .as_str()
            .contains("comment:comment-1"));
    }

    #[test]
    fn only_non_self_substantive_events_after_predecessor_can_be_new_feedback() {
        let previous = feedback_snapshot("2026-09-21T12:00:00Z", vec![], vec![]);
        for (comments, reviews) in [
            (
                vec![serde_json::json!({
                    "id": "old-comment", "author": { "login": "reviewer-001" },
                    "createdAt": "2026-09-21T11:59:00Z", "updatedAt": "2026-09-21T11:59:00Z"
                })],
                vec![],
            ),
            (
                vec![serde_json::json!({
                    "id": "self-comment", "author": { "login": "viewer-001" },
                    "createdAt": "2026-09-21T12:01:00Z", "updatedAt": "2026-09-21T12:01:00Z"
                })],
                vec![],
            ),
            (
                vec![],
                vec![serde_json::json!({
                    "id": "pending-review", "author": { "login": "reviewer-001" },
                    "state": "PENDING", "submittedAt": null, "updatedAt": "2026-09-21T12:01:00Z"
                })],
            ),
            (
                vec![serde_json::json!({
                    "id": "bot-comment", "author": { "login": "bot-001", "__typename": "Bot" },
                    "createdAt": "2026-09-21T12:01:00Z", "updatedAt": "2026-09-21T12:01:00Z"
                })],
                vec![],
            ),
        ] {
            let current = feedback_snapshot("2026-09-21T12:05:00Z", comments, reviews);
            assert!(current
                .view_with_ranking_since(WorkspaceView::Action, &TailoredRanking, Some(&previous))
                .is_empty());
        }
    }

    fn feedback_snapshot(
        captured_at: &str,
        comments: Vec<serde_json::Value>,
        reviews: Vec<serde_json::Value>,
    ) -> Snapshot {
        Snapshot::from_json(
            &serde_json::json!({
                "capturedAt": captured_at,
                "viewer": { "login": "viewer-001" },
                "searches": [{ "category": "authored", "ids": ["pr-001"] }],
                "pullRequests": [{
                    "id": "pr-001", "number": 1, "title": "Example", "url": "https://github.com/example/repository/pull/1",
                    "state": "OPEN", "isDraft": false, "author": { "login": "viewer-001" },
                    "repository": { "nameWithOwner": "example/repository" },
                    "reviewDecision": null, "mergeable": "UNKNOWN", "mergeStateStatus": "UNKNOWN",
                    "updatedAt": "2026-09-21T12:05:00Z", "comments": { "nodes": comments }, "reviews": { "nodes": reviews }
                }]
            })
            .to_string(),
        )
        .unwrap()
    }
}
