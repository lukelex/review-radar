import Foundation

struct QueueResponse: Decodable {
    let schemaVersion: Int
    let capturedAt: String
    let view: String
    let ranking: String
    let count: Int
    let pullRequests: [PullRequestCard]
    let sourceCount: Int
    let suppressedCount: Int
    let notificationEligibleIds: [String]
}

struct PullRequestCard: Decodable, Identifiable, Hashable {
    let id: String
    let repository: String
    let number: Int
    let title: String
    let url: String
    let updatedAt: String
    let lifecycle: String
    let memberships: [String]
    let priority: String
    let relationship: String
    let action: String
    let actionLabel: String
    let attentionRequired: Bool
    let explanation: Explanation
    let reviewFriction: ReviewFriction
    let currentFingerprint: String
    let events: [Event]

    struct Explanation: Decodable, Hashable {
        let heading: String
        let reasons: [Reason]
        let health: Health
    }

    struct Reason: Decodable, Hashable {
        let code: String
        let summary: String
        let evidence: [String]
        let nextAction: NextAction
    }

    struct NextAction: Decodable, Hashable {
        let label: String
        let url: String
    }

    struct Health: Decodable, Hashable {
        let reviewDecision: String?
        let checks: String?
        let mergeable: String
        let mergeStateStatus: String
        let isDraft: Bool
    }

    struct ReviewFriction: Decodable, Hashable {
        let status: String
        let level: String?
        let historical: Bool
        let coverage: String?
        let reviewRounds: Int?
        let reviewSeconds: Int?
        let reworkLines: Int?
        let contributors: [Contributor]
        let limitations: [String]

        struct Contributor: Decodable, Hashable {
            let signal: String
            let value: Int
            let unit: String
            let level: String
        }
    }

    struct Event: Decodable, Hashable, Identifiable {
        let fingerprint: String
        let kind: String
        let actor: String?
        let state: String?
        let occurredAt: String

        var id: String { fingerprint }
    }
}

enum Workspace: String, CaseIterable, Identifiable {
    case tailored
    case action
    case myPrs = "my-prs"
    case following
    case recent

    var id: String { rawValue }

    var title: String {
        switch self {
        case .tailored: "Tailored to you"
        case .action: "Action"
        case .myPrs: "My PRs"
        case .following: "Following"
        case .recent: "Recent"
        }
    }

    var subtitle: String {
        switch self {
        case .tailored: "Your next move, in focus."
        case .action: "The work that needs you."
        case .myPrs: "Your work, from draft to done."
        case .following: "Stay close to the conversation."
        case .recent: "A little perspective on what shipped in the last 14 days."
        }
    }

    var symbol: String {
        switch self {
        case .tailored: "scope"
        case .action: "flag"
        case .myPrs: "diamond"
        case .following: "eye"
        case .recent: "clock"
        }
    }
}

enum Ranking: String, CaseIterable, Identifiable {
    case tailored
    case newestActivity = "newest-activity"
    case highestFriction = "highest-friction"

    var id: String { rawValue }

    var title: String {
        switch self {
        case .tailored: "Priority + newest"
        case .newestActivity: "Newest activity"
        case .highestFriction: "Highest friction"
        }
    }
}

enum Display {
    private static let iso8601 = ISO8601DateFormatter()
    private static let relative = RelativeDateTimeFormatter()

    static func humanize(_ value: String) -> String {
        value.replacingOccurrences(of: "_", with: " ")
            .replacingOccurrences(of: "-", with: " ")
            .lowercased()
            .capitalized
    }

    static func age(_ value: String) -> String {
        guard let date = iso8601.date(from: value) else { return value }
        return relative.localizedString(for: date, relativeTo: Date())
    }

    static func frictionDetail(_ assessment: PullRequestCard.ReviewFriction) -> String {
        let contributors = assessment.contributors.map {
            "\(humanize($0.signal)): \($0.value) \($0.unit)"
        }
        let limitations = assessment.limitations.map(humanize)
        let facts = contributors + limitations
        return facts.isEmpty ? "No additional review history is available." : facts.joined(separator: " · ")
    }
}
