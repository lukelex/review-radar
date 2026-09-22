# Quickshell bar adapter

Requires Quickshell (with `Quickshell.Io`), Qt Quick Controls, and systemd's
`busctl` on PATH. The Qt app and Quickshell must share the same user session bus.

1. Start Review Radar. Enable **Preferences → Desktop integration → Enable bar
   integration**, then Save. Reopen Preferences to check interface availability.
2. Copy `apps/quickshell/review-radar/` into your Quickshell configuration directory.
3. Import it and place its component in your existing bar layout:

```qml
import QtQuick
import Quickshell
import "review-radar" as Radar

ShellRoot {
    PanelWindow {
        anchors { top: true; left: true; right: true }
        implicitHeight: 36
        Radar.ReviewRadar { anchors.centerIn: parent }
    }
}
```

Left-click opens the existing app. Right-click (or the keyboard Menu key) offers
Open, Refresh, and Preferences. The tooltip names the current workspace, sync
state, and capture timestamp. The count is the current workspace's unsuppressed
attention-required cards, not a global count. Search text does not change it.

If the app exits, the interface is disabled, or the session bus is unavailable,
the component shows **Radar —** rather than a misleading zero. Start the app and
enable the integration to reconnect. The component never launches an extra app
instance. Close-to-tray is a separate opt-in if you want the app running hidden.

The component reads cached local status every three seconds. This does **not**
poll GitHub or run the queue executable. Only explicit Refresh invokes the app's
normal refresh command. No tokens, card titles, repository identities, or database
paths are transmitted. It performs no ranking, classification, persistence, or
notification delivery.

## Local status protocol v1

Session-bus service `org.reviewradar.App`, object `/org/reviewradar/Bar`, interface
`org.reviewradar.Bar1`, registered only while the saved integration preference is
on. A second instance cannot replace an existing service owner. Disable/re-enable
the preference to retry registration after a conflict or bus outage.

- `GetSnapshot() -> string`: JSON with `version: 1`, `available` (a successful
  projection exists), `workspace`, `attentionCount`, `capturedAt`, `syncState`.
- `syncState`: `unavailable`, `loading`, `syncing`, `ready`, `stale`, or `error`.
  During loading/error, values remain from the last successful projection, with
  its workspace ID. No count is claimed before the first successful projection.
- `OpenWorkspace()`, `OpenPreferences()`, `Refresh()`: explicit app commands.

Inspect without changing application state:

```sh
busctl --user --auto-start=no --json=short call org.reviewradar.App \
  /org/reviewradar/Bar org.reviewradar.Bar1 GetSnapshot
```

This is a same-user desktop interface, not a network server. Qt's OS adapter owns
the DBus implementation; other clients can reproduce the status/command contract
with their platform's IPC mechanism.

## Verification

The Qt CMake test suite adds `review-radar-bar` when `dbus-run-session` and `busctl`
are installed. It uses an isolated session bus to test the payload, explicit
commands, registration conflicts, disabling, and re-enabling. If `quickshell` is
on PATH, it also runs `smoke.qml` against that fixture service and verifies status
rendering inputs, a refresh request, and transition to disconnected after the
service is withdrawn. The smoke configuration is test-only; use the PanelWindow
example above for an actual bar.
