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

The initial shell shows the already-ranked Tailored workspace, cached/stale state,
card selection, details, search, and refresh-on-open/five-minute cadence. It uses
`~/Library/Application Support/review-radar/` for capture and state files. Set
the same `REVIEW_RADAR_CAPTURE_DATABASE`, `REVIEW_RADAR_STATE_DATABASE`,
`REVIEW_RADAR_COLLECTOR_COMMAND`, and `REVIEW_RADAR_QUEUE_COMMAND` overrides as
the Linux client to use another helper location or fixture setup.

See [`docs/macos-client-plan.md`](../../docs/macos-client-plan.md) for the
remaining parity, integration, testing, and release checklists.
