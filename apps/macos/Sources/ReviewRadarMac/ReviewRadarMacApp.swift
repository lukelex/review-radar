import SwiftUI

@main
struct ReviewRadarMacApp: App {
    @NSApplicationDelegateAdaptor(ReviewRadarAppDelegate.self) private var appDelegate
    @StateObject private var preferences: PreferencesStore
    @StateObject private var queue: QueueStore

    init() {
        let integration = MacOsIntegration()
        let preferences = PreferencesStore(directory: integration.applicationDataDirectory)
        _preferences = StateObject(wrappedValue: preferences)
        _queue = StateObject(wrappedValue: QueueStore(osIntegration: integration, preferences: preferences))
    }

    var body: some Scene {
        WindowGroup("Review Radar", id: "review-radar") {
            ContentView()
                .environmentObject(queue)
                .environmentObject(preferences)
                .task {
                    appDelegate.preferences = preferences
                    await queue.start()
                }
        }
        .defaultSize(width: 1120, height: 760)
        .commands {
            CommandGroup(after: .appInfo) {
                Button("Refresh Review Radar") { Task { await queue.refresh() } }
                    .keyboardShortcut("r", modifiers: [.command])
                Button("Preferences…") { queue.preferencesRequested = true }
                    .keyboardShortcut(",", modifiers: [.command])
            }
        }

        MenuBarExtra(isInserted: menuBarInserted) {
            MenuBarContent().environmentObject(queue)
        } label: {
            HStack(spacing: 3) {
                Image(systemName: "scope")
                if queue.attentionCount > 0 { Text("\(queue.attentionCount)") }
            }
            .accessibilityLabel("Review Radar, \(queue.attentionCount) needing attention in the current workspace")
        }
        .menuBarExtraStyle(.menu)
    }

    private var menuBarInserted: Binding<Bool> {
        Binding(
            get: { preferences.value.menuBarEnabled },
            set: { enabled in
                var updated = preferences.value
                updated.menuBarEnabled = enabled
                if !enabled { updated.keepRunningWhenWindowCloses = false }
                try? preferences.save(updated)
            }
        )
    }
}
