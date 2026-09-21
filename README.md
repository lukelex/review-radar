# Review Radar

An actionable, cross-platform GitHub pull-request workspace.

Review Radar answers three questions quickly:

1. What needs my attention now?
2. Has someone reviewed or commented on one of my pull requests?
3. What is the current health of pull requests I own or follow?

It is intentionally an action queue, not a generic notification feed.

## Architecture

Review Radar uses a shared Rust core with native platform shells:

- Linux: Qt 6/QML application, with optional Quickshell integration.
- macOS: SwiftUI application and menu-bar integration.
- Windows: WinUI 3 application and system-tray integration.

The shared core owns GitHub data, ranking, event transitions, acknowledgement,
and snoozing. Platform shells own only presentation and OS integration.

See [the project plan](docs/project-plan.md) and
[the architecture](docs/architecture.md).

## Repository layout

```text
apps/
  linux-qt/       Linux Qt/QML shell and Quickshell adapter
  macos/          Future SwiftUI shell
  windows/        Future WinUI 3 shell
crates/
  domain/         PR model, prioritisation, and event transitions
  github/         GitHub API client and snapshot normalisation
  state/          Local acknowledgement and snooze persistence
  platform/       Platform-neutral notification/launch contracts
docs/             Product plan, architecture, and decisions
tests/fixtures/   GitHub snapshot fixtures shared by all clients
```

## Status

Planning and repository scaffolding. No production implementation exists yet.
