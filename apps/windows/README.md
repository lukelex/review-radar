# Review Radar for Windows

The Windows client is a WinUI 3 foundation shell for Windows 10 version 1809 or
newer. It renders the versioned, already-ranked Tailored queue through the shared
collector and queue helper boundary; it does not query GitHub or rank cards in C#.

On a Windows development machine with the .NET 8 SDK, run:

```powershell
dotnet run --project apps/windows/ReviewRadar.Windows.csproj
```

The shell stores capture and local-state databases below
`%APPDATA%\review-radar\`. Development overrides match the other native clients:
`REVIEW_RADAR_CAPTURE_DATABASE`, `REVIEW_RADAR_STATE_DATABASE`,
`REVIEW_RADAR_COLLECTOR_COMMAND`, `REVIEW_RADAR_QUEUE_COMMAND`, and
`REVIEW_RADAR_SKIP_COLLECTION`. A packaged build will prefer matching helpers in
`Helpers\` beside the application executable.

See [`docs/windows-client-plan.md`](../../docs/windows-client-plan.md) for the
remaining workspace, interaction, notification, integration, testing, and
packaging work.
