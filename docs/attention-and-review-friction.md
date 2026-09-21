# Attention explanations and review friction

Status: accepted design direction for future implementation. The static
[mockups](mockups/README.md) illustrate the experience; their friction assessments,
review-round counts, durations, and churn labels are invented examples, not output
from the current collector or domain model.

Implementation progress: current snapshot-based attention explanations are now
projected in `crates/domain/src/attention.rs` and included in queue JSON as
`explanation` (heading, ordered reasons with evidence fields and next actions,
and concurrent health). Reasons use factual snapshot conditions without claiming
unavailable event newness or requester identity. The primary reason matches the
existing classification; additional authored blockers remain visible. See
[TODO](../TODO.md) for the remaining work.

## Two questions on every PR

1. **Why does this need my attention?** Explain the user's relationship, the
   outstanding event or condition, and the next useful action.
2. **How much has this PR struggled?** Explain accumulated review friction (the
   “pain level”), independently of its current urgency.

Every card and table row must justify its place without expansion. A generic
“New comment” badge is supporting evidence; “user-004 left a new comment on your
PR. Read and respond as needed” connects it to the user. Keep all concurrent health
issues visible, including failures and conflicts alongside review feedback.

For waiting, following, drafts, and completed PRs, use **Why this is here** and
explicit **Waiting** or **For awareness** wording where appropriate. Never invent
an obligation to justify inclusion. High friction can coexist with no action
needed from the user. Completed PRs describe outcomes rather than stale actions.

### Presentation contract

- Identity: title, repository/number, lifecycle, relationship when known.
- Prominent reason: factual, concise, personally relevant, above health metadata.
- Supporting health: all applicable signals, including unknown states.
- Review friction: a level plus its main contributing facts; available on every
  representation, including compact queue entries and table rows.
- Age: distinguish latest activity from the timestamp of the triggering event;
  neither is the duration in review.
- Primary action: Start review, View feedback, View discussion, View checks, or
  Open/View PR, according to the reason. Use a specific GitHub target when known,
  otherwise fall back to the PR. Read, snooze, and copy-link remain available.
- Detail: all outstanding reasons, friction contributors, evidence links and
  coverage, then the event timeline. Timeline truncation and metric coverage are
  separate facts and must be labelled separately.

Only make claims supported by data. A comment is not necessarily a question;
participation search membership does not establish “you commented.” Readiness to
merge stays provisional pending GitHub requirements and permissions.

## Review friction (pain level)

Use **Review friction** as the interface label. Present **Low**, **Moderate**, or
**High**, with explanatory facts rather than an opaque numeric score. This is a
measure of the PR's journey, not a judgment about its author or reviewers.

Initial candidate contributors:

| Signal | Intended meaning | Measurement constraints |
| --- | --- | --- |
| Review rounds | Feedback → author revision → subsequent review cycles | Count coherent cycles, not comments, reviewers, or individual review submissions; define how concurrent reviewers and re-requests are grouped. |
| Code churn during review | Rework after review began | Initial diff size is not churn. Capture revision history; net additions/deletions alone cannot measure repeated rework. Account for rebases, force pushes, and generated files. |
| Time in review | Time spent ready for review before completion | Exclude draft intervals. Separate waiting for review from active author work when evidence permits; unknown ownership remains unknown. |

Later candidates are repeated blockers (recurring CI failures, conflicts, or
changes requested) and stalled progress while a next step is outstanding. Avoid
double-counting correlated signals such as age and waiting duration. Comment
volume alone is not pain: productive discussion can be long.

Examples in the mockups include:

- High: 4 review rounds, 12 days in review, substantial rework.
- High while waiting: 3 rounds, 16 days in review, awaiting another review for 6 days.
- Low: first review round, 1 day in review, little rework.

These are illustrative assessments, not thresholds. Before implementation, choose
and version the cycle definition, churn units/normalization, duration rules,
combination policy, and level thresholds using representative histories. A
combination should allow prolonged waiting to matter even with little churn, and
should avoid equating large PRs with difficult reviews automatically.

### Missing history and lifecycle

- **Limited history**: insufficient coverage to confidently assign a level. Show
  the known facts and which signals are missing; never treat missing data as Low.
- **Not assessed**: review has not started, for example a never-ready draft.
  A PR returned to draft can retain earlier friction while its review clock pauses.
- **Historical review friction**: completed PRs retain the explanation at
  completion. Time stops accumulating when merged/closed. Reopening requires an
  explicit interval policy before implementation.
- Retain collection bounds and coverage with each assessment. Distinguish
  observed-from-first-capture measurements from complete GitHub history. A
  display that shows only a few timeline events does not itself imply the
  underlying metric history is incomplete.

## Ranking and state semantics

The default Tailored and Action order remains the documented priority bands,
newest activity first within each band. Default Following and Recent ordering
remains newest activity first; My PRs retains its proposed authored-action,
awaiting-review, and draft groupings.

Offer an explicit **Highest friction** alternate sort within the selected view:

1. Assessed PRs by level: High, Moderate, Low.
2. Within a level, longest time in review, then newest activity, then stable PR
   identity for deterministic ties. Unknown duration follows known duration.
3. Limited-history PRs, then not-assessed PRs, each newest-first with stable ties.

This alternate order crosses priority groups; hide those group headings while it
is selected. It changes ordering only, never view membership. Recent compares
historical assessments. The My PRs frame demonstrates the alternate ordering.

Friction alone does not create an attention-required state, notification, or
reactivation. Elapsed time and friction recalculation must not become meaningful
event fingerprints. Acknowledgement and snooze still use the newest meaningful
event fingerprint, and a changed fingerprint reactivates the PR. Notification
deduplication and the first-observation no-notification baseline still apply.

## Future implementation boundaries

- `crates/github`: collect and normalize the evidence needed for review cycles,
  ready/draft intervals, revision comparisons, and completion. Preserve bounded
  collection and explicit truncation. Current snapshots alone do not establish
  full review-cycle or cumulative churn history; perform a data-availability spike
  before defining schema/query changes.
- `crates/domain`: derive structured reasons, next actions, and friction
  assessments from normalized evidence. Suggested assessment fields include level
  (optional), assessment status, contributors with values/units and evidence IDs,
  coverage window, missing signals, computation time, and policy version. Reasons
  should carry reason codes and factual parameters suitable for native rendering.
- `crates/domain/src/ranking.rs`: add a `RankingStrategy` with proposed stable ID
  `highest-friction`. Order projected cards only; classification and view membership
  remain in projection. Clients select the strategy ID rather than sorting.
- `crates/state`: persist per-device acknowledgement, snooze, and attention
  observations in the separate local store. Any new historical observation cache
  requires an explicit storage/retention decision; it must not be conflated with
  notification deduplication or contain access tokens.
- Native clients: render supplied reasons, evidence, assessments, and explicit
  commands. No client-side GitHub queries, friction calculation, or ranking.

### Delivery and acceptance criteria

1. Inventory historical evidence, bounds, and coverage; choose measurement and
   storage policies. Resolve cycle boundaries, churn normalization, draft/reopen
   intervals, and thresholds before treating mock values as computed features.
2. Implement domain explanations independently of historical friction; a useful
   attention reason should not depend on a complete review history.
3. Add fixture-driven friction derivation and the alternate ranking strategy.
4. Connect the already-projected representation to native clients.

Verify meaningful scenarios: simultaneous feedback and failing checks; high
friction while waiting; long productive discussions without revision cycles;
large initial diffs with little rework; repeated revisions with little net diff;
draft pauses; incomplete capture history; completion and reopening; stable sort
ties; and repeated refreshes that change elapsed time without generating attention
events. Every displayed level must be explainable from the same evidence shown in
detail, and every card must reveal its relevance without expansion.
