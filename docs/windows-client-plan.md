# Windows client and packaging plan

Windows WinUI 3 is a supported interface in the native-client contract, but its
shell is deliberately being built in slices. Do not publish an installer or
signing pipeline for it before it can run the same collector, queue, and
state-helper contract as the other clients.

## Foundation

- [x] Create the Windows 10+ WinUI 3 project, app manifest, design-system mark,
  and Windows CI build gate.
- [x] Run collector and queue helpers as separate processes, parsing queue stdout
  as one versioned JSON response and retaining stderr for diagnostics.
- [x] Store capture and local-state files under `%APPDATA%\review-radar\`, with
  development command/database overrides and an embedded-helper lookup path.
- [x] Present the already-ranked Tailored projection with card selection, refresh
  on open/five-minute cadence, and explicit local/cached/failure status text.
- [ ] Run and fix the Windows CI build on a hosted Windows runner. Initial run
`35901077769` could not start because the repository Actions budget was
exhausted; no Windows compiler result is available yet.

The approved self-hosted-runner setup, security boundaries, workflow changes, and
LLM handoff instructions are in [`windows-local-ci-plan.md`](windows-local-ci-plan.md).

## Required before Windows packaging

- Implement the remaining four workspace projections, ranking selection, complete
  card/details surface, keyboard access, local acknowledgement, and snooze commands.
- Add Preferences and Windows notifications whose filters and quiet hours affect
  delivery only, never shared attention observation or deduplication.
- Provide a discoverable optional tray/background lifecycle equivalent to Linux
  and macOS, using Windows-native presentation.
- Bundle the Windows-native Rust helper executables with the app; use local data
  paths without shipping credentials or captures.
- Add a Windows CI build/test gate, then an installer artifact, Authenticode
  signing, release upload, and SmartScreen/signing validation.

## Packaging status

No Windows package exists yet. This is an explicit parity gap, not an unsupported
feature: the installer/signing work must accompany the WinUI shell rather than
shipping a placeholder application.
