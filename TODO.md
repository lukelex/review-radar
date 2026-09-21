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

- [ ] Compare meaningful reviewer events across captures to classify new feedback
  without treating all captured comments as new obligations.
- [ ] Collect the historical evidence identified by the spike and connect it to
  live assessments; do not present unavailable metrics as measured values.
- [ ] Connect the projected reasons, friction breakdown, and ranking selection to
  the Qt/QML native client when its shell is implemented.
