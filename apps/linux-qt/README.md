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
repeat alerts. Browser actions, including **Start review**, open URLs through
the host's desktop OpenURI portal when running in Docker, with a native desktop
fallback when no portal is available.

```sh
cmake -S apps/linux-qt -B build/linux-qt
cmake --build build/linux-qt
./build/linux-qt/review-radar-linux
```

To create a directly usable host bundle containing the Qt app and all Rust
executables it launches, use the repository script:

```sh
./scripts/build-native
GH_TOKEN="$(gh auth token)" ./build/native/bin/review-radar
```

To create the same relocatable archive used by the GitHub alpha release:

```sh
./scripts/package-release 0.1.0-alpha.1
sha256sum --check build/review-radar-linux-x86_64-0.1.0-alpha.1.tar.gz.sha256
```

The archive is Linux x86_64-specific and contains the Qt runtime, plugins, QML
modules, Rust helpers, launcher, checksum companion, and a bundled README. A
tag such as `v0.1.0-alpha.1`, or the **Release Linux alpha** workflow, publishes
the archive as a GitHub prerelease.

The script builds the existing root `Dockerfile`'s `linux-desktop` stage and
extracts the resulting application and Rust helpers under `build/native/bin/`.
The extracted application runs directly on the host with the Qt runtime, plugins,
and QML modules bundled from the Docker image; Docker is only used to compile and
assemble it. It does not copy credentials into the build output. Set
`REVIEW_RADAR_NATIVE_BUILD_DIR` to choose another output directory or
`REVIEW_RADAR_NATIVE_IMAGE` to choose the temporary Docker image tag. The host
only needs a graphical Wayland or X11 session and compatible system graphics,
font, and display libraries.

On Linux, native capture and local-state files are stored in
`$XDG_DATA_HOME/review-radar/` (normally `~/.local/share/review-radar/`).

The Compose desktop launcher applies a shared resource budget to the Qt shell
and its helper processes: 2 CPUs, 1 GiB of memory, and 256 processes by
default. Override these limits with `REVIEW_RADAR_DESKTOP_CPUS`,
`REVIEW_RADAR_DESKTOP_MEMORY`, and `REVIEW_RADAR_DESKTOP_PIDS`; these are Docker
cgroup limits and do not change ranking or refresh semantics.

Qt is intentionally a native Linux build; Docker CI continues to verify the Rust
core. Quickshell may launch this binary or show an attention count, but is not
required by the dashboard. The Qt build was verified locally with the same CMake
commands above; runtime GitHub access still requires `GH_TOKEN`.

## Workspace design

The high-fidelity references and rendered Qt screenshots live in
[`docs/mockups/high`](../../docs/mockups/high/README.md). The workspace uses
shared QML style tokens, cards, badges, and controls. Select a card to open its
health, friction, and captured activity in the detail panel; narrow windows show
details in place of the list. Selection follows the PR identity across model
refreshes and clears when the PR leaves the current dataset.

- **Ctrl+K**: focus local search.
- **Ctrl+R**: refresh GitHub.
- **Escape**: close details, then clear search.
- Browser, mark-read, snooze, and copy-link actions are available in details.

Run `ctest --test-dir build/linux-qt --output-on-failure` after building to check
startup and the populated workspace. The UI test uses isolated illustrative data
and checks search, selection, actions, narrow layout, and model replacement.
With the Docker desktop image, use:

```sh
docker run --rm --user 0 --entrypoint ctest review-radar-ui-check \
  --test-dir /tmp/review-radar-linux-build --output-on-failure
```

Build the split images with `docker compose build`.

## Docker desktop session

The split Docker images build the Rust core and Qt app separately. Start the
desktop image from a graphical host session:

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
