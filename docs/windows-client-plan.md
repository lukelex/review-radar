# Windows client and packaging plan

Windows WinUI 3 is a supported interface in the native-client contract, but its
shell has not started: `apps/windows/` intentionally contains only a placeholder.
Do not publish an installer or signing pipeline for it before it can run the same
collector, queue, and state-helper contract as the other clients.

## Required before Windows packaging

- Implement the WinUI 3 shell with the five workspace projections, ranking
  selection, full card/details surface, keyboard access, local acknowledgement,
  and snooze commands.
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
