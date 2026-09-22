import XCTest
@testable import ReviewRadarMac

@MainActor
final class PreferencesTests: XCTestCase {
    func testOvernightQuietHoursSuppressDelivery() {
        var preferences = Preferences()
        preferences.quietHours = true
        preferences.quietHoursStart = "22:00"
        preferences.quietHoursEnd = "07:00"
        let calendar = Calendar.current
        let overnight = calendar.date(from: DateComponents(year: 2026, month: 9, day: 23, hour: 23, minute: 0))!
        let morning = calendar.date(from: DateComponents(year: 2026, month: 9, day: 24, hour: 8, minute: 0))!
        XCTAssertTrue(preferences.isQuiet(at: overnight))
        XCTAssertFalse(preferences.isQuiet(at: morning))
    }

    func testInvalidQuietHoursDoNotReplaceSavedPreferences() throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let store = PreferencesStore(directory: directory)
        var invalid = store.value
        invalid.quietHoursStart = "not-a-time"
        XCTAssertThrowsError(try store.save(invalid))
        XCTAssertEqual(store.value, Preferences())
    }
}
