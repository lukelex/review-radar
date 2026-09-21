# Native client and performance contract

This is the long-lived, platform-neutral contract for Review Radar clients.
Linux Qt/QML is the reference implementation, but macOS SwiftUI, Windows WinUI,
and future TUI clients must reproduce the behavior and information hierarchy
described here without copying toolkit-specific code or pixels.

## Maintenance rule

Any change to collection performance, projection performance, loading behavior,
UX/UI behavior, terminology, or an architectural decision must update this
document in the same change. Record:

1. the user-visible or measurable behavior;
2. the reason for the change and important tradeoffs;
3. platform-neutral acceptance criteria; and
4. the implementation reference, if one exists.

The repository's `AGENTS.md` makes this a persistent instruction for coding
agents. The design tokens and language remain in [`design-system.md`](design-system.md);
this document records cross-platform behavior and decisions rather than replacing
that visual source of truth.

## Collection and projection performance

The collector must preserve one coherent successful capture. A client may show
the previous successful capture while a new capture runs, but must not combine
partially classified cards into a new ranked workspace.

Current behavior:

- Independent search categories run concurrently while pagination within each
  category remains sequential. When the known remaining rate-limit budget is too
  low for the category fan-out, the collector uses a sequential fallback.
- Search memberships are collected with lightweight queries containing PR IDs and
  `updatedAt` values.
- PR IDs are deduplicated across all search memberships before hydration.
- Full nested PR data is hydrated once per unique new or changed PR.
- The latest stored payload is reused when `updatedAt` is unchanged.
- If changed-PR hydration fails and a previous payload exists, that payload may be
  used as stale fallback. It is marked with `_reviewRadar.hydrationStale` and
  must never be presented as a successful fresh update.
- A PR with no cached payload cannot silently enter a capture without successful
  hydration.
- The predecessor snapshot is used for feedback comparison without repeating
  review-history normalization that is only needed for the current projection.

The collector reports cache hits, hydration count, request duration, slowest
request, response bytes, and GraphQL cost. These measurements are diagnostic;
they must not change ranking or attention semantics.

During refresh it also emits progress for each search/page and hydration batch.
Native clients should surface that as concise progress copy (for example,
“Searching 3 of 6 · page 2 of 4” or “Hydrating batch 4 of 9”), while retaining
the last successful projection when one exists.

Resource use is bounded at the client runtime boundary. A packaged client must
not allow its refresh helpers to grow without limit: helper work is serialized,
refreshes are cadence-limited, and the runtime may apply a shared CPU, memory,
and process-count budget to the shell and helpers together. A budget breach may
delay or fail a refresh, but must leave the last usable projection available and
must not produce a partial projection.

### Cross-platform acceptance criteria

- Refreshing does not blank a usable previous workspace.
- A failed refresh leaves the last successful projection available and clearly
  identifies cached/stale data.
- Identical PRs appearing in multiple memberships do not cause repeated detail
  hydration in one capture.
- Unchanged PRs are not fully rehydrated on every refresh.
- No client treats stale fallback data as current data.
- All clients consume the same ranked projection and explicit command surface.
- A client remains responsive under its configured runtime budget; resource
  exhaustion cannot create an unbounded helper-process or refresh loop.

## Loading and refresh states

Clients distinguish these states:

1. **Local loading:** no projection is available yet; show a waiting state or
   skeletons.
2. **Cached while syncing:** a previous projection is usable; keep it visible,
   disclose that fresh data is being fetched, and show modest updating feedback.
3. **Fresh local workspace:** the latest successful capture is displayed.
4. **Cached after failure:** retain the projection and explain that refresh failed.

The latest successful capture time must remain distinct from ongoing work or an
error. An idle process is not evidence that data is current.

The reference shell uses skeleton parent cards only when no cached list exists,
keeps cached cards visible during sync, shows an updating treatment on cached
cards, and defers the activity timeline briefly after opening details. This is
the safe form of view-targeted optimization: the shared snapshot remains
complete, while expensive detail rendering is delayed until it is useful. Other
clients may use native equivalents, but must preserve the same state meanings.

## Information hierarchy and interaction

Every PR card communicates, in order:

1. repository, PR number, and activity age;
2. title;
3. personal explanation and evidence-backed reasons;
4. health, lifecycle, and review friction as separate signals;
5. the matching next action and a way to inspect details.

Details expose the explanation, health, friction evidence and limitations,
bounded captured activity, and browser/read/snooze/copy actions. Expanding a card
must not change its classification or ranking. A client may defer rendering
activity details, but must not fabricate a partial explanation.

Selection is identified by PR ID across refreshes. If the PR remains in the
rendered dataset, its selection is retained and its content is updated; if it
leaves the dataset, selection is cleared.

Desktop clients provide basic Vim-style keyboard navigation when a search field
is not being edited: `j` moves to the next matching PR, `k` moves to the previous
matching PR, `l` opens details for the highlighted PR, and `h` closes details.
While details are open, `j` and `k` move between PRs without leaving the detail
context. The highlighted item must remain visible and the shortcuts must not
intercept text input.

## Decisions and tradeoffs

### Stable Linux application-data directory

The Linux client stores its capture and per-device state files below
`$XDG_DATA_HOME/review-radar/`, falling back to `~/.local/share/review-radar/`.
It must not derive the directory from a duplicated organization/application name
such as `Review Radar/Review Radar`. This keeps native launches, packaged builds,
and helper processes on the same local-state contract without changing the
cross-platform freedom to use each platform's standard data location.

Acceptance criteria:

- A normal Linux launch uses exactly one `review-radar` directory below the XDG
  data root.
- Capture and state databases remain separate files in that directory.
- Existing override environment variables continue to take precedence.
- The path contains no GitHub token or other credential.

Implementation reference: `QueueController::applicationDataFile` in
`apps/linux-qt/queuecontroller.cpp`.

### Atomic projections over progressive classification

Progressive visual loading is allowed, but progressive domain classification is
not. Ranking, attention explanations, notifications, and local-state fingerprints
must be derived from a complete successful projection. This prevents a client
from showing an explanation or notification that changes when late PR details
arrive.

### Cached data is useful but not fresh

Showing cached data is preferred to an empty workspace, especially during a slow
network refresh. Every client must distinguish cached/stale, loading, fresh, and
failed states in text or an equivalent accessible treatment; color alone is not
enough.

### Shared semantics, native presentation

Native clients share view IDs, ranking, card meaning, action commands, and state
semantics. They may use platform-native menus, typography, notifications,
navigation, and loading treatments. They must not move GitHub queries,
classification, ranking, notification deduplication, or local persistence into
the UI layer.

## References

- [`architecture.md`](architecture.md) — shared-core and client boundaries.
- [`project-plan.md`](project-plan.md) — product views, ranking, and refresh
  cadence.
- [`design-system.md`](design-system.md) — tokens, language, accessibility, and
  visual hierarchy.
- [`github-data-spike.md`](github-data-spike.md) — bounded query and truncation
  constraints.
