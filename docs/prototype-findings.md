# Quickshell prototype findings

The prototype in `~/dotfiles/linux/config/quickshell` establishes the first useful
interaction model. This repository uses it as behavioral input, not as code to
copy into every client.

## Keep

- A compact bar entry with an attention indicator opens the PR workspace.
- PRs are parent cards showing repository/number, title, action label, latest
  activity, and relative age without requiring expansion.
- Expanding a card shows review activity in chronological order.
- Primary actions include opening the PR in a browser and marking activity seen.
- The queue prioritizes review requests and authored PRs needing action, then
  authored PRs awaiting review and followed PRs; items are newest-first per band.
- Notification startup establishes a baseline instead of replaying old activity.

## Replace in the product architecture

- `scripts/github-prs` performs one `gh pr view` request per result and derives
  actions in `jq`. The Rust GitHub layer uses bounded GraphQL snapshots; the Rust
  domain layer owns classification and ranking.
- `GitHubPrService.qml` polls every minute and QML consumes loosely typed JSON.
  Production refresh is five minutes, plus refresh-on-open, and clients receive a
  typed, already-ranked view model.
- The prototype equates any authored approval with readiness to merge. The core
  additionally requires a non-draft, mergeable PR without failing/pending checks;
  readiness stays provisional until ruleset and permission data is available.
- Seen review IDs currently live inside notification-center settings. Product
  acknowledgement, snooze, event watermarks, and notification deduplication belong
  in `crates/state`, independent of desktop notification history.
- Notification scripts query GitHub independently. Product notifications must be
  emitted only from persisted attention-state transitions after a successful poll.
- Quickshell remains a launcher/bar adapter. The Linux dashboard itself is a Qt/QML
  application and must not depend on Quickshell APIs.

The initial Rust projection in `crates/domain` ports the prototype's action labels,
parent/event shape, relationship precedence, and grouped ordering. It intentionally
does not implement acknowledgement or notifications; those require comparisons
across persisted captures in the state layer.
