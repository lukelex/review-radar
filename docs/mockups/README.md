# Low-fidelity mockups

Static design proposals, using illustrative data. Each screen is available as a
PNG and an editable SVG of the same name. Counts describe the full view; list
screens show only a visible subset. These are proposed UI behaviors, not a record
of implemented functionality.

## 1. Tailored to you — what should I do next?

Explicit priority-group headings explain the ranking. Cards separate the reason
for attention from health indicators so multiple problems stay visible together.
The remaining following and recent groups continue below the visible area.

![Tailored queue wireframe](01-tailored.png)

## 2. Action — what needs my attention?

Use the same cards, filtered to outstanding attention. Keep review obligations
distinct from action on authored PRs. Local read/snooze actions are available from
the selected PR, and snoozed items remain discoverable in the sidebar.

![Action view wireframe](02-action.png)

## 3. My PRs — how healthy is my work?

A denser table makes it easier to compare review, checks, and mergeability across
authored PRs. Unknown and missing signals are explicit. Health remains visible
even after an item has been acknowledged locally.

![Authored PR health wireframe](03-my-prs.png)

## 4. Following — what is happening around my work?

Explain participation separately from urgency. This view can overlap Action when
a followed PR requests a review; view counts are not additive. Detailed reasons
such as “you commented” would require richer participation data than the current
collector's search memberships.

![Following view wireframe](04-following.png)

## 5. Recent — what finished?

Show the last 14 days of completions with an explicit merged/closed distinction.
Lifecycle takes precedence over old attention labels. Keep discussion history
available from the same detail view.

![Recent completions wireframe](05-recent.png)

## 6. PR detail — what changed, and what can I do?

A split view retains queue context while showing all health signals, actions, and
a newest-first event timeline. Explicitly label incomplete history. CI transition
events shown here are a design target; the collector currently stores snapshots.
For review obligations, add “Start review” linking to the PR's Files changed page.

![PR detail wireframe](06-detail.png)

## Shared representation

- Sidebar: stable view names and scoped counts.
- Header: search, last successful refresh, manual refresh, active account.
- PR identity: repository/number, lifecycle, title.
- Attention: why this item needs action; multiple health indicators alongside it.
- Age: latest activity time, independent of the reason for attention.
- Detail: consistent browser, mark-read, snooze, and copy-link actions.
- Ordering: the documented priority bands for Tailored and Action; newest activity
  first within each band. Following and Recent use newest activity first. My PRs
  uses authored-action, awaiting-review, and draft groupings as a design proposal.

The frames target the standalone desktop workspace. A Quickshell bar entry would
launch this workspace and expose its attention count.
