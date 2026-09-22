import Foundation
import Combine
import AppKit

protocol OsIntegration {
    var applicationDataDirectory: URL { get }
    func openURL(_ url: URL)
    func copyText(_ text: String)
}

struct MacOsIntegration: OsIntegration {
    let applicationDataDirectory: URL

    init(fileManager: FileManager = .default) {
        let root = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        applicationDataDirectory = root.appendingPathComponent("review-radar", isDirectory: true)
        try? fileManager.createDirectory(at: applicationDataDirectory, withIntermediateDirectories: true)
    }

    func openURL(_ url: URL) {
        NSWorkspace.shared.open(url)
    }

    func copyText(_ text: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }
}

@MainActor
final class QueueStore: ObservableObject {
    @Published private(set) var cards: [PullRequestCard] = []
    @Published private(set) var sourceCount = 0
    @Published private(set) var suppressedCount = 0
    @Published private(set) var capturedAt: String?
    @Published private(set) var phase: Phase = .loading
    @Published var selectedCardID: PullRequestCard.ID?

    enum Phase: Equatable { case loading, syncing, fresh, stale(String), failed(String) }

    private let osIntegration: OsIntegration
    private var refreshTask: Task<Void, Never>?
    private var timer: Timer?

    init(osIntegration: OsIntegration = MacOsIntegration()) {
        self.osIntegration = osIntegration
    }

    deinit { timer?.invalidate(); refreshTask?.cancel() }

    func start() async {
        await refresh()
        timer = Timer.scheduledTimer(withTimeInterval: 5 * 60, repeats: true) { [weak self] _ in
            Task { await self?.refresh() }
        }
    }

    func refresh() async {
        guard refreshTask == nil else { return }
        refreshTask = Task {
            await loadProjection(recordAttention: true)
            guard !ProcessInfo.processInfo.environment.keys.contains("REVIEW_RADAR_SKIP_COLLECTION") else {
                refreshTask = nil
                return
            }
            guard await collect() else { refreshTask = nil; return }
            await loadProjection(recordAttention: true)
            refreshTask = nil
        }
        await refreshTask?.value
    }

    func loadProjection(recordAttention: Bool = false) async {
        if !cards.isEmpty { phase = .syncing } else { phase = .loading }
        do {
            let response = try await Helpers.queue(
                workspace: .tailored,
                recordAttention: recordAttention,
                osIntegration: osIntegration
            )
            guard response.schemaVersion == 1 else {
                throw Helpers.HelperError.unsupportedQueueSchema(response.schemaVersion)
            }
            cards = response.pullRequests
            sourceCount = response.sourceCount
            suppressedCount = response.suppressedCount
            capturedAt = response.capturedAt
            phase = .fresh
            if let id = selectedCardID, !cards.contains(where: { $0.id == id }) { selectedCardID = nil }
        } catch {
            phase = cards.isEmpty ? .failed(error.localizedDescription) : .stale(error.localizedDescription)
        }
    }

    func openURL(_ url: URL) {
        osIntegration.openURL(url)
    }

    func copyText(_ text: String) {
        osIntegration.copyText(text)
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
}

enum Helpers {
    static func queue(workspace: Workspace, recordAttention: Bool,
                      osIntegration: OsIntegration) async throws -> QueueResponse {
        let output = try await run(command("REVIEW_RADAR_QUEUE_COMMAND", fallback: "review-radar-queue"), [
            "--database", captureDatabase(osIntegration).path,
            "--state-database", stateDatabase(osIntegration).path,
            "--view", workspace.rawValue, "--ranking", "tailored", "--record-attention", recordAttention ? "true" : "false"
        ])
        return try JSONDecoder().decode(QueueResponse.self, from: output.stdout)
    }

    static func collect(osIntegration: OsIntegration) async throws {
        _ = try await run(command("REVIEW_RADAR_COLLECTOR_COMMAND", fallback: "review-radar-github"), [
            "--database", captureDatabase(osIntegration).path
        ])
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
            process.executableURL = executable; process.arguments = arguments
            process.standardOutput = stdout; process.standardError = stderr
            process.terminationHandler = { completed in
                Task {
                    let output = Output(
                        stdout: await stdoutReader.value,
                        stderr: await stderrReader.value
                    )
                    if completed.terminationStatus == 0 { continuation.resume(returning: output) }
                    else { continuation.resume(throwing: HelperError.processFailed(executable, output.stderr)) }
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

        var errorDescription: String? {
            switch self {
            case let .processFailed(executable, stderr):
                let detail = String(decoding: stderr, as: UTF8.self)
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                return detail.isEmpty ? "Could not run \(executable.lastPathComponent)." : detail
            case let .unsupportedQueueSchema(version):
                return "This app does not support queue schema version \(version)."
            }
        }
    }
}
