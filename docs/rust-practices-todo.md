# Rust core improvement plan

This checklist tracks the reliability and maintainability practices selected for
the shared Rust core. Each item should land in its own commit where practical so
the change can be reviewed and reverted independently.

## Planned commits

- [x] **Typed domain identifiers and enums** — reduce stringly typed PR IDs,
  fingerprints, lifecycle, and state values at core boundaries.
- [x] **Typed crate errors** — introduce crate-owned `thiserror` errors and keep
  `anyhow` at binary/application boundaries.
- [x] **Deterministic time** — inject a clock or explicit `now` value into
  time-sensitive domain and state operations.
- [x] **Atomic capture writes** — make capture persistence transactional and
  prove failed writes do not expose partial captures.
- [x] **Versioned migrations** — replace implicit schema setup with explicit,
  tested SQLite migrations.
- [x] **Typed serialization boundaries** — reduce unvalidated `serde_json::Value`
  use in GitHub response handling while retaining intentionally raw payloads.
- [x] **Property-based invariants** — test ranking, membership deduplication,
  fingerprints, acknowledgement, and reactivation properties.
- [x] **Structured diagnostics** — add `tracing` spans and fields for capture,
  request, cache, hydration, and projection work without logging secrets.
- [x] **Resilient networking** — add bounded retries/backoff, explicit timeout
  categories, rate-limit behavior, and process-boundary cancellation-safe
  refreshes.
- [x] **CI quality policy** — enforce formatting, clippy policy, unsafe-code
  policy, dependency advisories, and license checks.
- [x] **Library-first core** — move reusable collector and configuration logic
  out of binary-only modules for future clients and focused tests; queue
  projection diagnostics now expose an explicit projection boundary.
- [x] **Documentation and review gates** — keep core invariants, error semantics,
  and testing commands current as each practice lands.

## Completion rule

An item is complete only when its behavior is covered by tests or an explicit
acceptance check, the relevant architecture/contract documentation is updated,
and the repository's Docker verification commands pass. Test-only `unwrap()` and
`expect()` calls are acceptable when their fixture setup makes failure a test
failure; production paths must return contextual errors.

## Existing implementation notes

- State commands already receive an explicit `DateTime<Utc>` and queue
  projection receives an explicit observation time; binaries are the clock
  boundary.
- Capture persistence already writes the capture, PR rows, searches, and
  memberships in one SQLite transaction and commits only after validation.
- The local state database rejects newer schema versions and records its
  supported version through SQLite `user_version`; future changes should extend
  that migration path.

- Collector retries are bounded to three attempts with 250/500 ms backoff
  for transport timeouts and retryable HTTP statuses (408, 429, and 5xx).
  A collector run is a process boundary: cancellation terminates the process,
  and persistence occurs only after a complete capture, so no partial refresh
  can become visible.
- `review-radar-github` exposes `parse_config`, `collect_with_token`, and the
  complete `Capture` model from its library target; binaries remain thin
  application boundaries.
