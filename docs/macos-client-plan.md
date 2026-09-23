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
- [x] Confirm the macOS CI runner builds and runs the fixture-driven Swift tests
  (GitHub Actions run `35897206706` on macOS 14).

## Workspace parity

- [x] Add the Action, My PRs, Following, and Recent workspace navigation with
  stable view IDs and retain selection by PR ID across refreshes.
- [x] Add keyboard navigation, paging, and shortcut help using macOS conventions
  while preserving shared meanings.
- [x] Add local search and ranking selection without moving sort policy into the
  client.
- [x] Render personal explanations, separate Review/Checks/Merge health values,
  lifecycle, friction, activity, and details without deriving display semantics.
- [x] Add browser, copy, acknowledge, and snooze commands using the state helper
  and current fingerprint.

## macOS integration

- [x] Implement `MacOsIntegration` for default-browser opening, clipboard, and
  Application Support paths behind the same OS-effect boundary.
- [x] Deliver state-deduplicated attention transitions through UserNotifications,
  with next-action and canonical-PR actions plus deep links.
- [x] Add a menu-bar item as an independent opt-in, including a discoverable
  Quit command and capability-aware preference state.
- [x] Implement native Preferences with the shared notification, category, and
  quiet-hours semantics; preserve drafts, atomic saves, and failure copy.
- [x] Complete menu-bar preference application and last-window lifecycle behavior.

## Release and quality

- [x] Package the SwiftUI app with matching universal Rust helpers, generated
  design-system icon assets, code-signing inputs, and no credentials.
- [ ] Verify first-run baseline, refresh-on-open/five-minute cadence, offline
  stale state, notification/category/quiet-hour behavior, and restart persistence.
- [ ] Audit VoiceOver labels, focus order, Dynamic Type, reduced motion, contrast,
  and keyboard access.
- [x] Add macOS CI build/test separately from Linux packaging.
- [x] Prepare macOS code-signing and notarization separately from Linux packaging.

## Completion criteria

The macOS shell consumes the same ranked projection and explicit state commands
as Linux, presents equivalent queue meanings with native SwiftUI interaction, and
keeps GitHub access, classification, ranking, local acknowledgement/snooze state,
and notification eligibility in the shared Rust helpers.
