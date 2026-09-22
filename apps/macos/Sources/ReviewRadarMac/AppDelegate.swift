import AppKit

final class ReviewRadarAppDelegate: NSObject, NSApplicationDelegate {
    weak var preferences: PreferencesStore?

    func applicationShouldTerminateAfterLastWindowClosed(_: NSApplication) -> Bool {
        guard let preferences else { return true }
        let value = preferences.value
        return !(value.menuBarEnabled && value.keepRunningWhenWindowCloses)
    }
}
