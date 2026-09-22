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
    let actionLabel: String
    let currentFingerprint: String
    let attentionRequired: Bool
    let lifecycle: String
    let updatedAt: String
    let explanation: Explanation
    let reviewFriction: ReviewFriction?

    struct Explanation: Decodable, Hashable {
        let heading: String
        let reasons: [Reason]
        let health: Health
    }

    struct Reason: Decodable, Hashable {
        let code: String
        let summary: String
        let nextAction: NextAction?
    }

    struct NextAction: Decodable, Hashable {
        let label: String
        let url: String?
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
    }
}

enum Workspace: String, CaseIterable, Identifiable {
    case tailored, action, myPrs = "my-prs", following, recent
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
}
