# Repository guidance

## Current state

- This is a planning scaffold: `apps/`, `crates/`, and `tests/fixtures/` contain only placeholders. There are no build manifests, lockfiles, CI workflows, or build/test/lint commands yet; do not assume a Cargo workspace exists.
- Read `docs/architecture.md` for component contracts, `docs/project-plan.md` for product semantics and delivery phases, and `docs/decisions/0001-native-shells.md` for the accepted UI decision. These describe intended behavior, not implemented features.
- The first planned milestone is bounded GitHub queries and snapshot fixtures in `tests/fixtures/`, before the domain core and Linux UI.

## Implementation boundaries

- Keep GitHub queries and normalization in `crates/github`; classification, ranking, event fingerprints, and reactivation in `crates/domain`; per-device persistence and notification deduplication in `crates/state`.
- `crates/platform` defines URL-opening, clipboard, and notification contracts only; it must not contain platform implementations or UI toolkit dependencies.
- Native clients consume already-ranked snapshots and explicit commands. GitHub queries, ranking, event classification, and persistence belong in the shared Rust core.
- Linux uses Qt 6/QML as a standalone application. Quickshell is an integration adapter, not the primary runtime. The planned macOS and Windows shells use SwiftUI and WinUI 3 respectively.

## Product invariants

- Acknowledgement and snooze are per-device state, independent of GitHub's notification-read state; they must not mutate GitHub. New meaningful events reactivate acknowledged or snoozed PRs.
- Persist local state in the OS-standard application-data location, without GitHub access tokens.
- Notifications are triggered by newly entering an attention-required state, with deduplication across polling cycles—not by every refresh or event.
- Default ranking uses priority bands, newest-first within each band; use the exact ordering in `docs/project-plan.md`. Recent closed/merged PRs cover the previous 14 days.
- V1 targets the authenticated `github.com` account only, refreshing on open and every five minutes while running.
