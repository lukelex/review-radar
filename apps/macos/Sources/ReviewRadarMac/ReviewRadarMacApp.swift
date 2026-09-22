import SwiftUI

@main
struct ReviewRadarMacApp: App {
    @StateObject private var queue = QueueStore()

    var body: some Scene {
        WindowGroup("Review Radar") {
            ContentView()
                .environmentObject(queue)
                .environmentObject(queue.preferences)
                .task { await queue.start() }
        }
        .defaultSize(width: 1120, height: 760)
        .commands {
            CommandGroup(after: .appInfo) {
                Button("Refresh Review Radar") { Task { await queue.refresh() } }
                    .keyboardShortcut("r", modifiers: [.command])
            }
        }
    }
}
