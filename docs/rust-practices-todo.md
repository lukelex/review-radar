# Rust core improvement plan

This checklist tracks the reliability and maintainability practices selected for
the shared Rust core. Each item should land in its own commit where practical so
the change can be reviewed and reverted independently.

## Planned commits

- [ ] **Typed domain identifiers and enums** — reduce stringly typed PR IDs,
  fingerprints, lifecycle, and state values at core boundaries.
- [x] **Typed crate errors** — introduce crate-owned `thiserror` errors and keep
  `anyhow` at binary/application boundaries.
- [ ] **Deterministic time** — inject a clock or explicit `now` value into
  time-sensitive domain and state operations.
- [ ] **Atomic capture writes** — make capture persistence transactional and
  prove failed writes do not expose partial captures.
- [ ] **Versioned migrations** — replace implicit schema setup with explicit,
  tested SQLite migrations.
- [ ] **Typed serialization boundaries** — reduce unvalidated `serde_json::Value`
  use in GitHub response handling while retaining intentionally raw payloads.
- [ ] **Property-based invariants** — test ranking, membership deduplication,
  fingerprints, acknowledgement, and reactivation properties.
- [ ] **Structured diagnostics** — add `tracing` spans and fields for capture,
  request, cache, hydration, and projection work without logging secrets.
- [ ] **Resilient networking** — add bounded retries/backoff, explicit timeout
  categories, rate-limit behavior, and cancellation-safe refresh boundaries.
- [ ] **CI quality policy** — enforce formatting, clippy policy, unsafe-code
  policy, dependency advisories, and license checks.
- [ ] **Library-first core** — move reusable collector, queue, and configuration
  logic out of binary-only modules for future clients and focused tests.
- [ ] **Documentation and review gates** — keep core invariants, error semantics,
  and testing commands current as each practice lands.

## Completion rule

An item is complete only when its behavior is covered by tests or an explicit
acceptance check, the relevant architecture/contract documentation is updated,
and the repository's Docker verification commands pass. Test-only `unwrap()` and
`expect()` calls are acceptable when their fixture setup makes failure a test
failure; production paths must return contextual errors.
