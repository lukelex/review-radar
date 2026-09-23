import AppKit
import Combine
import Foundation
import UserNotifications

struct NotificationAction {
    let id: String
    let label: String
    let url: URL
}

struct NotificationRequest {
    let id: String
    let title: String
    let body: String
    let actions: [NotificationAction]
}

protocol OsIntegration {
    var applicationDataDirectory: URL { get }
    func openURL(_ url: URL)
    func copyText(_ text: String)
    func showNotification(_ request: NotificationRequest) async -> Bool
    func setNotificationActivationHandler(_ handler: @escaping (URL) -> Void)
}

final class MacOsIntegration: NSObject, OsIntegration, UNUserNotificationCenterDelegate {
    let applicationDataDirectory: URL
    private let notificationCenter = UNUserNotificationCenter.current()
    private var notificationActivationHandler: ((URL) -> Void)?

    init(fileManager: FileManager = .default) {
        let root = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        applicationDataDirectory = root.appendingPathComponent("review-radar", isDirectory: true)
        try? fileManager.createDirectory(at: applicationDataDirectory, withIntermediateDirectories: true)
        super.init()
        notificationCenter.delegate = self
        notificationCenter.setNotificationCategories([UNNotificationCategory(
            identifier: "org.reviewradar.pull-request",
            actions: [
                UNNotificationAction(identifier: "next-action", title: "Open next action", options: [.foreground]),
                UNNotificationAction(identifier: "open-pull-request", title: "Open pull request", options: [.foreground])
            ],
            intentIdentifiers: []
        )])
    }

    func openURL(_ url: URL) { NSWorkspace.shared.open(url) }

    func copyText(_ text: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }

    func showNotification(_ request: NotificationRequest) async -> Bool {
        do {
            let granted = try await notificationCenter.requestAuthorization(options: [.alert, .badge, .sound])
            guard granted else { return false }
            let content = UNMutableNotificationContent()
            content.title = request.title
            content.body = request.body
            content.threadIdentifier = request.id
            content.categoryIdentifier = request.actions.isEmpty ? "" : "org.reviewradar.pull-request"
            content.userInfo = Dictionary(uniqueKeysWithValues: request.actions.map { ($0.id, $0.url.absoluteString) })
            try await notificationCenter.add(UNNotificationRequest(identifier: request.id, content: content, trigger: nil))
            return true
        } catch {
            return false
        }
    }

    func setNotificationActivationHandler(_ handler: @escaping (URL) -> Void) {
        notificationActivationHandler = handler
    }

    func userNotificationCenter(_: UNUserNotificationCenter, willPresent _: UNNotification,
                                withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void) {
        completionHandler([.banner, .sound])
    }

    func userNotificationCenter(_: UNUserNotificationCenter, didReceive response: UNNotificationResponse,
                                withCompletionHandler completionHandler: @escaping () -> Void) {
        defer { completionHandler() }
        let action = response.actionIdentifier == UNNotificationDefaultActionIdentifier ? "open-pull-request" : response.actionIdentifier
        guard let value = response.notification.request.content.userInfo[action] as? String,
              let url = URL(string: value) else { return }
        notificationActivationHandler?(url)
    }
}

@MainActor
final class QueueStore: ObservableObject {
    enum Phase: Equatable { case loading, syncing, fresh, stale(String), failed(String) }

    @Published private(set) var cards: [PullRequestCard] = []
    @Published private(set) var sourceCount = 0
    @Published private(set) var suppressedCount = 0
    @Published private(set) var capturedAt: String?
    @Published private(set) var phase: Phase = .loading
    @Published private(set) var actionError: String?
    @Published var workspace: Workspace = .tailored {
        didSet { if workspace != oldValue { projectionChanged() } }
    }
    @Published var ranking: Ranking = .tailored {
        didSet { if ranking != oldValue { projectionChanged() } }
    }
    @Published var selectedCardID: PullRequestCard.ID?
    @Published var navigationCardID: PullRequestCard.ID?
    @Published var search = ""
    @Published var controlHeld = false
    @Published var preferencesRequested = false

    private struct ProjectionKey: Hashable {
        let workspace: Workspace
        let ranking: Ranking
    }

    private var cache: [ProjectionKey: QueueResponse] = [:]
    private let osIntegration: OsIntegration
    let preferences: PreferencesStore
    private var refreshTask: Task<Void, Never>?
    private var timer: Timer?

    init(osIntegration: OsIntegration = MacOsIntegration(), preferences: PreferencesStore? = nil) {
        self.osIntegration = osIntegration
        self.preferences = preferences ?? PreferencesStore(directory: osIntegration.applicationDataDirectory)
        osIntegration.setNotificationActivationHandler { [weak self] url in
            Task { @MainActor in self?.openURL(url) }
        }
    }

    deinit {
        timer?.invalidate()
        refreshTask?.cancel()
    }

    var selectedCard: PullRequestCard? {
        cards.first(where: { $0.id == selectedCardID })
    }

    var navigationCard: PullRequestCard? {
        cards.first(where: { $0.id == navigationCardID })
    }

    var attentionCount: Int { cards.filter(\.attentionRequired).count }

    var filteredCards: [PullRequestCard] {
        let needle = search.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !needle.isEmpty else { return cards }
        return cards.filter { card in
            [card.title, card.repository, String(card.number)]
                .contains { $0.localizedCaseInsensitiveContains(needle) }
        }
    }

    func start() async {
        await refresh()
        timer = Timer.scheduledTimer(withTimeInterval: 5 * 60, repeats: true) { [weak self] _ in
            guard let self else { return }
            Task { @MainActor in await self.refresh() }
        }
    }

    func refresh() async {
        guard refreshTask == nil else { return }
        let task = Task { [weak self] in
            guard let self else { return }
            await self.loadProjection(recordAttention: true)
            guard !ProcessInfo.processInfo.environment.keys.contains("REVIEW_RADAR_SKIP_COLLECTION") else {
                self.refreshTask = nil
                return
            }
            guard await self.collect() else {
                self.refreshTask = nil
                return
            }
            await self.loadProjection(recordAttention: true)
            self.refreshTask = nil
        }
        refreshTask = task
        await task.value
    }

    func acknowledge(_ card: PullRequestCard) async -> Bool {
        await applyStateCommand("acknowledge", card: card, until: nil)
    }

    func snooze(_ card: PullRequestCard, preset: SnoozePreset) async -> Bool {
        await applyStateCommand("snooze", card: card, until: preset.until())
    }

    func openURL(_ url: URL) { osIntegration.openURL(url) }
    func copyText(_ text: String) { osIntegration.copyText(text) }
    func clearActionError() { actionError = nil }

    func testNotification() async -> Bool {
        await osIntegration.showNotification(NotificationRequest(
            id: "review-radar-test",
            title: "Review Radar test",
            body: "Desktop notifications are connected.",
            actions: []
        ))
    }

    func moveNavigation(by amount: Int) {
        let cards = filteredCards
        guard !cards.isEmpty else { return }
        let current = navigationCardID.flatMap { id in cards.firstIndex(where: { $0.id == id }) }
        let startingIndex = current ?? (amount > 0 ? -1 : cards.count)
        let target = (startingIndex + amount).quotientAndRemainder(dividingBy: cards.count)
        let index = target.remainder >= 0 ? target.remainder : target.remainder + cards.count
        navigationCardID = cards[index].id
        if selectedCardID != nil { selectedCardID = cards[index].id }
    }

    func openNavigationDetails() {
        if let navigationCardID { selectedCardID = navigationCardID }
    }

    func closeDetails() { selectedCardID = nil }

    func resetWorkspaceFocus() {
        selectedCardID = nil
        search = ""
        navigationCardID = nil
    }

    func nextWorkspace() {
        guard let index = Workspace.allCases.firstIndex(of: workspace) else { return }
        workspace = Workspace.allCases[(index + 1) % Workspace.allCases.count]
    }

    func selectWorkspace(_ index: Int) {
        guard Workspace.allCases.indices.contains(index) else { return }
        workspace = Workspace.allCases[index]
    }

    private func projectionChanged() {
        selectedCardID = nil
        navigationCardID = nil
        applyCachedProjection()
        Task { await loadProjection() }
    }

    private func currentKey() -> ProjectionKey { ProjectionKey(workspace: workspace, ranking: ranking) }

    private func applyCachedProjection() {
        guard let cached = cache[currentKey()] else {
            cards = []
            sourceCount = 0
            suppressedCount = 0
            capturedAt = nil
            phase = .loading
            return
        }
        apply(cached)
        phase = .syncing
    }

    private func loadProjection(recordAttention: Bool = false) async {
        let requested = currentKey()
        if requested == currentKey() {
            phase = cache[requested] == nil ? .loading : .syncing
        }
        do {
            let response = try await Helpers.queue(
                workspace: requested.workspace,
                ranking: requested.ranking,
                recordAttention: recordAttention,
                osIntegration: osIntegration
            )
            guard response.schemaVersion == 1 else {
                throw Helpers.HelperError.unsupportedQueueSchema(response.schemaVersion)
            }
            guard response.view == requested.workspace.rawValue, response.ranking == requested.ranking.rawValue else {
                throw Helpers.HelperError.invalidProjection
            }
            cache[requested] = response
            if requested == currentKey() {
                apply(response)
                await sendNotifications(response)
                phase = .fresh
            }
        } catch {
            guard requested == currentKey() else { return }
            phase = cards.isEmpty ? .failed(error.localizedDescription) : .stale(error.localizedDescription)
        }
    }

    private func apply(_ response: QueueResponse) {
        cards = response.pullRequests
        sourceCount = response.sourceCount
        suppressedCount = response.suppressedCount
        capturedAt = response.capturedAt
        if let id = selectedCardID, !cards.contains(where: { $0.id == id }) {
            selectedCardID = nil
        }
        if let id = navigationCardID, !cards.contains(where: { $0.id == id }) {
            navigationCardID = nil
        }
    }

    private func sendNotifications(_ response: QueueResponse) async {
        let eligible = Set(response.notificationEligibleIds)
        for card in response.pullRequests where eligible.contains(card.id) {
            guard let reason = preferences.value.matchingNotificationReason(in: card),
                  let next = URL(string: reason.nextAction.url),
                  let canonical = URL(string: card.url) else { continue }
            var actions = [NotificationAction(id: "next-action", label: reason.nextAction.label, url: next)]
            if canonical != next {
                actions.append(NotificationAction(id: "open-pull-request", label: "Open pull request", url: canonical))
            }
            _ = await osIntegration.showNotification(NotificationRequest(
                id: card.currentFingerprint,
                title: card.actionLabel,
                body: "\(card.repository) #\(card.number)\n\(reason.summary)",
                actions: actions
            ))
        }
    }

    private func collect() async -> Bool {
        do {
            try await Helpers.collect(osIntegration: osIntegration)
            return true
        } catch {
            phase = cards.isEmpty ? .failed(error.localizedDescription) : .stale(error.localizedDescription)
            return false
        }
    }

    private func applyStateCommand(_ command: String, card: PullRequestCard, until: Date?) async -> Bool {
        actionError = nil
        do {
            try await Helpers.state(command, card: card, until: until, osIntegration: osIntegration)
            await loadProjection()
            return true
        } catch {
            actionError = error.localizedDescription
            return false
        }
    }
}

enum SnoozePreset: CaseIterable, Identifiable {
    case laterToday
    case tomorrow
    case nextWeek

    var id: Self { self }
    var title: String {
        switch self {
        case .laterToday: "Later today"
        case .tomorrow: "Tomorrow"
        case .nextWeek: "Next week"
        }
    }

    func until(now: Date = Date()) -> Date {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(secondsFromGMT: 0)!
        switch self {
        case .laterToday: return now.addingTimeInterval(4 * 60 * 60)
        case .tomorrow: return calendar.date(bySettingHour: 9, minute: 0, second: 0, of: calendar.date(byAdding: .day, value: 1, to: now)!)!
        case .nextWeek: return calendar.date(bySettingHour: 9, minute: 0, second: 0, of: calendar.date(byAdding: .day, value: 7, to: now)!)!
        }
    }
}

enum Helpers {
    static func queue(workspace: Workspace, ranking: Ranking, recordAttention: Bool,
                      osIntegration: OsIntegration) async throws -> QueueResponse {
        let output = try await run(command("REVIEW_RADAR_QUEUE_COMMAND", fallback: "review-radar-queue"), [
            "--database", captureDatabase(osIntegration).path,
            "--state-database", stateDatabase(osIntegration).path,
            "--view", workspace.rawValue,
            "--ranking", ranking.rawValue,
            "--record-attention", recordAttention ? "true" : "false"
        ])
        return try JSONDecoder().decode(QueueResponse.self, from: output.stdout)
    }

    static func collect(osIntegration: OsIntegration) async throws {
        _ = try await run(command("REVIEW_RADAR_COLLECTOR_COMMAND", fallback: "review-radar-github"), [
            "--database", captureDatabase(osIntegration).path
        ])
    }

    static func state(_ commandName: String, card: PullRequestCard, until: Date?,
                      osIntegration: OsIntegration) async throws {
        var arguments = [
            "--database", stateDatabase(osIntegration).path,
            commandName,
            "--pull-request-id", card.id,
            "--fingerprint", card.currentFingerprint
        ]
        if let until {
            arguments += ["--until", ISO8601DateFormatter().string(from: until)]
        }
        _ = try await run(command("REVIEW_RADAR_STATE_COMMAND", fallback: "review-radar-state"), arguments)
    }

    private static func captureDatabase(_ osIntegration: OsIntegration) -> URL {
        override("REVIEW_RADAR_CAPTURE_DATABASE") ?? osIntegration.applicationDataDirectory.appendingPathComponent("review-radar.sqlite3")
    }

    private static func stateDatabase(_ osIntegration: OsIntegration) -> URL {
        override("REVIEW_RADAR_STATE_DATABASE") ?? osIntegration.applicationDataDirectory.appendingPathComponent("review-radar-state.sqlite3")
    }

    private static func override(_ name: String) -> URL? {
        guard let path = ProcessInfo.processInfo.environment[name], !path.isEmpty else { return nil }
        return URL(fileURLWithPath: path)
    }

    private static func command(_ name: String, fallback: String) -> URL {
        let path = ProcessInfo.processInfo.environment[name] ?? fallback
        if path.contains("/") { return URL(fileURLWithPath: path) }
        let directories = (ProcessInfo.processInfo.environment["PATH"] ?? "")
            .split(separator: ":")
            .map(String.init)
        if let executable = directories
            .map({ URL(fileURLWithPath: $0).appendingPathComponent(path) })
            .first(where: { FileManager.default.isExecutableFile(atPath: $0.path) }) {
            return executable
        }
        return URL(fileURLWithPath: path)
    }

    struct Output { let stdout: Data; let stderr: Data }

    static func run(_ executable: URL, _ arguments: [String]) async throws -> Output {
        try await withCheckedThrowingContinuation { continuation in
            let process = Process(), stdout = Pipe(), stderr = Pipe()
            let stdoutReader = Task.detached { stdout.fileHandleForReading.readDataToEndOfFile() }
            let stderrReader = Task.detached { stderr.fileHandleForReading.readDataToEndOfFile() }
            process.executableURL = executable
            process.arguments = arguments
            process.standardOutput = stdout
            process.standardError = stderr
            process.terminationHandler = { completed in
                Task {
                    let output = Output(stdout: await stdoutReader.value, stderr: await stderrReader.value)
                    if completed.terminationStatus == 0 {
                        continuation.resume(returning: output)
                    } else {
                        continuation.resume(throwing: HelperError.processFailed(executable, output.stderr))
                    }
                }
            }
            do {
                try process.run()
            } catch let error {
                stdout.fileHandleForWriting.closeFile()
                stderr.fileHandleForWriting.closeFile()
                stdoutReader.cancel()
                stderrReader.cancel()
                continuation.resume(throwing: error)
            }
        }
    }

    enum HelperError: LocalizedError {
        case processFailed(URL, Data)
        case unsupportedQueueSchema(Int)
        case invalidProjection

        var errorDescription: String? {
            switch self {
            case let .processFailed(executable, stderr):
                let detail = String(decoding: stderr, as: UTF8.self)
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                return detail.isEmpty ? "Could not run \(executable.lastPathComponent)." : detail
            case let .unsupportedQueueSchema(version):
                return "This app does not support queue schema version \(version)."
            case .invalidProjection:
                return "The queue response did not match the requested workspace."
            }
        }
    }
}
