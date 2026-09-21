# Attention and review friction TODO

Work through these in order, with a separate commit for each completed task.
Design: [attention and review friction](docs/attention-and-review-friction.md).

- [x] **1. Attention explanations:** project evidence-backed personal reasons,
  concurrent health signals, and explicit next actions in the Rust domain;
  verify actionable, waiting, following, and completed cases.
  Implemented in `crates/domain/src/attention.rs`; queue JSON includes explanations
  and all health signals. New-comment obligations still require historical event
  comparison; a bounded snapshot does not establish newness.
- [x] **2. History/data spike:** inventory collector and fixture coverage for
  review cycles, churn, and ready/draft intervals; document measurement policy,
  missing-history behavior, and collection requirements.
  Findings and the experimental v1 policy are in
  `docs/review-friction-data-spike.md`. Current captures cannot support measured
  friction levels; history collection is a separate follow-through task.
- [x] **3. Friction assessments and ranking:** implement a versioned, explainable
  assessment contract with representative history fixtures, conservative missing
  data handling, and the `highest-friction` strategy selectable by stable ID.
  Implemented in `crates/domain/src/friction.rs` and `ranking.rs`, with synthetic
  history fixtures covering cycles, rework, waiting, draft pauses, completion,
  missing data, and reopening. The queue accepts `--ranking highest-friction`.
  Existing captures report Limited history; thresholds remain experimental.

## Follow-through

- [x] Compare meaningful reviewer events across captures to classify new feedback
  without treating all captured comments as new obligations. The projection compares
  an authored PR against its immediate predecessor; non-self, non-bot substantive
  feedback must be absent there and occur after its capture time. The first capture
  and missing/incomplete comparisons stay a no-event baseline.
- [x] Collect bounded ready/draft, lifecycle, review, and commit evidence and
  connect it to live assessments. Pagination and missing churn are explicit; the
  current normalizer reports Limited history rather than a fabricated level.
- [x] **Targeted parent-diff collection:** capture bounded, reproducible changed-line
  evidence between successive PR revisions after review starts; preserve comparison
  coverage and failures rather than substituting commit totals. In progress: the
  collector now requests each bounded commit's first-parent additions/deletions;
  the normalizer establishes an initial-review change-volume baseline only when
  complete parent-diff evidence is available; calibration remains separate.
- [x] **Friction calibration:** regenerate generalized representative histories and
  validate the experimental Low/Moderate/High thresholds before calling levels
  measured production output.
  Parent-diff traversal fixture: 100 generalized merged PRs from the requested
  repository, with review commit links and first-parent changed-line evidence.
  Calibrated v1 from 98 complete parent-diff journeys; thresholds and percentile
  rationale are recorded in `docs/review-friction-data-spike.md`.
- [x] Connect the projected reasons, concurrent health, friction breakdown, and
  ranking selection to the Qt/QML native client. Its model renders domain output
  only; Qt does not recalculate classification, friction, or sorting.
- [x] **Persistent new-feedback visibility:** retain a detected feedback event as
  an outstanding local reason until acknowledged or superseded, without changing
  first-observation notification baselines.
  Implemented in the domain projection and separate state-store table; bounded
  event windows no longer make an unacknowledged feedback reason disappear.

## CI

- [x] Build and smoke-test the final runtime Docker image in CI, including its
  expected binaries and startup behavior.
- [x] Add lightweight dependency and security checks such as `cargo audit` or
  `cargo deny`, without introducing paid services.
- [x] Add optional coverage generation and retain reports as GitHub artifacts
  rather than depending on an external coverage service.
- [x] Run fixture projection checks for the supported queue views in CI.
- [x] Compile the Linux Qt client when `apps/linux-qt` becomes active, ideally
  only when the client or its build dependencies change.
  `.github/workflows/linux-qt.yml` is path-filtered and runs build, QML lint,
  and the offscreen startup test.
