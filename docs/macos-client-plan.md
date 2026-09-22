# macOS client game plan

This checklist defines the SwiftUI shell's delivery order. It inherits shared
projection, state, ranking, and notification semantics from the native client
contract; no item authorizes duplicating them in Swift.

## Foundation

- [x] Create a macOS-only SwiftUI package under `apps/macos`.
- [x] Decode the `review-radar-queue` JSON response as a read-only native view
  model.
- [x] Run the existing collector/queue helper boundary with stdout reserved for
  JSON and stderr retained for diagnostics.
- [x] Store capture and device-state paths in Application Support using the
  shared `review-radar` directory name.
- [x] Present ranked Tailored cards, selection, an empty state, cached/stale
  status, and an explicit refresh action.
- [ ] Build and run the package on a supported macOS runner and add fixture-driven
  Swift tests to CI.

## Workspace parity

- [ ] Add the Action, My PRs, Following, and Recent workspace navigation with
  stable view IDs and persisted selection by PR ID.
- [ ] Add local search, ranking selection, keyboard navigation, paging, and
  shortcut help using macOS conventions while preserving shared meanings.
- [ ] Render personal explanations, separate Review/Checks/Merge health values,
  lifecycle, friction, activity, and details without deriving display semantics.
- [ ] Add browser, copy, acknowledge, and snooze commands using the state helper
  and current fingerprint.

## macOS integration

- [x] Implement `MacOsIntegration` for default-browser opening, clipboard, and
  Application Support paths behind the same OS-effect boundary.
- [ ] Deliver state-deduplicated attention transitions through UserNotifications,
  with next-action and canonical-PR actions plus deep links.
- [ ] Add a menu-bar item as an independent opt-in, including a discoverable
  Quit command and capability-aware preference state.
- [ ] Implement native Preferences with the shared notification, category,
  quiet-hours, and integration semantics; preserve drafts, atomic saves, and
  failure copy.

## Release and quality

- [ ] Package the SwiftUI app with matching Rust helpers, icon assets, code-signing
  inputs, and no credentials.
- [ ] Verify first-run baseline, refresh-on-open/five-minute cadence, offline
  stale state, notification/category/quiet-hour behavior, and restart persistence.
- [ ] Audit VoiceOver labels, focus order, Dynamic Type, reduced motion, contrast,
  and keyboard access.
- [ ] Add macOS CI build/test and release-notarization preparation separately from
  Linux packaging.

## Completion criteria

The macOS shell consumes the same ranked projection and explicit state commands
as Linux, presents equivalent queue meanings with native SwiftUI interaction, and
keeps GitHub access, classification, ranking, local acknowledgement/snooze state,
and notification eligibility in the shared Rust helpers.
