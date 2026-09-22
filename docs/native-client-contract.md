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

The queue executable writes exactly one JSON response to stdout. All tracing
and diagnostic output goes to stderr, including when `RUST_LOG` is enabled.
Native clients parse the complete stdout stream as JSON; mixing logs into that
stream causes a queue-response parse failure. This boundary is covered by the
`queue_output` executable integration test.

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

Hydration batches start conservatively and adapt within one capture from the
observed request cost, latency, and remaining rate-limit budget. The adaptation
is bounded by a minimum and maximum batch size and reduces the next batch after
slow, expensive, or rate-limit-constrained requests. A failed or timed-out
request retains the existing stale-fallback behavior; adaptive sizing must not
create a partial projection.

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

### Health status presentation

Clients render Review, Checks, and Merge as separate status values rather than
one undifferentiated health sentence. Labels remain explicit, while the value
uses a semantic positive, caution, negative, or neutral treatment and a text
symbol. Color is supplemental and must not be the only indication of meaning.
The shared projection remains the source of the structured health fields; shells
may choose platform-native chip or badge layouts.

Review friction uses a separate semantic low/medium/high/unknown color scale.
It communicates accumulated review difficulty, not urgency or PR health. Clients
must retain the explicit friction label, level, and a non-color cue; insufficient
history remains unknown rather than low.

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

Browser actions must open the user's host browser, including when the client is
running in a container. On Linux Docker sessions, the reference client uses the
host session's `org.freedesktop.portal.OpenURI` portal and falls back to the
native desktop URL launcher when no portal is available. Other clients should
use their platform's equivalent host/default-browser mechanism.

Selection is identified by PR ID across refreshes. If the PR remains in the
rendered dataset, its selection is retained and its content is updated; if it
leaves the dataset, selection is cleared.

Desktop clients provide basic Vim-style keyboard navigation when a search field
is not being edited: `j` moves to the next matching PR, `k` moves to the previous
matching PR, `l` opens details for the highlighted PR, `o` opens the selected PR's
canonical URL in the host browser, `a` asks for confirmation before acknowledging
the selected PR, `s` opens snooze choices, `y` copies its canonical URL, `/` focuses
search, and `?` shows shortcut help. `h` closes details. `Esc` closes details if
open, clears the search field, and returns focus to the workspace. While details are open,
`j` and `k` move between PRs without leaving the detail context. `Ctrl+N` cycles
to the next workspace, wrapping at the end. The highlighted item must remain
visible and the shortcuts must not intercept text input.

The five primary workspaces are directly addressable with `Ctrl+1` through
`Ctrl+5`, in the documented order: Tailored, Action, My PRs, Following, and
Recent. `Ctrl+D` pages the PR list down by most of a viewport and `Ctrl+U` pages
it up by the same amount. These paging shortcuts must preserve the current
selection and must not intercept search-field editing.

While the Control modifier is held, desktop clients reveal subtle numeric hints
beside the five workspace navigation items. The hints are discoverability aids,
not a replacement for accessible names or the shortcut guide, and disappear on
modifier release or window deactivation. Modifier state must be observed at the
native-shell level so the hints remain reliable while focus is in search or
another child control.

## Decisions and tradeoffs

### Shared modal presentation

Shortcuts and confirmations share a centered, viewport-bounded modal with a
readable title, scrolling body, and persistent footer. This keeps longer help
content usable on smaller windows. Escape dismisses; confirmations initially
focus Cancel and capture their target when opened so refresh cannot redirect
the action to a different PR. Shortcuts use grouped keycaps and Done.
Future preferences reuse the same hierarchy with grouped settings and explicit
Save/Cancel for draft edits; no preferences screen is implemented yet.
Acceptance: header and actions stay reachable at the minimum window size,
confirmation cancellation causes no state mutation, and all variants use shared
design tokens. Linux reference: `apps/linux-qt/qml/RadarModal.qml`.

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

### Relocatable Linux release bundles

The Linux release is published as a relocatable x86_64 archive containing
the native shell, Rust helper executables, the matching Qt runtime/plugins/QML
modules, and a launcher that resolves its bundle root at runtime. The archive
must not embed credentials or depend on the build machine's absolute path. It
may rely on the host's compatible graphics, font, display, and libc stack.

Acceptance criteria:

- Extracting the archive to a different directory does not require rebuilding
  or editing the launcher.
- The launcher uses the bundled Qt ABI rather than an arbitrary host Qt minor
  version, while leaving host system ABI and graphics libraries host-provided.
- Every published archive has a SHA-256 companion and stable/prerelease status
  matching its version.
- The release documents Linux x86_64 and graphical-session requirements.

Implementation reference: `scripts/package-release` and
`.github/workflows/release.yml`.

## References

- [`architecture.md`](architecture.md) — shared-core and client boundaries.
- [`project-plan.md`](project-plan.md) — product views, ranking, and refresh
  cadence.
- [`design-system.md`](design-system.md) — tokens, language, accessibility, and
  visual hierarchy.
- [`github-data-spike.md`](github-data-spike.md) — bounded query and truncation
  constraints.
