# Repository guidance

## Current state

- The Cargo workspace contains the GitHub collector in `crates/github` and fixture-driven ranking/event projection in `crates/domain`; native apps and the state/platform crates remain placeholders.
- Read `docs/architecture.md` for component contracts, `docs/project-plan.md` for product semantics and delivery phases, and `docs/decisions/0001-native-shells.md` for the accepted UI decision. These describe intended behavior, not implemented features.
- Read [the design system and language](docs/design-system.md) before changing UI, branding, or user-facing copy. Follow its shared tokens, components, terminology, and interaction guidance; update it when those conventions change. Visual references and implementation screenshots live in `docs/mockups/high/`.
- `docs/prototype-findings.md` is the verified bridge from the existing dotfiles prototype: preserve its interaction model, but do not move its `gh`/`jq` queries, one-minute polling, ranking, or notification state into QML.
- Docker is the supported toolchain. Run the collector with `GH_TOKEN="$(gh auth token)" docker compose run --rm collector`; it appends private, unsanitized captures to the gitignored `data/review-radar.sqlite3`.
- Inspect the live domain projection with `docker compose run --rm queue --view tailored` (or `action`, `my-prs`, `following`, `recent`). It reads the latest SQLite capture without calling GitHub and applies the separate local-state database; pass `--capture-id` to inspect a specific capture. Use `--record-attention true` only after a successful capture so first observation baselines instead of generating notifications.
- Verify with `docker build --target builder -t review-radar-builder .`, then run `docker run --rm review-radar-builder cargo fmt --all -- --check`, `docker run --rm review-radar-builder cargo clippy --workspace --all-targets --locked -- -D warnings`, and `docker run --rm review-radar-builder cargo test --workspace --locked`.
- Read `docs/github-data-spike.md` before changing `crates/github/src/query.graphql` or `schema.sql`; collection is intentionally bounded, nested truncation is retained in JSON payloads, and search memberships must remain explicit.
- `tests/fixtures/github/github-snapshot.json` is a generalized real capture. Regenerate it only with `docker compose run --rm seed-export`; never copy or commit `data/review-radar.sqlite3`. Fixture replacement can renumber all pseudonyms and must pass the identity-field tests.

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
