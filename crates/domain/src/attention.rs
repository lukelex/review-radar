//! Evidence-backed presentation of the current classification. Explanations do
//! not invent event newness from a bounded snapshot or change attention state.

use serde::Serialize;

use crate::{Action, PullRequest};

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Explanation {
    pub heading: &'static str,
    pub reasons: Vec<Reason>,
    pub health: Health,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reason {
    pub code: Action,
    pub summary: &'static str,
    /// Snapshot field or search membership supporting this reason. This is not
    /// an event timestamp: current conditions may outlive their captured events.
    pub evidence: Vec<&'static str>,
    pub next_action: NextAction,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextAction {
    pub label: &'static str,
    pub url: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub review_decision: Option<String>,
    pub checks: Option<String>,
    pub mergeable: String,
    pub merge_state_status: String,
    pub is_draft: bool,
}

pub(crate) fn explain(
    pr: &PullRequest,
    action: Action,
    authored: bool,
    attention_required: bool,
    checks: Option<&str>,
) -> Explanation {
    let mut codes = vec![action];
    // Preserve simultaneous problems instead of hiding everything behind the
    // precedence-selected top action. Completed PRs never have stale obligations.
    if authored && pr.state == "OPEN" {
        for (present, code) in [
            (
                pr.review_decision.as_deref() == Some("CHANGES_REQUESTED"),
                Action::ChangesRequested,
            ),
            (
                matches!(checks, Some("FAILURE" | "ERROR")),
                Action::ChecksFailing,
            ),
            (
                pr.mergeable == "CONFLICTING" || pr.merge_state_status == "DIRTY",
                Action::MergeConflict,
            ),
        ] {
            if present && !codes.contains(&code) {
                codes.push(code);
            }
        }
    }
    Explanation {
        heading: if attention_required {
            "Why this needs your attention"
        } else {
            "Why this is here"
        },
        reasons: codes.into_iter().map(|code| reason(pr, code)).collect(),
        health: Health {
            review_decision: pr.review_decision.clone(),
            checks: checks.map(str::to_owned),
            mergeable: pr.mergeable.clone(),
            merge_state_status: pr.merge_state_status.clone(),
            is_draft: pr.is_draft,
        },
    }
}

fn reason(pr: &PullRequest, code: Action) -> Reason {
    let (summary, evidence, label, suffix): (_, Vec<_>, _, _) = match code {
        Action::ReviewRequested => (
            "Your review is requested and remains outstanding.",
            vec!["search:review_requested"], "Start review", "/files",
        ),
        Action::ChangesRequested => (
            "Changes were requested on your PR. Review and address the feedback.",
            vec!["search:authored", "reviewDecision"], "View feedback", "",
        ),
        Action::NewFeedback => (
            "New reviewer feedback was captured after the previous refresh. Read and respond as needed.",
            vec!["search:authored", "capture-delta"], "View feedback", "",
        ),
        Action::ChecksFailing => (
            "Checks are failing on your PR. Inspect the failures.",
            vec!["search:authored", "commits.statusCheckRollup.state"], "View checks", "/checks",
        ),
        Action::MergeConflict => (
            "Your PR has a merge conflict that needs resolution.",
            vec!["search:authored", "mergeable", "mergeStateStatus"], "Open PR", "",
        ),
        Action::ReadyToMerge => (
            "Your PR is approved and has no known blocking checks or conflicts. Confirm merge requirements in GitHub.",
            vec!["search:authored", "reviewDecision", "isDraft", "mergeable", "commits.statusCheckRollup.state"], "Open PR", "",
        ),
        Action::AwaitingReview => (
            "Your PR is waiting for review or check progress; no current action is identified.",
            vec!["search:authored", "reviewDecision", "commits.statusCheckRollup.state"], "Open PR", "",
        ),
        Action::Draft => (
            "Your draft is here to track work before it is ready for review.",
            vec!["search:authored", "isDraft"], "Open PR", "",
        ),
        Action::Following => (
            "This PR is in your followed workspace. Catch up when useful.",
            vec!["searchMemberships"], "View activity", "",
        ),
        Action::Merged => (
            "This PR was merged. Its outcome is here for awareness.",
            vec!["searchMemberships", "state"], "View PR", "",
        ),
        Action::Closed => (
            "This PR was closed without merging. Its outcome is here for awareness.",
            vec!["searchMemberships", "state"], "View PR", "",
        ),
    };
    Reason {
        code,
        summary,
        evidence,
        next_action: NextAction {
            label,
            url: format!("{}{suffix}", pr.url.trim_end_matches('/')),
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::{Action, Snapshot};

    const SEED: &str = include_str!("../../../tests/fixtures/github/github-snapshot.json");

    #[test]
    fn simultaneous_problems_have_reasons_and_destinations() {
        let mut snapshot = Snapshot::from_json(SEED).unwrap();
        let pr = &mut snapshot.pull_requests[0];
        pr.state = "OPEN".into();
        pr.review_decision = Some("CHANGES_REQUESTED".into());
        pr.mergeable = "CONFLICTING".into();
        let explanation = super::explain(pr, Action::ChangesRequested, true, true, Some("FAILURE"));
        assert_eq!(
            explanation
                .reasons
                .iter()
                .map(|r| r.code)
                .collect::<Vec<_>>(),
            [
                Action::ChangesRequested,
                Action::ChecksFailing,
                Action::MergeConflict
            ]
        );
        assert!(explanation.reasons[1].next_action.url.ends_with("/checks"));
        assert_eq!(explanation.health.checks.as_deref(), Some("FAILURE"));
        assert_eq!(explanation.heading, "Why this needs your attention");
    }

    #[test]
    fn completions_suppress_stale_obligations_and_unknown_is_not_passing() {
        let mut snapshot = Snapshot::from_json(SEED).unwrap();
        let pr = &mut snapshot.pull_requests[0];
        pr.state = "MERGED".into();
        pr.review_decision = Some("CHANGES_REQUESTED".into());
        pr.mergeable = "CONFLICTING".into();
        let explanation = super::explain(pr, Action::Merged, true, false, None);
        assert_eq!(explanation.reasons.len(), 1);
        assert_eq!(explanation.reasons[0].code, Action::Merged);
        assert_eq!(explanation.health.checks, None);
        assert_eq!(explanation.heading, "Why this is here");
    }

    #[test]
    fn every_projected_card_has_personal_context_and_a_next_action() {
        let snapshot = Snapshot::from_json(SEED).unwrap();
        for card in snapshot.tailored_queue() {
            let primary = &card.explanation.reasons[0];
            assert_eq!(primary.code, card.action);
            assert!(!primary.evidence.is_empty());
            assert!(primary.next_action.url.starts_with(&card.url));
            assert_eq!(
                card.explanation.heading == "Why this needs your attention",
                card.attention_required
            );
            if card.action == Action::ReviewRequested {
                assert_eq!(primary.next_action.label, "Start review");
                assert!(primary.next_action.url.ends_with("/files"));
            }
        }
    }
}
