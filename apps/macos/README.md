# macOS SwiftUI client

This is the initial native SwiftUI shell. It consumes the existing
`review-radar-github` and `review-radar-queue` process contract; Swift does not
query GitHub, classify/rank pull requests, or persist acknowledgement/snooze
state.

On macOS 14 or later, with matching helper executables on `PATH`:

```sh
cd apps/macos
swift run ReviewRadarMac
```

The shell shows all five already-ranked workspaces, cached/stale state, local
search, ranking selection, card selection/details, browser/copy actions, and
local mark-read/snooze commands. Keyboard shortcuts match the native-client
contract outside text fields. Preferences include Notification Center permission,
delivery categories, local quiet hours, and a test alert. It refreshes on open and
every five minutes. An optional menu-bar item provides Open, Refresh, Preferences,
and Quit; it can keep the app refreshing when the last workspace window closes.
It uses `~/Library/Application Support/review-radar/` for capture and state files.
Set `REVIEW_RADAR_CAPTURE_DATABASE`, `REVIEW_RADAR_STATE_DATABASE`,
`REVIEW_RADAR_COLLECTOR_COMMAND`, `REVIEW_RADAR_QUEUE_COMMAND`, and
`REVIEW_RADAR_STATE_COMMAND` to use another helper location or fixture setup.
Packaged apps prefer the matching signed helpers embedded in
`Review Radar.app/Contents/Helpers/`; those command overrides remain available
for development and diagnostics.

See [`docs/macos-client-plan.md`](../../docs/macos-client-plan.md) for the
remaining runner-validation, release, and accessibility checklists.

The `macOS SwiftUI CI` workflow runs `swift build --build-tests` and `swift test`
on a macOS 14 runner. Its fixture test locks the versioned queue-response shape.
