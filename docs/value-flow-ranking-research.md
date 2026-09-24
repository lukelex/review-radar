# Ranking PRs for faster review-to-delivery flow

Status: handoff evidence collection and conservative response-episode projection
are implemented; **Flow first remains a research proposal, not a shipped ranking
policy**. Windows implementation is paused. The new evidence does not change the
default Tailored order or notification behavior.

## The decision to optimize

“Shortest path to big value” means **choose the next action the current user can
take that is most likely to advance valuable work through its next handoff for the
effort required**, while keeping older and uncertain work visible. A PR is not a
single job: it may cycle through request → review submission → author revision →
re-review, then wait for approval, checks, and merge. The shortest *next action*
is not necessarily the shortest *path to production*.

Optimize two separately observable outcomes:

1. **Handoff latency:** request → first submitted human review; substantive
   feedback → first subsequent author revision; revision → next submitted review
   (when a new review is actually requested). Report these separately, not as one
   “review time.”
2. **Delivery:** ready for review → merge, plus approval → merge, with completed
   PRs and unfinished PRs counted separately. Throughput and the age of outstanding
   work must remain visible; a lower mean achieved by ignoring hard PRs is failure.

Business impact is not currently collected. Initially, treat each qualified
handoff as equally valuable; “high value” requires an explicit, auditable source
(for example, a user's coarse impact declaration or a verified release dependency).
Do not infer business value from PR size, repo name, author, labels, comment count,
or accumulated **Review friction**. Friction is a description of past difficulty;
it is neither next-action cost nor delivery value. An impact declaration would be
a new shared state/command/presentation feature, not a field invented in a shell.

### A small flow model

```text
draft --ready--> ready --request--> awaiting reviewer
                                  --submitted review--> awaiting author (if feedback needs work)
                                  --approval--> checks/merge gate
awaiting author --revision + renewed request--> awaiting reviewer
checks/merge gate --merge--> Recent (history only)
```

This is a measurement model, not a replacement for current `Action` or workspace
membership. A reviewer can submit a review without requesting code changes;
comments need not demand a revision; pushes can occur without addressing feedback.
Multiple reviewers can overlap. Only claim a stage transition when the source
evidence supports it. Waiting on another person is not a new obligation for the
viewer. A critical path across dependencies is useful only when dependencies are
known, not guessed from a PR title.

## What the current product can actually order

- `crates/domain/src/ranking.rs` implements `tailored` (priority bands, descending
  `updatedAt`), `newest-activity` (descending `updatedAt`), and
  `highest-friction` (assessed difficulty first, then limited history). All sort
  already-projected cards with a stable node-ID tiebreaker. Default priority
  bands and view membership are specified in [the product plan](project-plan.md).
- `crates/domain/src/lib.rs` exposes action, relationship, attention-required,
  explanation, health, fingerprint, and review-friction assessment on each card.
  It does **not** expose current request age, local-action effort, business impact,
  a dependency graph, or a reliable head-push timestamp. A `review_requested`
  search membership establishes a **current** request, not when it began.
- `crates/github/src/hydrate.graphql` fetches bounded reviews with `submittedAt`,
  lifecycle transitions, and a commit tail with `committedDate`/parent-diff
  counters. It does not fetch review-request timeline events. Commit time is not
  push/hand-off time; `updatedAt` is any PR activity. `queue.rs` preserves history
  coverage and normalizes some review/revision evidence for **friction**, not for
  exact stage ownership or request-to-review latency. See the
  [review-friction spike](review-friction-data-spike.md).
- Authored `new-feedback` compares consecutive captures and retains outstanding
  local evidence; its first capture is a baseline. It should not be repurposed as
  a complete historical event stream. Local acknowledgement/snooze suppresses
  cards before display. Ranking never changes its fingerprints or notifications.

**Consequences:** a useful current baseline is the existing Tailored queue, plus
an offline *candidate* order of confidently actionable PRs. An effort/value ratio
using today's card fields would be false precision. Neither “oldest `updatedAt`”
nor “highest friction” is a measured review turnaround optimization.

## Scheduling algorithms: which objective each one serves

| Policy | Rule and what it optimizes | PR fit / failure mode |
| --- | --- | --- |
| Priority bands + newest activity (current default) | Obligations before waiting; recent within a band. Simple, explainable, no estimates. | Fresh minor updates can repeatedly bury an older request; recency is not a handoff clock. Retain as the comparison baseline. |
| FIFO / oldest *outstanding handoff* | Sort by verified request or feedback age. Gives old work service and protects the tail. | Needs a real stage anchor; ignores effort and impact. Oldest `updatedAt` is not FIFO. Useful as an aging guardrail. |
| SPT / shortest next-action effort | Ascending expected active minutes. Under a fixed, single-worker batch with known durations and equal weights, minimizes average completion time. | PR diff size is a poor stand-in for review minutes; repeatedly favoring easy jobs can starve complex, high-value reviews. |
| SRPT / shortest remaining processing time | Re-evaluate remaining work after preemption; minimizes mean response time in its idealized single-server setting with known job sizes. | Human reviews are not freely preemptible; context switching and multi-person handoffs violate its assumptions. Not a UI comparator for PRs. |
| EDD / earliest due date | Protects deadlines/max lateness under suitable single-machine assumptions. | Use only for explicit real deadlines; a guessed date encourages deadline gaming and ignores value without further policy. |
| WSPT / Smith's ratio | Descending `weight / effort`; exactly minimizes weighted sum of completion times for independent, known-size jobs on one non-preemptive worker all available at the start. | Best *interpretation* for quick valuable handoffs, **not** a theorem for multi-reviewer, recurring, stochastic PR cycles. Needs credible impact and effort inputs. |
| Cost-of-delay / effort (WSJF-style heuristic) | Descending marginal delay cost divided by next-action effort. | Better when value decays with time, e.g. a verified release gate; cost of delay must be measured or declared, not inferred from age alone. |
| Critical-path / dependency-first | Advance an action that releases downstream work. | Needs a verified dependency graph and who can act next; a high-fan-out dependency is not necessarily ready for review. |
| Contextual prediction / bandit | Predict effort, unblock probability, and response under a policy; explore vs. exploit. | Sparse, selected outcomes and policy feedback create bias; opaque scores are unjustified before logging and an offline baseline. A later experiment, not v1. |

For two independent jobs `i, j`, doing `i` first costs
`w_i * p_i + w_j * (p_i + p_j)` in weighted completion time; doing `j` first
reverses them. Choose `i` first exactly when `w_i / p_i >= w_j / p_j`.
Illustration (invented numbers, minutes and *declared* impact units): a 10-minute
review worth 20 units, a 60-minute review worth 30, and a 5-minute review worth 1
sort as 10 → 60 → 5 by WSPT, versus 5 → 10 → 60 by SPT. Weighted completion totals
are `20×10 + 30×70 + 1×75 = 2,375` vs.
`1×5 + 20×15 + 30×75 = 2,555` impact-minutes. The example establishes the
tradeoff, not PR-delivery gains: another person may still take days to revise.

**Recommendation:** eventually offer an opt-in, shared-core **Flow first** order:
a guarded WSPT-like *next-handoff* heuristic among actionable PRs, with a
verified-handoff-age guardrail and a transparent fallback for missing data. Keep
Tailored as default until a measured improvement and cross-interface parity are
demonstrated. Do not substitute Highest friction for the new policy.

## Evidence needed for turnaround measurements

Create a normalized **handoff episode** per request/re-review, keyed by PR ID,
requester/reviewer where known, and source event ID. Store source time *or* an
observed interval `(last capture without state, first capture with state]`, not a
made-up timestamp. Each episode records stage, responsible role when known,
start/end evidence IDs, observation cutoff, coverage (complete/partial), and why
it ended/censored. Merge/close, reviewer removal, conversion to draft, re-request,
reopen, and replacement of a head SHA must be explicit transitions. Bounded
connections and the five-minute refresh imply left-truncated and interval-censored
observations; an initial capture is not a zero-minute request.

| Clock / endpoint | Candidate evidence | Important qualification |
| --- | --- | --- |
| Request → review submission | Review-requested/removed timeline events (subject and team as available), current request membership, review `submittedAt`. | Validate API fields and team vs. individual matching in a bounded spike. Review decisions can remove a request; re-requests start new episodes. `ready` ≠ requested. |
| Substantive review → next revision | Non-self, non-bot submitted review `submittedAt` with state, followed by an author head change first observed in a capture. | A submitted `COMMENTED` review need not require revision; `CHANGES_REQUESTED` is clearer but may be stale. Author revision is not proof feedback was addressed. Head SHA first-observed interval is safer than `committedDate`. |
| Revision/re-request → next review | Changed head SHA, renewed request where observable, next submitted review. | No re-request means do not label waiting as reviewer latency. Concurrent reviewers need separate clocks or an explicit first-response aggregation. |
| Approval → merge | Review submission/decision and `mergedAt`, considering later dismissals/revisions. | Merges depend on checks, permission, and other actors; do not blame reviewers for this interval. |

The collector requests bounded review-requested, review-request-removed, and
head-force-pushed events in a separate `handoffItems` timeline connection; raw
payloads retain the source evidence and its independent pagination flag. The queue
normalizer emits optional per-card `handoffHistory` with exact-user
request→submitted-review episodes, removed/pending/inconclusive outcomes, pending
wall-clock age, and separate force-push OIDs. Response and pending durations are
suppressed unless both request-event and review connections are complete and all
evidence is valid. Team requests are retained but not matched to individual
reviewers without an attribution source. This evidence does not drive ranking or
attention.

After rebuilding the collector image, a bounded live query passed validation at 9
points for 3 hydrated PRs (20 event limit; 13,351 response bytes) and returned
request, request-removal, and force-push events with no handoff pagination gaps.
A direct, one-PR `reviewRequests(first: 100)` lookup returned one user reviewer at
cost 1. This verifies user event/current-state identity paths, but not
team/mannequin attribution or broad-history coverage. Do not build a ranking input
from this small sample. Next validate representative user/team request histories,
add capture-interval evidence for ordinary head changes first observed between
polls, and calibrate effort separately from elapsed waiting. Preserve source
evidence and explicit retention/versioning separately from per-device attention
state. Never use commit-author time as an exact push time or copy private captures
into fixtures.

## Candidate ranking contract (after evidence validation)

Policy ID proposal: `flow-first-v1`. Define its versioned features in the Rust
domain, from the same successful capture and normalized episodes for all clients:

- **Eligible next step:** an actual `attention_required` action owned by the
  viewer (review, reply/revise, or investigate failed checks/merge blockers). A
  waiting/following PR may be valuable but cannot jump ahead merely because its
  historical friction is high. Merged PRs appear only in Recent during its 14-day
  window, not Tailored; closed PRs retain their existing membership. Ranking
  cannot create an obligation.
- **Weight:** initially one unit per qualified handoff. After a separate shared
  impact command exists, a small, documented, capped weight from an explicit
  declaration and *verified* downstream dependency can be tried. Show the input
  and its provenance; absent impact is neutral/unknown, not “low value.” Never
  optimize review *approval* rate or reward perfunctory reviews.
- **Effort:** estimated active minutes for *the viewer's next step*, not total PR
  age, time waiting on others, or net diff lines. Request-to-review wall time does
  not measure active review minutes. First obtain consented effort labels (e.g.
  voluntary coarse estimates or opt-in work sessions) and validate review-scope
  features; then try coarse calibrated buckets by action and review scope, with
  cohort medians, missingness, and uncertainty rather than per-author performance
  judgments. Require a minimum effort floor to avoid division by zero. The
  current capture has no review-work-duration labels, so no live effort score
  should be synthesized from the existing timestamps.
- **Progress probability (later):** the calibrated probability that this action
  advances a *valid* stage, rather than a claim that it will merge the PR. Until
  measured, omit it rather than inventing a multiplier.
- **Age guardrail:** compare time since the *same outstanding handoff* to a
  versioned, calibrated stage-specific threshold. Promote overdue actions ahead
  of ordinary ratio ordering, oldest verified/observed first. Capture-only ages
  are labelled “observed since,” not exact request age. Unknown cost/impact actions
  retain a neutral fallback within the actionable lane and become eligible for
  age promotion; missing evidence must never turn into zero value.

Proposed order: (1) actionable actions beyond the calibrated wait guardrail,
oldest verified/observed handoff first; (2) ordinary actionable actions with
defensible inputs by descending `weight / max(effort, floor)`; (3) ordinary
actionable actions with unknown inputs by current priority band and newest
activity; (4) waiting/for-awareness items by current Tailored order. Use stable PR
ID for ties; comparison uses fixed-point/rational arithmetic and a single pinned
`as_of` pinned to the successful capture's timestamp. Calibrate the guardrail and
unknown placement
against starvation **before** enabling: a steady stream of overdue items can still
overwhelm any static sort. In such overload, disclose the queue's aging backlog
instead of promising a finite wait. Consider an explicit fair-share lane in v2.

This is a **heuristic**, not a proven optimal PR scheduler. It can cross current
review-requested and authored-action priority bands **only when explicitly
selected**. It never changes classification, workspace membership, local
acknowledgement/snooze, or notification eligibility. The `RankingStrategy` in
`crates/domain/src/ranking.rs` orders projected cards, selected by stable ID through
the queue; extract/version inputs during projection, not inside a sorting
comparator or native shell. `O(E + n log n)` normalization-plus-ranking per
successful capture is adequate; no per-comparison network or database work.
Re-ranking due to elapsed time must not generate attention events. Failed refresh
keeps the last successful order labelled cached instead of aging it as fresh data.

## Validation and staged delivery

1. **Measure first.** Build fixture-driven episode reconstruction from bounded
   captured evidence, with known gaps and source/capture bounds. Report request →
   review, review → revision, renewed request → re-review, approval → merge,
   p50/p75/p90, unfinished-work age, coverage, and throughput, split by
   actionable stage and repository where samples permit. Report wall-clock time;
   business-hours time may be a secondary metric. Current generalized friction
   fixtures cannot validate these turnaround clocks.
2. **Compare policies offline.** Replay sanitized chronological captures with
   Tailored, oldest verified handoff, SPT estimate, and guarded ratio candidate.
   Never look ahead past replay time; include open/censored PRs, unknown inputs,
   truncated histories, team requests, repeated cycles, large PRs, and drafts.
   Track rank stability and age of the oldest item. Offline replay is diagnostic:
   historical users did not obey the candidate order, so replay cannot establish
   a causal improvement.
3. **Shadow and calibrate.** Log versioned rank inputs/reasons and missingness in
   aggregate/sanitized diagnostics (never token or private PR text). Choose effort
   cohorts, impact semantics, service thresholds, and a meaningful representative
   baseline before setting a target. Avoid per-developer speed comparisons.
4. **Opt-in experiment.** Expose `flow-first-v1` through the queue CLI and every
   compatible native client with identical rank meaning and an explanation such as
   “Review requested · observed waiting 18 h · short review estimate (limited
   evidence).” Run a consented, controlled trial where possible. Evaluate p75
   request→review and review→revision, p90 tail, ready→merge, throughput, missed
   obligations, override/undo, and rank churn together; no degraded tail or
   hidden high-impact work in exchange for more quick completions. Pick numeric
   success thresholds from baseline and sample size, not this document's example.
5. **Promote deliberately.** Change the default only with measured evidence that
   turnaround and delivery improve, an explainable unknown-data behavior, and a
   new product decision updating `project-plan.md` and the native contract. Retain
   the existing sort IDs and a way back to Tailored.

Fixture acceptance includes two review requests with different recipients; one
unanswered request; review then revision then re-request; approval without a
revision; draft/closed/reopened cases; missing/truncated history; head SHA changes
with backdated commit time; missing effort/impact; aging under sustained short-job
arrival; stable ties; and a high-friction waiting item below actionable work. Sort
changes must preserve view membership and local attention outcomes. Validate all
workspace projections; Windows remains paused, so track its explicit parity gap
and follow-up before shipping a shared UI feature.

## Research basis and limits

- [Lawler et al., *Weighted sum of completion times*, §4.1](https://web.mit.edu/schulz/www/epapers/lqss.pdf): Smith's ratio and its exact single-machine, independent-job objective. This theorem does not cover cyclic PR workflows.
- [Scully et al., *Simple Near-Optimal Scheduling for the M/G/1*](https://arxiv.org/abs/1907.10792): known-size SRPT vs. unknown-size policies in a queueing model; human review is not that model.
- [Sadowski et al., *Modern Code Review: A Case Study at Google*](https://research.google/pubs/modern-code-review-a-case-study-at-google/): real review latency and change-size observations in one organization, not Review Radar's baseline.
- [Kudrjavets et al., *Mining Code Review Data to Understand Waiting Times Between Acceptance and Merging*](https://arxiv.org/abs/2203.05048): independent downstream merge delay matters alongside the first response; results from Gerrit/Phabricator do not establish local effect sizes.
- [The Kanban Guide, flow metrics](https://kanbanguides.org/english/): WIP, throughput, work-item age, and cycle time make tail behavior observable alongside averages. Reducing WIP is an operating practice, not something sorting alone enforces.
