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

## Interface parity

Every shared user-facing behavior must remain current across Linux Qt, macOS
SwiftUI, Windows WinUI, and the CLI/TUI contract. A platform may use native
presentation and integration APIs, but it must preserve equivalent meaning,
discoverability, local-state effects, notification delivery semantics,
accessibility, and lifecycle behavior. A change is not complete merely because it
works in the shell being edited.

If an interface cannot implement the behavior in the same change, document the
gap, rationale, platform-neutral acceptance criteria, and explicit follow-up in
its tracked plan before merging. Never silently make a shared feature available in
only one interface.

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
- Hydration retains aggregate check-rollup state plus check-context count and
  truncation metadata, but omits individual check-context nodes until a shared
  projection consumes them.
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

Hydration requests omit unused review-request nodes, connection totals, and
individual check-context nodes. They retain the bounded review, comment,
review-thread comment, lifecycle, commit-diff, and aggregate check evidence
needed to produce the same complete shared projection.

The bounded PR timeline also retains review-requested, review-request-removed,
and head-force-pushed events (reviewer type/identity where available, and before/
after head OIDs). This is raw source evidence for future handoff-episode
normalization; it is not yet a turnaround metric or ranking input. It uses a
separate bounded timeline connection so handoff pagination cannot make the
existing lifecycle/friction timeline appear incomplete (or vice versa). No client
infers exact request age or head-push time from `updatedAt` or commit-author dates.
The query has been exercised on a bounded live capture: request, removal, and
force-push events were returned for the sample, with no handoff pagination gaps.
Team/mannequin identity coverage and broader history coverage remain unvalidated.
Do not enable episode metrics until representative histories are observed and API
cost/coverage are characterized.

The queue projection includes optional per-card `handoffHistory` episodes. An
exact request→submitted-review wall-clock duration or pending age is emitted only
when both relevant connections are complete and all event bounds/IDs validate.
Team requests remain unattributed to individual responders; removals, pending
requests, and incomplete histories have distinct outcomes. Force-push events are
reported separately from ordinary revision commits. This evidence is diagnostic
only: it does not affect ranking, attention fingerprints, local state, or
notifications. Native clients must not turn it into a value/effort score or imply
that elapsed time is active review effort.

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

Red health treatment is reserved for failing or errored checks. Changes requested
and merge conflicts are amber follow-up states with distinct non-color symbols;
they must not be presented as equivalent to a failed check.

### Selection and keyboard navigation

A rendered PR card has at most one active outline in a workspace. Mouse selection
and keyboard navigation resolve to the same selected PR when details are open;
otherwise the keyboard-navigation target alone receives the outline. Toolkit
button focus and hover styling must not create a second active-outline indication.

### Accessibility acceptance criteria

- Health and friction chips expose their full label and value to assistive
  technologies; symbols and color are supplemental.
- Keyboard focus has a visible, non-color-only two-pixel outline on interactive
  navigation, search, sort, and action controls.
- Decorative shortcut hints and brand images do not create duplicate spoken
  announcements.
- A selected card and a keyboard-navigation target never appear as separate active
  cards.

Review friction uses a separate semantic low/medium/high/unknown color scale.
It communicates accumulated review difficulty, not urgency or PR health. Clients
must retain the explicit friction label, level, and a non-color cue; insufficient
history retains its explicit limited-history status rather than being presented
as low.

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

A failed request must never replace, clear, or otherwise invalidate the last
successful projection, including a successful projection with zero cards. The
client marks that projection cached after failure and opens a modal for every
failed refresh, projection, or local command request. The modal identifies the
failed operation, retains readable diagnostic detail (such as process output,
exit status, or response parse error), and allows the user to select or scroll
that detail. This favors preserving an actionable workspace over presenting a
potentially misleading empty state; detailed diagnostics may expose technical
information, so they remain in the in-app modal rather than a transient alert.

Acceptance criteria:

- After a successful projection, a later failed request leaves the same cards,
  selection, counts, and latest successful capture time intact.
- A failed request opens one modal with a specific operation title and its full
  available error detail; closing it does not discard the cached projection.
- If no successful projection exists, the failure modal still appears and the
  normal no-data loading/error treatment remains available.

Linux reference: `QueueController::reportRequestFailure` and the
`request-failure-dialog` in `apps/linux-qt/qml/Main.qml`.

The reference shell uses skeleton parent cards only when no cached list exists,
keeps cached cards visible during sync, shows an updating treatment on cached
cards, and defers the activity timeline briefly after opening details. This is
the safe form of view-targeted optimization: the shared snapshot remains
complete, while expensive detail rendering is delayed until it is useful. Other
clients may use native equivalents, but must preserve the same state meanings.

## Information hierarchy and interaction

### macOS SwiftUI foundation

`apps/macos` is a macOS 14+ Swift Package shell around the existing helper
process/JSON contract. It renders the five already-ranked workspace projections,
local search/ranking selection, card selection/details, refresh-on-open/five-
minute cadence, and loading/fresh/stale/failed states. It invokes the state helper
for acknowledgement and snooze with the projected current fingerprint. It stores
helper databases under Application Support using the shared `review-radar`
directory name. The delivery checklist is `docs/macos-client-plan.md`.

Its AppKit key monitor applies the documented navigation shortcuts only while the
first responder is not a text editor; it
supports local search focus, workspace cycling/direct selection, paging, shortcut
help, and Control-held numeric hints. Acceptance: the Swift process consumes stdout
as one queue JSON response, leaves stderr as diagnostics, never runs GitHub work
outside the collector helper, invokes state only through its explicit command
surface, and keeps the last successful projection visible across a failed refresh.
Its local `MacOsIntegration` owns Application Support paths, browser opening, and
copying; the queue view model does not call those host APIs directly.

On macOS, `MacOsIntegration` uses `UNUserNotificationCenter` only after the
shared queue response has supplied persisted `notificationEligibleIds`. The local
preference store writes a complete `preferences.json` atomically after validating
quiet-hour `HH:mm` values. Its master switch, four reason-code category filters,
and local-time quiet window suppress delivery only: attention observation and
deduplication still happen in Rust. The matching enabled reason supplies the
notification title/body and next-action URL; the canonical PR remains a separate
Notification Center action when it differs. Test alerts bypass delivery filters
and carry no PR action. Activation follows the same host-browser path as card
actions.

macOS provides an independent, persisted, opt-in menu-bar item rather than a
Linux tray or Quickshell adapter. It exposes Open Review Radar, Refresh,
Preferences, and Quit, and labels the current workspace's attention-required
count. Keeping the app alive after its last window closes requires both the
menu-bar item and its dependent saved preference; turning the menu-bar item off
also clears that background preference. This preserves a discoverable path back
to a hidden running app.

The macOS package carries fixture-driven decoding and local-preference tests, run
by `.github/workflows/macos.yml` on a macOS 14 runner. Linux hosts may validate
the shared Rust queue schema but cannot substitute for SwiftUI/AppKit compilation
or Notification Center interaction tests.

The macOS release package is built by `scripts/package-macos`, verified without
credentials in `.github/workflows/macos-package.yml`, and signed/notarized only by
the separate, manually dispatched `.github/workflows/macos-release.yml`. Windows
packaging remains blocked on its WinUI shell; its required parity work and
installer/signing acceptance criteria are tracked in `docs/windows-client-plan.md`.

### Windows WinUI foundation

`apps/windows` is a Windows 10+ WinUI 3 foundation shell around the same helper
process/JSON contract. It starts with the Tailored projection, preserved cached
cards during a refresh, explicit status text for local loading/sync/failure, and
the shared five-minute cadence. `WindowsOsIntegration` owns `%APPDATA%\review-
radar`, default-browser launching, and clipboard access; the queue view model does
not call Windows APIs directly. Development overrides and packaged helper lookup
match the other shells.

The foundation intentionally does not claim full workspace, command, notification,
preference, tray, keyboard, accessibility, or installer parity. Those gaps are
explicitly tracked in `docs/windows-client-plan.md`. Acceptance: queue stdout is
decoded as one schema-versioned response, stderr remains diagnostic-only, helper
work stays outside the view model, failed refreshes preserve any usable cards, and
no C# code queries GitHub or ranks/classifies pull requests.

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

### Proposed next-handoff ranking (research only)

The [value-flow ranking research](value-flow-ranking-research.md) proposes an
opt-in shared-core `flow-first-v1` strategy after validating review-request,
review-submission, and author-revision episode evidence. It targets shorter
request → review and review → revision turnaround without claiming that PR age,
review friction, or diff size is business value or next-action effort. Current
Tailored priority bands and newest-first ties remain the product default.

Observable behavior **if implemented**: the selected strategy orders actionable
PRs by an evidence-backed next-handoff heuristic with explicit wait-age protection
and an honest unknown-input fallback. It may cross review-requested and authored-
action bands only when opted into; it never changes workspace membership,
attention classification, fingerprints, acknowledgement/snooze, or notification
deduplication. Projection uses one successful capture/as-of time; failed refreshes
retain the last ranked cards with a cached label. Ranking, feature derivation, and
stable-ID selection live in Rust, not a native client.

Tradeoff: a short-action bias can neglect long reviews; aging and tail-latency
checks constrain it, but no static sort guarantees progress during overload.
Impact, effort, and stage clocks need verified evidence; missing history is never
displayed as an exact elapsed time or zero impact. The proposal is not enabled in
the CLI, Linux Qt, macOS SwiftUI, or Windows WinUI. Windows' paused shell also
lacks general ranking selection; record its parity follow-up before shipping a
shared UI control. All compatible clients and the CLI/TUI contract must show the
same ranking ID, rank order, fallback semantics, and explanatory evidence when
this capability is introduced, or carry explicit tracked gaps under the interface
parity rule above.

Platform-neutral acceptance before enabling: repeated/partial handoffs are not
misdated; a missing/unknown effort cannot be scored as free; aged work is not
permanently buried by a stream of short actions; high-friction waiting work does
not masquerade as an obligation; Recent remains historical; local state and
notifications are identical under both sorts; stable ties and stale refreshes do
not cause churn; offline replay and a consented rollout compare both median and
tail turnaround plus delivery/throughput against the unchanged Tailored baseline.

### Shared modal presentation

Shortcuts and confirmations share a centered, viewport-bounded modal with a
readable title, scrolling body, and persistent footer. This keeps longer help
content usable on smaller windows. Escape dismisses; confirmations initially
focus Cancel and capture their target when opened so refresh cannot redirect
the action to a different PR. Shortcuts use grouped keycaps and Done.
Preferences reuse the same hierarchy with grouped settings and explicit
Save/Cancel for draft edits; the first implemented setting controls desktop alerts.
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

Implementation reference: `LinuxOsIntegration::applicationDataFile` in
`apps/linux-qt/linuxosintegration.cpp`.

### Native OS-effect adapters

Preferences design direction: OS integrations are independently optional. The
proposed modal groups notifications, desktop integration, workspace, appearance,
keyboard, account/sync, local data, general, and advanced settings. Changes remain
draft across sections and apply together on Save; Cancel and Escape offer discard
when dirty. Unsupported capabilities must be explained rather than shown as
working controls. Notification opt-out should keep attention observations current,
so opting back in does not replay a backlog. Delivery filters may narrow core
eligibility but must never create new eligibility in the UI. Tray-dependent
background operation must not leave an undiscoverable running application.

The first native preference is the desktop-notification master switch, enabled by
default. Linux stores it atomically in `preferences.json` under the OS adapter's
application-data directory, separately from captures and attention state. Save
applies only after persistence succeeds; failure keeps the modal and draft open.
Cancel or Escape with a dirty draft requires discard confirmation. Delivery is
gated after projection, so muted observations still update shared deduplication
state. Restart restores the saved choice. Other settings remain design proposals.

Preferences includes an explicit Test notification action. It sends one synthetic
alert through the OS adapter even when automatic alerts are disabled, without
saving draft preferences or touching attention observations. The test has no PR
activation URL or browser action. Report adapter acceptance as sent, not proof of
on-screen display (desktop quiet modes may suppress it); report delivery failure
inline. This makes the platform integration independently diagnosable.

Notification delivery preferences also expose four independent categories:
review requests, feedback requiring a response, failed checks, and merge
conflicts. A card is delivered when at least one enabled category matches one of
its projected reason codes. These are delivery filters only: the queue still
records every attention observation and deduplicates every meaningful fingerprint
before the client filters delivery. Turning a category back on does not replay
transitions observed while it was muted.

The same section provides optional local quiet hours with start/end `HH:mm`
values. During the window, the client suppresses delivery only; projection,
attention observation, fingerprints, and category evaluation continue. Windows
may cross midnight, use the device's local clock, and are validated before an
atomic save. Invalid times keep the draft open with no partial preference update.
Quiet hours are not a GitHub setting and do not replay suppressed transitions
when they end.

### Optional desktop tray

The system-tray icon defaults off. Preferences → Desktop integration saves the
tray switch, attention-dot switch, and separate keep-running-on-close switch with
the notification setting in one atomic update. Disabling the tray clears the
close-to-tray preference. Draft edits do not affect the running tray.

An available, enabled tray provides Open Review Radar, Refresh, Preferences, and
Quit. Primary activation restores the workspace; Preferences restores the window
and opens the modal. Quit always exits. Closing the window hides it only when
both saved switches are on and the OS adapter currently reports a tray host.
Otherwise closing behaves normally. Host loss restores a hidden window; the
reference adapter checks for host changes every two seconds. This avoids leaving
a background app without a discoverable way back to its workspace.

The optional attention dot and tooltip count reflect unsuppressed,
attention-required cards in the current projected workspace, not a fabricated
global count. Switching workspaces updates that scope. A failed sync keeps the
last projected indicator, just as it keeps the last workspace. Ranking and
attention classification remain in the shared core.

Acceptance: defaults exit on close; saved tray choices survive restart; no-host
close exits normally; host loss restores the window; disabling the tray clears
background behavior; refresh and notification observation continue while hidden.
Linux uses QSystemTrayIcon/QMenu behind OsIntegration and QApplication for the
menu runtime. QML controls explicitly use Basic style to avoid inheriting an
unavailable widget theme. Other native shells should preserve these behaviors
using their own tray or menu-bar APIs.

### Optional Quickshell bar adapter

Bar integration is a separate, persisted opt-in under Desktop integration and
defaults off. Saving registers a same-user status/command interface through the
OS adapter. Failure to register (for example, a competing app instance) is shown
in Preferences; it does not silently claim that the bar is connected. The saved
opt-in can be toggled to retry. Disabling withdraws the interface immediately.

The bar renders only a versioned summary of the last successful workspace
projection: workspace ID, attention-required count, capture timestamp, and sync
state. Counts are of unsuppressed cards in that workspace, independent of text
search. A workspace switch keeps the predecessor workspace label until its new
projection succeeds, avoiding a mismatched label/count. No successful projection
means unavailable, not zero attention. Failed sync retains the last count with an
error state. No card identities, tokens, or database paths cross this interface.

Primary activation restores the existing app; the context menu offers Open,
Refresh, and Preferences. These invoke the existing app commands. The bar never
launches a second collector, classifies cards, persists state, or sends alerts.
The reference component reads cached local status every three seconds (no GitHub
requests); invalid/failed responses clear its count, with a ten-second watchdog
for a nonresponding helper. Explicit Refresh alone starts a GitHub refresh.

Closing/exiting the app disconnects the bar. Keeping the app running hidden still
requires the separate tray and close-to-tray preferences. Quickshell setup is
manual so enabling a preference never edits the user's bar configuration.

Acceptance: saved opt-in survives restart; disabled interface is inaccessible;
available zero and disconnected are distinct; labels/counts stay coherent across
view switches; controls reach the running app; failed/restarted service connections
recover; bar reads leave capture and attention state untouched. Linux reference:
`apps/quickshell/README.md` specifies the DBus v1 contract. Tests use a private
session bus and, when installed, the real Quickshell runtime. Other platforms can
implement the same summary/commands using their own IPC.

Linux notifications identify the `review-radar-linux` application and carry the
embedded canonical logo as freedesktop `image-data` (RGBA pixels with explicit
stride, dimensions, and alpha). The daemon must not need access to the app's
filesystem or an installed icon theme entry: it may run outside the container or
receive notifications from an uninstalled build. The icon name and desktop entry
remain identity/fallback hints. Acceptance: a separate receiver with no installed
Review Radar icon receives the same logo pixels through DBus. This is verified by
the private-bus notification integration test, including signature and pixel hash.

Notifications expose explicit actions from the shared projected card: its relevant next
action (when it has a URL) and a canonical Open pull request action when distinct.
The OS adapter maps action IDs back to URLs; the queue supplies no DBus details.
Test notifications have no PR action. Action labels remain text, rather than
icon-only affordances, and a notification service may choose its own rendering.

The native preferences shell follows the high-fidelity reference: a wide modal
with a section rail, white bordered setting groups, a notification example, and
persistent header/footer. At narrower sizes the rail becomes horizontal and the
content scrolls independently. All nine planned sections remain discoverable;
unimplemented options are labeled Planned and cannot change behavior. This keeps
the approved information hierarchy while making the initial functional scope
explicit. Acceptance: switching sections preserves unsaved notification edits,
and Save/Cancel remain reachable at the minimum application window size.
`docs/mockups/high/preferences.html` explores the layout and candidate options;
simulated availability and future settings are explicitly marked. The prototype
uses a contained scrolling body and persistent actions, adapts navigation to a
horizontal rail on narrow windows, and retains native keyboard focus and labeled
controls. Keeping this first step isolated permits option selection before adding
storage, platform capability detection, and delivery-policy changes.

Native clients isolate operating-system effects behind a client-local interface.
The interface accepts already-classified, state-deduplicated requests and owns
their platform delivery, including notification actions. It must not classify PRs,
decide notification eligibility, or access GitHub.

Acceptance criteria:

- Queue and view-model code import no platform notification API directly.
- A notification request carries a stable ID, presentation text, and optional
  activation URL; the OS adapter may use the ID to replace a prior visible alert.
- A notification activation reaches the same browser-opening command path as a
  card action.
- The adapter can expand to URL, clipboard, and storage-path effects without
  changing domain, state, or queue semantics.

Linux reference: `OsIntegration` and `LinuxOsIntegration` in `apps/linux-qt/`.

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
