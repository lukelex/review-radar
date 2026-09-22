import AppKit
import SwiftUI

struct MenuBarContent: View {
    @Environment(\.openWindow) private var openWindow
    @EnvironmentObject private var queue: QueueStore

    var body: some View {
        Button("Open Review Radar") { openWorkspace() }
        Button("Refresh") { Task { await queue.refresh() } }
        Button("Preferences") {
            openWorkspace()
            queue.preferencesRequested = true
        }
        Divider()
        Button("Quit Review Radar") { NSApp.terminate(nil) }
    }

    private func openWorkspace() {
        openWindow(id: "review-radar")
        NSApp.activate(ignoringOtherApps: true)
    }
}
