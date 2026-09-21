# Review-friction history spike

## Verified evidence

Inspected `crates/github/src/query.graphql`, the SQLite snapshot reconstruction,
and `tests/fixtures/github/github-snapshot.json`. This spike uses the generalized
fixture, not private captures or new GitHub requests.

The fixture contains 97 PRs: 33 open, 62 merged, 2 closed, including 5 drafts.
All 97 have created/updated timestamps and exactly one fetched commit. All 64
completed PRs have `closedAt`; the 62 merged PRs also have `mergedAt`.

| Source | Observed evidence | What it cannot establish |
| --- | --- | --- |
| Reviews | 483 fetched nodes; 3 PRs have `hasPreviousPage` | Revision/re-review cycles; even complete reviews lack intervening revision history |
| Comments | 182 nodes; no truncated top-level comment connections | A comment's semantic intent or whether discussion was painful |
| Review threads | 398 nodes; 7 PRs have truncated thread connections | Complete conversation history; nested replies have independent bounds |
| Commits | Latest SHA/date and current check rollup only | Cumulative churn, rebases, intermediate failures, author revision boundaries |
| Lifecycle | Current draft state and completion timestamps | Ready/draft intervals, initial draft state, reopened intervals |
| Diff | No additions, deletions, or changed-file counts | Initial diff size or repeated rework |

There are no `timelineItems` in the query or fixture. `createdAt` is not review
start, `updatedAt` is not a revision boundary, review count is not round count,
and a draft today is not proof it has never been reviewed. Current domain event
connections also discard completeness metadata; the raw stored payload retains
it and must be used by any future history normalizer.

**Conclusion:** existing captures cannot justify a Low/Moderate/High assessment.
Snapshot-only projection must report Limited history for all PRs, including drafts.
It must not silently substitute PR age, net diff size, or comment volume.

## Normalized history contract for the domain implementation

Provide an optional per-PR history alongside snapshot PRs, keyed by PR node ID.
Use integer UTC epoch seconds for computation, explicit observation start/end,
initial draft state, coverage (`complete` or `partial`), baseline diff lines, and
ordered evidence events with stable source IDs. The domain validates time bounds,
unique evidence IDs, and lifecycle/draft consistency. Equal timestamps retain the
normalizer's source order. Partial or inconsistent input never yields a level.

Events are:

- `ready` and `draft`: changes of review readiness;
- `review`: submitted substantive human review feedback, excluding author/self,
  bots, and pending reviews (filtering is the normalizer's responsibility);
- `revision`: a meaningful code revision with optional changed-line count;
- `closed`: terminal merged/closed event, freezing duration;
- `reopened`: unsupported in policy v1; report Limited history until an explicit
  episode policy is implemented.

Complete coverage means all required timeline, review, and revision evidence from
creation through the observation cutoff is accounted for. It is not equivalent to
one API connection returning `hasPreviousPage = false`. Completion evidence must
agree with the current PR lifecycle. A partial history can retain observed facts,
but its values must be labelled partial and cannot be ranked as measured levels.

## Experimental policy `review-friction-v1`

These are explicit, versioned starting heuristics, exercised against synthetic
journeys rather than claimed as calibrated against the generalized capture.

### Measurements

- **Rounds:** the first substantive review starts round 1. Only a revision after
  feedback followed by another substantive review increments the round. Multiple
  reviewers or many reviews before a revision remain one round. Draft-period events
  do not themselves add rounds.
- **Review seconds:** sum ready, open intervals through the observation cutoff;
  pause during draft and freeze at completion. An initially ready PR starts at
  creation; an initially draft PR starts on its first ready event. This includes
  both author work and reviewer waiting; v1 does not claim to distinguish them.
- **Rework lines:** sum additions plus deletions for successive meaningful
  revisions while ready, after the first substantive review. Never sum every
  cumulative PR diff. Normalize against initial review diff lines (minimum 1).
  Initial diff size contributes nothing by itself. Missing revision measurements,
  invalid rebases, or missing baseline make churn unknown, not zero.

### Combining signals

| Signal | Moderate | High |
| --- | --- | --- |
| Review rounds | 2 | 4 |
| Ready/open duration | 5 days | 14 days |
| Rework / initial review diff | 100% | 300% |

Overall High if any signal is High or at least two are Moderate; Moderate if one
is Moderate; otherwise Low. A prolonged wait can therefore be High without high
churn. The contributors, raw values/units, evidence IDs, and policy version are
exposed so the combination is inspectable. No numeric pain score is displayed.
Duration and separate waiting time are not counted twice.

Complete never-ready histories are Not assessed. Missing/partial/invalid histories
and unknown churn are Limited history with no level. Complete histories that
returned to draft retain accrued friction. Completed histories retain historical
friction without accruing more time. Age/recalculation never changes attention
fingerprints or creates notification eligibility.

## Collection follow-through

1. Prototype targeted, bounded history retrieval: creation state, ready/draft and
   close/reopen transitions, reviews, and revision head comparisons. Preserve
   pagination gaps, source IDs, and capture cutoffs; validate GitHub API capability
   and costs before changing the main query.
2. Establish a baseline diff at initial review readiness and compare successive
   meaningful revisions. Treat unresolvable force-push/rebase comparisons as
   missing coverage. Generated/vendor-file handling requires a normalization
   policy before using the experimental thresholds in production.
3. Keep source evidence in the GitHub capture store, separate from local
   acknowledgement/snooze state. Retain immutable evidence sufficient to reproduce
   an assessment; make retention and migration explicit in the collection change.
4. Pass normalized histories to domain projection. Existing queue captures omit
   them and will honestly remain Limited history until this collection work lands.
5. Calibrate against real, generalized histories; then version any policy changes.

See [the concept](attention-and-review-friction.md) for sort and presentation
semantics. Native shells consume the projection rather than calculating metrics.

## SimplerQMS calibration traversal

The first bounded calibration traversal covers the 100 most recently merged PRs
from `SimplerQMS/SimplerQMS` within the requested six-month window. Raw responses
remain in local temporary storage only. The commit-safe generalized result is
`tests/fixtures/domain/simplerqms-merged-calibration.json`.

The traversal found 99 PRs with at least one review and 59 with both a review and
more than one commit (median: 2 reviews and 2 commits; maxima: 20 reviews and 24
commits). It is sufficient to exercise review-round and elapsed-time analysis, but
the GitHub CLI response supplies aggregate PR additions/deletions rather than
first-parent deltas per commit. It therefore cannot calibrate rework ratios or
promote the experimental Low/Moderate/High thresholds. The fixture deliberately
omits raw repository, PR, actor, commit, author, URL, and message identities.

## Implemented bounded collection

The collector now preserves the latest bounded ready-for-review, draft, close, and
reopen timeline entries; bounded reviews; and a bounded commit tail, each with its
`hasPreviousPage` flag. The queue normalizes these payload fields into optional
`reviewHistories` before asking the domain to assess a card. It excludes self,
bot, and pending reviews from review-round evidence and retains stable source IDs.
Older captures that predate these fields simply have no normalized history.

The normalizer never derives churn from cumulative commit totals. It supplies no
baseline diff and no changed-line value, so even otherwise complete bounded
evidence produces `Limited history` with `unknown-code-churn`. Any pagination,
invalid field, or event inconsistency becomes partial coverage. This makes live
projections coverage-aware today without claiming a calibrated pain level. Targeted
parent-diff comparisons and calibration remain required before production levels.

First-parent additions/deletions are now collected for every bounded commit. When
the entire commit connection is present, every relevant commit has a parent diff,
and a substantive review is observed, the normalizer uses the sum of pre-review
commit deltas as an **initial-review change-volume** baseline. It sums post-review
parent deltas as rework. This is explicitly not a net pull-request diff: repeated
edits remain visible. Missing parent/counter evidence leaves the baseline or
rework unknown. Threshold calibration against representative histories remains
required before presenting these experimental measurements as production truth.
