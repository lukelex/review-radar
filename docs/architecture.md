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
- Reusable `collect_with_token` and queue `projection` library boundaries for
  native clients; binaries provide only persistence and process-facing wiring.

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

`PlatformCommand` carries these effects as plain data. Platform notifications
receive only transitions already deduplicated by `crates/state`; a native client
implements `PlatformEffects` for its own toolkit and operating system.

Each native shell must also isolate host effects behind a shell-local OS adapter.
The adapter owns platform APIs such as notification delivery, notification action
handling, URL launching, clipboard access, and application-data paths; queue and
view-model code depend only on its interface. Linux Qt's first adapter is
`OsIntegration`, with `LinuxOsIntegration` providing the DBus and portal details.

The Linux adapter also owns QSystemTrayIcon, its native menu, icon decoration,
and host-availability monitoring. It emits platform-neutral workspace, preferences,
refresh, and quit requests; the client owns window visibility and saved preferences.
Qt Widgets is required for the native tray menu, while the main workspace remains
Qt Quick/QML.

`apps/quickshell` is now a thin opt-in bar adapter. LinuxOsIntegration publishes a
versioned summary over a session-bus service only while enabled; the controller
supplies counts of already-classified cards and explicit sync state. Quickshell
reads cached status through `busctl` and invokes workspace/refresh/preferences
commands. It does not run shared-core executables or own any notification state.

## Client boundaries

Clients receive an already-ranked snapshot plus explicit commands. `Acknowledge`
and `SnoozeUntil` carry the card's current fingerprint, so persistence stays local
while a changed card signal reactivates it. Clients should not reimplement GitHub
queries, ranking, event classification, or persistence.

```text
GitHub API -> github -> domain + state -> client view model -> native UI
```

## Platform clients

| Platform | Primary UI | Integration |
| --- | --- | --- |
| Linux | Qt 6 / QML | Quickshell launcher/bar entry; native notifications |
| macOS | SwiftUI | Menu bar and Notification Center |
| Windows | WinUI 3 | System tray and Windows notifications |

The initial Linux app in `apps/linux-qt` refreshes the Rust collector on open and
every five minutes, then maps the ranked queue response into a fixed Qt model.
It sends acknowledgement and snooze commands back to the state executable. Its
QML layer only presents cards and client-side text search; it does not duplicate
GitHub, domain, ranking, or SQLite behavior.

`apps/macos` starts the SwiftUI implementation with the same process boundary:
the shell runs the collector and queue helpers, decodes queue stdout, and renders
the ranked projection. It keeps macOS-only UI/integration work out of the Rust
core and does not make Swift a second source of GitHub/domain behavior. Its
shell-local `MacOsIntegration` owns Application Support paths, default-browser
opening, and clipboard effects. The shell sends acknowledgement and snooze back
to the state helper with the projection's current fingerprint. Its same adapter
owns UserNotifications delivery/action activation; Swift only filters already
deduplicated queue eligibility using saved local preferences. The SwiftUI shell
owns its optional menu-bar scene and delegates only last-window termination policy
to its AppKit application delegate.

`apps/windows` begins the WinUI 3 implementation with the same process boundary.
Its `QueueStore` invokes the collector and queue helpers, parses queue stdout as
one versioned response, and renders only the ranked Tailored projection in the
foundation slice. `WindowsOsIntegration` owns `%APPDATA%\review-radar` paths,
browser opening, and clipboard effects. The current scope intentionally leaves
workspace switching, state commands, notifications, and tray lifecycle to the
tracked Windows delivery plan rather than duplicating shared logic in C#.

On Linux, `LinuxOsIntegration` delivers notifications through
`org.freedesktop.Notifications`. The client receives only the queue projection's
persisted `notificationEligibleIds`, and its OS adapter opens the relevant PR in
the browser from the notification action.

The adapter sends embedded logo pixels in the standard DBus `image-data` hint.
An installed icon name alone is insufficient when the host notification daemon
cannot see the app's container or bundle filesystem. Pixel encoding stays inside
the Linux adapter; the shared contracts and controller do not handle DBus images.

The notification master switch and category filters are client-local delivery
preferences. They run only after the queue projection supplies persisted
`notificationEligibleIds`; they cannot create, classify, or deduplicate attention
transitions.

## Local state

Local state is deliberately independent of GitHub's notification-read state. It
supports a personal action queue without mutating GitHub or conflating browser and
desktop workflows.

State is stored in the operating system's standard application-data location and
contains no GitHub access token. `StateStore::open_default()` resolves to
`$XDG_DATA_HOME/review-radar` (or `~/.local/share/review-radar`) on Linux,
`~/Library/Application Support/review-radar` on macOS, and `%APPDATA%` on Windows.
