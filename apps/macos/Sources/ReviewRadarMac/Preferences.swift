import Combine
import Foundation
import Security

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
    @Published private(set) var githubTokenConfigured: Bool
    @Published private(set) var githubTokenSaved: Bool
    private let file: URL
    private let keychainService = "org.reviewradar.github"
    private let keychainAccount = "api-token"

    init(directory: URL) {
        file = directory.appendingPathComponent("preferences.json")
        value = Self.load(from: file)
        githubTokenSaved = Self.readGitHubToken(service: keychainService, account: keychainAccount) != nil
        githubTokenConfigured = githubTokenSaved
            || !(ProcessInfo.processInfo.environment["GH_TOKEN"] ?? "").isEmpty
    }

    func save(_ preferences: Preferences) throws {
        guard preferences.validatesQuietHours() else { throw PreferencesError.invalidQuietHours }
        var preferences = preferences
        if !preferences.menuBarEnabled { preferences.keepRunningWhenWindowCloses = false }
        let data = try JSONEncoder().encode(preferences)
        try data.write(to: file, options: .atomic)
        value = preferences
    }

    func githubToken() -> String? {
        Self.readGitHubToken(service: keychainService, account: keychainAccount)
    }

    func saveGitHubToken(_ token: String) throws {
        let token = token.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !token.isEmpty else { throw CredentialError.emptyToken }
        let data = Data(token.utf8)
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: keychainAccount
        ]
        let update: [String: Any] = [kSecValueData as String: data]
        let status = SecItemUpdate(query as CFDictionary, update as CFDictionary)
        var add = query
        add[kSecValueData as String] = data
        add[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        let result = status == errSecItemNotFound ? SecItemAdd(add as CFDictionary, nil) : status
        guard result == errSecSuccess else { throw CredentialError.keychain(result) }
        githubTokenSaved = true
        githubTokenConfigured = true
    }

    func clearGitHubToken() throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: keychainService,
            kSecAttrAccount as String: keychainAccount
        ]
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw CredentialError.keychain(status)
        }
        githubTokenSaved = false
        githubTokenConfigured = !(ProcessInfo.processInfo.environment["GH_TOKEN"] ?? "").isEmpty
    }

    private static func readGitHubToken(service: String, account: String) -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        var result: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &result) == errSecSuccess,
              let data = result as? Data else { return nil }
        return String(data: data, encoding: .utf8)
    }

    enum CredentialError: LocalizedError {
        case keychain(OSStatus)
        case emptyToken
        var errorDescription: String? {
            switch self {
            case .keychain: "Could not update the GitHub token in the macOS Keychain."
            case .emptyToken: "Enter a GitHub token before saving."
            }
        }
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
