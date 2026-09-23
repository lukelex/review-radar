# Repository guidance

## Current state

- The Cargo workspace contains the GitHub collector in `crates/github` and fixture-driven ranking/event projection in `crates/domain`; native apps and the state/platform crates remain placeholders.
- Read `docs/architecture.md` for component contracts, `docs/project-plan.md` for product semantics and delivery phases, and `docs/decisions/0001-native-shells.md` for the accepted UI decision. These describe intended behavior, not implemented features.
- Read [the design system and language](docs/design-system.md) before changing UI, branding, or user-facing copy. Follow its shared tokens, components, terminology, and interaction guidance; update it when those conventions change. Visual references and implementation screenshots live in `docs/mockups/high/`.
- Read `docs/native-client-contract.md` before changing performance, UX/UI,
  loading states, client behavior, or architectural decisions. Treat it as a
  long-lived cross-platform contract: update it in the same change with the
  rationale, observable behavior, tradeoffs, and platform-neutral acceptance
  criteria so future macOS, Windows, and TUI clients can re-implement the
  behavior. Do not document only the Qt implementation or rely on chat history.
- `docs/prototype-findings.md` is the verified bridge from the existing dotfiles prototype: preserve its interaction model, but do not move its `gh`/`jq` queries, one-minute polling, ranking, or notification state into QML.
- Docker is the supported toolchain. Run the collector with `GH_TOKEN="$(gh auth token)" docker compose run --rm collector`; it appends private, unsanitized captures to the gitignored `data/review-radar.sqlite3`.
- Inspect the live domain projection with `docker compose run --rm queue --view tailored` (or `action`, `my-prs`, `following`, `recent`). It reads the latest SQLite capture without calling GitHub and applies the separate local-state database; pass `--capture-id` to inspect a specific capture. Use `--record-attention true` only after a successful capture so first observation baselines instead of generating notifications.
- Verify with `docker build --target builder -t review-radar-builder .`, then run `docker run --rm review-radar-builder cargo fmt --all -- --check`, `docker run --rm review-radar-builder cargo clippy --workspace --all-targets --locked -- -D warnings`, and `docker run --rm review-radar-builder cargo test --workspace --locked`.
- Read `docs/github-data-spike.md` before changing `crates/github/src/query.graphql` or `schema.sql`; collection is intentionally bounded, nested truncation is retained in JSON payloads, and search memberships must remain explicit.
- `tests/fixtures/github/github-snapshot.json` is a generalized real capture. Regenerate it only with `docker compose run --rm seed-export`; never copy or commit `data/review-radar.sqlite3`. Fixture replacement can renumber all pseudonyms and must pass the identity-field tests.

## Interface parity rule

- Keep every supported interface current at the same time. A shared user-facing
  feature, command, preference, state transition, workspace, ranking option, or
  notification behavior is incomplete until Linux Qt, macOS SwiftUI, Windows
  WinUI, and the CLI/TUI contract expose equivalent meaning and outcomes.
- Platform-native integrations may differ in presentation (for example, tray vs.
  menu bar), but they must preserve the same discoverability, local-state,
  delivery, accessibility, and lifecycle guarantees. Do not treat a completed
  implementation in one shell as permission to leave another shell stale.
- When an interface cannot implement a capability in the same change, update the
  relevant cross-platform contract and its tracked plan with the gap, rationale,
  acceptance criteria, and explicit follow-up before merging. Keep all other
  compatible interfaces updated; do not silently narrow shared behavior to the
  currently edited client.

## Implementation boundaries

- Keep GitHub queries and normalization in `crates/github`; classification, ranking, event fingerprints, and reactivation in `crates/domain`; per-device persistence and notification deduplication in `crates/state`.
- `crates/platform` defines URL-opening, clipboard, and notification contracts only; it must not contain platform implementations or UI toolkit dependencies.
- Ranking policies live in `crates/domain/src/ranking.rs`. They order projected cards only; classification and workspace-view membership stay in the domain projection. Add a `RankingStrategy` and select it by its stable ID rather than adding sort logic to a client or SQL query.
- `crates/state` uses a separate SQLite store for local-only acknowledgement, snooze, and attention observations. Acknowledge/snooze always needs the current newest meaningful-event fingerprint; a changed fingerprint must reactivate the PR. The first attention observation establishes a no-notification baseline.
- Native clients consume already-ranked snapshots and explicit commands. GitHub queries, ranking, event classification, and persistence belong in the shared Rust core.
- Keep the same projected dataset and command surface available to a CLI client; when building shared features, also account for a future TUI client rather than coupling behavior or data exclusively to a native GUI.
- Linux uses Qt 6/QML as a standalone application. Quickshell is an integration adapter, not the primary runtime. The planned macOS and Windows shells use SwiftUI and WinUI 3 respectively.

## Product invariants

- Acknowledgement and snooze are per-device state, independent of GitHub's notification-read state; they must not mutate GitHub. New meaningful events reactivate acknowledged or snoozed PRs.
- Persist local state in the OS-standard application-data location, without GitHub access tokens.
- Notifications are triggered by newly entering an attention-required state, with deduplication across polling cycles—not by every refresh or event.
- Default ranking uses priority bands, newest-first within each band; use the exact ordering in `docs/project-plan.md`. Recent closed/merged PRs cover the previous 14 days.
- V1 targets the authenticated `github.com` account only, refreshing on open and every five minutes while running.

## Commit discipline

- Before creating any commit, always run the repository lint and test checks:
  `docker run --rm review-radar-builder cargo fmt --all -- --check`,
  `docker run --rm review-radar-builder cargo clippy --workspace --all-targets --locked -- -D warnings`,
  and `docker run --rm review-radar-builder cargo test --workspace --locked`.
  If the builder image does not exist or is stale, build it first with
  `docker build --target builder -t review-radar-builder .`.
