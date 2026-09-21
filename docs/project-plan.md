# Project plan

## Product objective

Build an actionable GitHub pull-request workspace that keeps a developer current
on review obligations, feedback on authored pull requests, and PR health.

The default view, **Tailored to you**, prioritises:

1. Pull requests where a review is requested from the user.
2. Authored pull requests that need action: changes requested, new reviewer
   activity, failing checks, conflicts, or readiness to merge.
3. Authored pull requests awaiting review.
4. Followed pull requests with recent activity.
5. Pull requests closed or merged in the previous 14 days.

Items are newest-first within each priority band.

## User experience

### Views

- **Tailored to you**: the default ranked action queue.
- **Action**: attention-required pull requests only.
- **My PRs**: authored pull request health.
- **Following**: involved pull requests not authored by the user.
- **Recent**: closed and merged pull requests from the previous 14 days.

### Pull-request parents and events

Each pull request is a parent card with repository, number, title, lifecycle
state, concise health, latest event, and relative age. Expanding the card reveals
ordered child events: review requests, reviews, comments, CI changes, and
mergeability changes.

Every card prominently explains **why this needs your attention**, or **why this
is here** for waiting and informational items, with a matching next action.
**Review friction** expresses accumulated difficulty through review rounds,
rework, and time in review, with visible contributors and explicit missing-history
states. An opt-in **Highest friction** sort complements the default ordering.
See [attention explanations and review friction](attention-and-review-friction.md)
for the design, measurement decisions, and future implementation criteria.

### Actions

Each pull request supports:

- Open in browser.
- Start review in GitHub's **Files changed** view.
- Mark read.
- Snooze until later today, tomorrow, or next week.
- Copy link.

Acknowledgement and snooze are local-device state. A new meaningful event
reactivates the pull request.

## Scope

Version one supports the authenticated `github.com` account only. It refreshes
when opened and every five minutes while running. Desktop notifications are sent
only when a pull request newly enters an attention-required state.

## Delivery phases

### Phase 1: GitHub data spike

Define bounded queries and fixture snapshots for review requests, authored PRs,
involved PRs, reviews/comments, checks, mergeability, and recent completions.

**Exit:** snapshot data accurately classifies real PRs.

### Phase 2: Rust domain core

Implement the domain model, ranking, event fingerprinting, acknowledgement,
snoozing, and attention transition detection without UI dependencies.

**Exit:** local state survives restart and new activity reactivates acknowledged
or snoozed PRs.

### Phase 3: Linux client

Implement the Qt/QML dashboard, parent/event timeline, filters, actions, and
Quickshell bar integration.

**Exit:** the default view identifies the next action without expanding a card.

### Phase 4: Native notifications

Connect attention transitions to Linux notifications with deep links to the
dashboard or browser. Prevent duplicate alerts across polling cycles.

**Exit:** one new attention event produces one notification and one dashboard
entry.

### Phase 5: macOS and Windows clients

Build SwiftUI and WinUI 3 shells against the same core contracts and fixtures.

**Exit:** all platforms present equivalent ranking and state without duplicating
GitHub business logic.

### Phase 6: Hardening

Add rate-limit behavior, offline/stale states, migration handling, accessibility,
and fixture-driven integration tests.

## Success criteria

Within seconds of opening the dashboard, the user can identify every review they
owe, every authored PR needing a response, and the next browser destination for
each actionable item.
