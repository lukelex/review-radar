# macOS SwiftUI client

This is the initial native SwiftUI shell. It consumes the existing
`review-radar-github` and `review-radar-queue` process contract; Swift does not
query GitHub, classify/rank pull requests, or persist acknowledgement/snooze
state.

Configure GitHub access in Preferences → Account & sync. The token is stored in
macOS Keychain and passed only to the collector process; `GH_TOKEN` remains
available for CLI and automated runs.

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

## Packaging

On macOS, `scripts/package-macos VERSION` creates a universal `.app` archive with
the SwiftUI executable plus universal `review-radar-github`, `review-radar-queue`,
and `review-radar-state` helpers. It performs ad-hoc signing when no signing
identity is configured. Set `MACOS_CODESIGN_IDENTITY` for Developer ID signing and
also set `MACOS_NOTARY_PROFILE` to submit, staple, and archive a notarized build.
The app and helpers contain no GitHub token, capture, or local-state database.

`macOS package CI` builds and verifies an ad-hoc-signed archive on macOS 14. It
also attempts to retain that archive as a CI artifact; an exhausted GitHub Actions
artifact quota does not invalidate the completed package verification.
`Release macOS` is manually dispatched only after the corresponding GitHub release
tag exists. It requires these repository secrets: a base64 Developer ID Application
`.p12` (`MACOS_SIGNING_CERTIFICATE`), its password
(`MACOS_SIGNING_CERTIFICATE_PASSWORD`), its keychain identity
(`MACOS_CODESIGN_IDENTITY`), and App Store Connect API-key values
(`MACOS_NOTARY_API_KEY`, `MACOS_NOTARY_KEY_ID`, and
`MACOS_NOTARY_ISSUER_ID`). It imports those only on the hosted runner, notarizes
with a temporary keychain profile, staples the app, and uploads the ZIP plus its
SHA-256 sidecar to that existing release.
