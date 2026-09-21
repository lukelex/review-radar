# Linux Qt client

The standalone Qt 6/QML client consumes the already-ranked projection from
`review-radar-queue`. Its C++ `QueueController` maps the response to a fixed Qt
model and invokes `review-radar-state` for acknowledgement and snooze commands.
QML does not access GitHub, SQLite, ranking policies, or classification.

On open and every five minutes, the client runs `review-radar-github` followed by
the queue with attention observation recording. `GH_TOKEN` must be available to
the process. Capture and local-state databases use Qt's standard app-data
directory; `REVIEW_RADAR_CAPTURE_DATABASE` can override the capture path.
`REVIEW_RADAR_COLLECTOR_COMMAND`, `REVIEW_RADAR_QUEUE_COMMAND`, and
`REVIEW_RADAR_STATE_COMMAND` override executable paths. Set
`REVIEW_RADAR_SKIP_COLLECTION=true` to inspect an existing capture without a
GitHub refresh.

```sh
cmake -S apps/linux-qt -B build/linux-qt
cmake --build build/linux-qt
./build/linux-qt/review-radar-linux
```

Qt is intentionally a native Linux build; Docker CI continues to verify the Rust
core. Quickshell may launch this binary or show an attention count, but is not
required by the dashboard.
