import Combine
import Foundation

struct Preferences: Codable, Equatable {
    var notificationsEnabled = true
    var notifyReviewRequests = true
    var notifyFeedback = true
    var notifyChecks = true
    var notifyConflicts = true
    var quietHours = false
    var quietHoursStart = "18:00"
    var quietHoursEnd = "09:00"
    var menuBarEnabled = false
    var keepRunningWhenWindowCloses = false

    func validatesQuietHours() -> Bool {
        Self.time(quietHoursStart) != nil && Self.time(quietHoursEnd) != nil
    }

    func isQuiet(at date: Date = Date()) -> Bool {
        guard quietHours, let start = Self.time(quietHoursStart), let end = Self.time(quietHoursEnd) else { return false }
        let now = Calendar.current.dateComponents([.hour, .minute], from: date)
        let current = (now.hour ?? 0) * 60 + (now.minute ?? 0)
        let startsAt = start.hour * 60 + start.minute
        let endsAt = end.hour * 60 + end.minute
        return startsAt > endsAt ? current >= startsAt || current < endsAt : current >= startsAt && current < endsAt
    }

    func delivers(_ card: PullRequestCard) -> Bool {
        matchingNotificationReason(in: card) != nil
    }

    func matchingNotificationReason(in card: PullRequestCard) -> PullRequestCard.Reason? {
        guard notificationsEnabled, !isQuiet() else { return nil }
        return card.explanation.reasons.first { reason in
            switch reason.code {
            case "review-requested": notifyReviewRequests
            case "changes-requested", "new-feedback": notifyFeedback
            case "checks-failing": notifyChecks
            case "merge-conflict": notifyConflicts
            default: false
            }
        }
    }

    private static func time(_ string: String) -> (hour: Int, minute: Int)? {
        let parts = string.split(separator: ":", omittingEmptySubsequences: false)
        guard parts.count == 2, parts[0].count == 2, parts[1].count == 2,
              let hour = Int(parts[0]), let minute = Int(parts[1]),
              (0..<24).contains(hour), (0..<60).contains(minute) else { return nil }
        return (hour, minute)
    }
}

@MainActor
final class PreferencesStore: ObservableObject {
    @Published private(set) var value: Preferences
    private let file: URL

    init(directory: URL) {
        file = directory.appendingPathComponent("preferences.json")
        value = Self.load(from: file)
    }

    func save(_ preferences: Preferences) throws {
        guard preferences.validatesQuietHours() else { throw PreferencesError.invalidQuietHours }
        let data = try JSONEncoder().encode(preferences)
        try data.write(to: file, options: .atomic)
        value = preferences
    }

    enum PreferencesError: LocalizedError {
        case invalidQuietHours

        var errorDescription: String? {
            "Quiet hours must use valid local times in HH:mm format."
        }
    }

    private static func load(from file: URL) -> Preferences {
        guard let data = try? Data(contentsOf: file), let preferences = try? JSONDecoder().decode(Preferences.self, from: data) else {
            return Preferences()
        }
        return preferences
    }
}
