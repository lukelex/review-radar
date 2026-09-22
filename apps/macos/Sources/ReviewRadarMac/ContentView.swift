import SwiftUI

struct ContentView: View {
    @EnvironmentObject private var queue: QueueStore
    @State private var search = ""

    private var visibleCards: [PullRequestCard] {
        let needle = search.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !needle.isEmpty else { return queue.cards }
        return queue.cards.filter { card in
            [card.title, card.repository, String(card.number)].localizedCaseInsensitiveContains(needle)
        }
    }

    var body: some View {
        NavigationSplitView {
            List {
                Label(Workspace.tailored.title, systemImage: icon(for: .tailored))
            }
            .navigationTitle("Review Radar")
            .safeAreaInset(edge: .bottom) {
                Text("Local to this device")
                    .font(.caption).foregroundStyle(.secondary).padding()
            }
        } content: {
            List(selection: $queue.selectedCardID) {
                if visibleCards.isEmpty { emptyState }
                ForEach(visibleCards) { card in CardRow(card: card).tag(card.id) }
            }
            .searchable(text: $search, prompt: "Search title, repository, or PR number")
            .navigationTitle(Workspace.tailored.title)
            .toolbar { ToolbarItem(placement: .primaryAction) { refreshButton } }
            .safeAreaInset(edge: .bottom) { statusLine }
        } detail: {
            if let card = queue.cards.first(where: { $0.id == queue.selectedCardID }) {
                DetailView(card: card)
            } else {
                ContentUnavailableView("Select a pull request", systemImage: "rectangle.stack", description: Text("The ranked queue remains visible while you inspect details."))
            }
        }
        .frame(minWidth: 860, minHeight: 600)
    }

    private var refreshButton: some View {
        Button { Task { await queue.refresh() } } label: { Label("Refresh", systemImage: "arrow.clockwise") }
            .disabled(queue.phase == .syncing || queue.phase == .loading)
    }

    @ViewBuilder private var emptyState: some View {
        switch queue.phase {
        case .loading: ContentUnavailableView("Loading workspace", systemImage: "arrow.triangle.2.circlepath")
        case .failed(let message): ContentUnavailableView("Could not load workspace", systemImage: "exclamationmark.triangle", description: Text(message))
        default: ContentUnavailableView("No pull requests here", systemImage: "checkmark.circle", description: Text("There are no matching cards in this workspace."))
        }
    }

    private var statusLine: some View {
        HStack(spacing: 6) {
            Circle().frame(width: 6, height: 6).foregroundStyle(statusColor)
            Text(statusText).font(.caption).foregroundStyle(.secondary)
            Spacer()
            if queue.suppressedCount > 0 { Text("\(queue.suppressedCount) hidden on this device").font(.caption).foregroundStyle(.secondary) }
        }.padding(.horizontal).padding(.vertical, 8).background(.bar)
    }

    private var statusText: String {
        switch queue.phase {
        case .loading: "Loading workspace…"
        case .syncing: "Refreshing GitHub in the background…"
        case .fresh: queue.capturedAt.map { "Updated \($0)" } ?? "Local workspace"
        case .stale(let message): "Cached after refresh failure · \(message)"
        case .failed(let message): "Could not load workspace · \(message)"
        }
    }
    private var statusColor: Color { if case .failed = queue.phase { return .red }; if case .stale = queue.phase { return .orange }; return .indigo }
    private func icon(for workspace: Workspace) -> String { switch workspace { case .tailored: "scope"; case .action: "flag"; case .myPrs: "diamond"; case .following: "eye"; case .recent: "clock" } }
}

private struct CardRow: View {
    let card: PullRequestCard
    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            HStack { Text("\(card.repository) #\(card.number)").font(.caption).foregroundStyle(.secondary); Spacer(); Text(card.updatedAt).font(.caption2).foregroundStyle(.tertiary) }
            Text(card.title).font(.headline).lineLimit(2)
            Text(card.explanation.heading).font(.subheadline).foregroundStyle(.indigo).lineLimit(1)
            Text(card.explanation.reasons.first?.summary ?? card.actionLabel).font(.caption).foregroundStyle(.secondary).lineLimit(2)
            HStack { Text(card.actionLabel).font(.caption.weight(.semibold)).padding(.horizontal, 7).padding(.vertical, 4).background(.indigo.opacity(0.12), in: Capsule()); if let friction = card.reviewFriction { Text("Friction · \(friction.level ?? friction.status)").font(.caption).foregroundStyle(.secondary) } }
        }.padding(.vertical, 6)
    }
}

private struct DetailView: View {
    let card: PullRequestCard
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                Text("\(card.repository) #\(card.number)").font(.subheadline).foregroundStyle(.secondary)
                Text(card.title).font(.title2.weight(.semibold))
                GroupBox(card.explanation.heading) { VStack(alignment: .leading, spacing: 9) { ForEach(card.explanation.reasons, id: \.code) { reason in Text(reason.summary) } } }
                GroupBox("Health") { LabeledContent("Review", value: card.explanation.health.reviewDecision ?? (card.explanation.health.isDraft ? "Draft" : "Unknown")); LabeledContent("Checks", value: card.explanation.health.checks ?? "Unknown"); LabeledContent("Merge", value: card.explanation.health.mergeable) }
                HStack {
                    if let url = URL(string: card.explanation.reasons.first?.nextAction?.url ?? card.url) {
                        Button(card.explanation.reasons.first?.nextAction?.label ?? "Open pull request") {
                            queue.openURL(url)
                        }
                        .buttonStyle(.borderedProminent)
                    }
                    if let url = URL(string: card.url) {
                        Button("Copy link") { queue.copyText(url.absoluteString) }
                    }
                }
                Spacer()
            }.padding(28)
        }.navigationTitle("Pull request")
    }
}
