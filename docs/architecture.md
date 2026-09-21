# Architecture

## Decision

Review Radar uses a shared Rust core and native UI shells. A single custom-rendered
cross-platform UI would reduce initial work, but would feel ported on at least one
target operating system. Native shells preserve expected menu, tray, notification,
accessibility, keyboard, and window behavior.

Qt/QML is the Linux client because it integrates well with the existing Quickshell
environment while remaining suitable for a standalone desktop application.

## Core boundaries

### `crates/domain`

- Pull-request and event models.
- Relationship classification: authored, review-requested, following.
- Health and attention-state derivation.
- Tailored-view ranking.
- Event fingerprinting and reactivation logic.

### `crates/github`

- Authenticated GitHub API access.
- Snapshot queries and pagination.
- Conversion from GitHub responses into domain models.
- Rate-limit and retry policy.

### `crates/state`

- Per-device state store.
- Read watermark and snooze expiry per pull request.
- Last-emitted attention transition for notification deduplication.
- Schema migration.

The current SQLite store keys acknowledgement and snooze to the newest meaningful
event fingerprint. If that fingerprint changes, the item reactivates immediately;
otherwise a snooze stays effective until expiry. First attention observation is a
baseline, and only a later transition from non-attention to attention is eligible
for a notification.

### `crates/platform`

- Contracts for opening URLs, copying links, and delivering notifications.
- No platform implementation or UI toolkit dependency.

## Client boundaries

Clients receive an already-ranked snapshot plus explicit commands. They should not
reimplement GitHub queries, ranking, event classification, or persistence.

```text
GitHub API -> github -> domain + state -> client view model -> native UI
```

## Platform clients

| Platform | Primary UI | Integration |
| --- | --- | --- |
| Linux | Qt 6 / QML | Quickshell launcher/bar entry; native notifications |
| macOS | SwiftUI | Menu bar and Notification Center |
| Windows | WinUI 3 | System tray and Windows notifications |

## Local state

Local state is deliberately independent of GitHub's notification-read state. It
supports a personal action queue without mutating GitHub or conflating browser and
desktop workflows.

State is stored in the operating system's standard application-data location and
contains no GitHub access token.
