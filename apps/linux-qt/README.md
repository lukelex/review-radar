# Linux Qt client

The standalone Qt 6/QML client consumes the already-ranked projection from
`review-radar-queue`. Its C++ `QueueController` maps the response to a fixed Qt
model and invokes `review-radar-state` for acknowledgement and snooze commands.
QML does not access GitHub, SQLite, ranking policies, or classification.
It renders every domain-provided reason, health summary, friction contributor or
limitation, and ranking selection; it never derives those values locally.

On open and every five minutes, the client runs `review-radar-github` followed by
the queue with attention observation recording. `GH_TOKEN` must be available to
the process. Capture and local-state databases use Qt's standard app-data
directory; `REVIEW_RADAR_CAPTURE_DATABASE` can override the capture path.
`REVIEW_RADAR_COLLECTOR_COMMAND`, `REVIEW_RADAR_QUEUE_COMMAND`, and
`REVIEW_RADAR_STATE_COMMAND` override executable paths. Set
`REVIEW_RADAR_SKIP_COLLECTION=true` to inspect an existing capture without a
GitHub refresh.

On a Linux session that implements `org.freedesktop.Notifications`, the client
sends an alert only for IDs in the queue response's `notificationEligibleIds`.
Those IDs originate from persisted attention transitions, so polling does not
repeat alerts. The notification's **Open pull request** action opens the PR URL
in the default browser.

```sh
cmake -S apps/linux-qt -B build/linux-qt
cmake --build build/linux-qt
./build/linux-qt/review-radar-linux
```

Qt is intentionally a native Linux build; Docker CI continues to verify the Rust
core. Quickshell may launch this binary or show an attention count, but is not
required by the dashboard. The Qt build was verified locally with the same CMake
commands above; runtime GitHub access still requires `GH_TOKEN`.

## Docker desktop session

The Docker target builds the Qt app and Rust executables together. Start it from a
graphical host session:

```sh
./scripts/desktop
```

Compose mounts the host's runtime directory, X11 socket, and Xauthority cookie.
The container chooses Wayland when `$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY` is a socket
(Hyprland), otherwise it selects XCB when `$DISPLAY` has a matching X11 socket
(i3). It mounts the repository's `data/` directory for both the capture and the
local state database. Set `QT_QPA_PLATFORM=wayland` or `QT_QPA_PLATFORM=xcb` to
force a backend while diagnosing a host-specific issue. The script invokes Docker
Compose from the repository root, provides `GH_TOKEN` from `gh auth token` unless
it is already exported, and maps the container to your host UID/GID. For a
Wayland-only session without an Xauthority file, it creates and cleans up an empty
one so the optional X11 bind mount remains valid.
