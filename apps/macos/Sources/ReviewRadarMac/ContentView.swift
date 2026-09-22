import SwiftUI

struct ContentView: View {
    @EnvironmentObject private var queue: QueueStore
    @State private var keyboard = KeyboardMonitor()
    @FocusState private var searchFocused: Bool
    @State private var acknowledgementTarget: PullRequestCard?
    @State private var snoozeTarget: PullRequestCard?
    @State private var showShortcutHelp = false
    @State private var copied = false

    private var visibleCards: [PullRequestCard] { queue.filteredCards }

    var body: some View {
        NavigationSplitView {
            List(selection: workspaceSelection) {
                ForEach(Workspace.allCases.indices, id: \.self) { index in
                    let workspace = Workspace.allCases[index]
                    HStack(spacing: 6) {
                        if queue.controlHeld {
                            Text("\(index + 1)").font(.caption.weight(.semibold)).foregroundStyle(.secondary)
                        }
                        Label(workspace.title, systemImage: workspace.symbol)
                    }
                    .tag(workspace)
                }
            }
            .navigationTitle("Review Radar")
            .safeAreaInset(edge: .bottom) { localStateSummary }
        } content: {
            VStack(spacing: 0) {
                workspaceHeader
                ScrollViewReader { proxy in
                    List(selection: $queue.selectedCardID) {
                        if visibleCards.isEmpty { emptyState }
                        ForEach(visibleCards) { card in
                            CardRow(card: card)
                                .tag(card.id)
                                .id(card.id)
                                .listRowBackground(card.id == queue.navigationCardID ? Color.indigo.opacity(0.08) : Color.clear)
                                .contextMenu { cardActions(card) }
                        }
                    }
                    .onChange(of: queue.navigationCardID) { id in
                        if let id { proxy.scrollTo(id, anchor: .center) }
                    }
                    .onChange(of: queue.selectedCardID) { id in
                        if let id { queue.navigationCardID = id }
                    }
                }
                .searchable(text: $queue.search, prompt: "Search title, repository, or PR number")
                .searchFocused($searchFocused)
                .safeAreaInset(edge: .bottom) { statusLine }
            }
            .navigationTitle(queue.workspace.title)
            .toolbar {
                ToolbarItem(placement: .primaryAction) { refreshButton }
                ToolbarItem(placement: .automatic) {
                    Picker("Sort pull requests", selection: $queue.ranking) {
                        ForEach(Ranking.allCases) { ranking in Text(ranking.title).tag(ranking) }
                    }
                    .labelsHidden()
                    .accessibilityLabel("Sort pull requests")
                }
            }
        } detail: {
            if let card = queue.selectedCard {
                DetailView(card: card, copied: $copied) { action in
                    perform(action, for: card)
                }
            } else {
                ContentUnavailableView(
                    "Select a pull request",
                    systemImage: "rectangle.stack",
                    description: Text("The ranked queue remains visible while you inspect details.")
                )
            }
        }
        .frame(minWidth: 860, minHeight: 600)
        .confirmationDialog(
            "Mark pull request as read?",
            isPresented: acknowledgementBinding,
            titleVisibility: .visible
        ) {
            Button("Mark read") {
                if let card = acknowledgementTarget { Task { _ = await queue.acknowledge(card) } }
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("This pull request stays quiet until a meaningful event changes.")
        }
        .confirmationDialog("Snooze pull request", isPresented: snoozeBinding, titleVisibility: .visible) {
            ForEach(SnoozePreset.allCases) { preset in
                Button(preset.title) {
                    if let card = snoozeTarget { Task { _ = await queue.snooze(card, preset: preset) } }
                }
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("The pull request returns when this time passes or a meaningful event changes.")
        }
        .alert("Could not update local state", isPresented: actionErrorBinding) {
            Button("OK", role: .cancel) { queue.clearActionError() }
        } message: {
            Text(queue.actionError ?? "Try again.")
        }
        .sheet(isPresented: $showShortcutHelp) { ShortcutHelp() }
        .onAppear {
            keyboard.start(handleKeyboardAction, controlChanged: { queue.controlHeld = $0 })
        }
        .onDisappear { keyboard.stop() }
    }

    private var actionErrorBinding: Binding<Bool> {
        Binding(get: { queue.actionError != nil }, set: { if !$0 { queue.clearActionError() } })
    }

    private var acknowledgementBinding: Binding<Bool> {
        Binding(get: { acknowledgementTarget != nil }, set: { if !$0 { acknowledgementTarget = nil } })
    }

    private var snoozeBinding: Binding<Bool> {
        Binding(get: { snoozeTarget != nil }, set: { if !$0 { snoozeTarget = nil } })
    }

    private var workspaceSelection: Binding<Workspace?> {
        Binding(
            get: { queue.workspace },
            set: { if let workspace = $0 { queue.workspace = workspace } }
        )
    }

    private var workspaceHeader: some View {
        HStack(alignment: .bottom, spacing: 16) {
            VStack(alignment: .leading, spacing: 5) {
                Text("WORKSPACE / \(queue.workspace.title.uppercased())")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
                Text(queue.workspace.title).font(.title2.weight(.semibold))
                Text(queue.workspace.subtitle).font(.subheadline).foregroundStyle(.secondary)
            }
            Spacer()
            Text("\(visibleCards.count) \(visibleCards.count == 1 ? "pull request" : "pull requests")")
                .font(.caption.weight(.medium))
                .foregroundStyle(.secondary)
                .padding(.horizontal, 9).padding(.vertical, 6)
                .background(.quaternary, in: Capsule())
        }
        .padding(.horizontal, 24).padding(.vertical, 20)
    }

    private var refreshButton: some View {
        Button { Task { await queue.refresh() } } label: {
            Label("Refresh", systemImage: "arrow.clockwise")
        }
        .disabled(queue.phase == .syncing || queue.phase == .loading)
        .keyboardShortcut("r", modifiers: [.command])
    }

    @ViewBuilder private var emptyState: some View {
        switch queue.phase {
        case .loading:
            ContentUnavailableView("Loading workspace", systemImage: "arrow.triangle.2.circlepath")
        case .failed(let message):
            ContentUnavailableView("Could not load workspace", systemImage: "exclamationmark.triangle", description: Text(message))
        default:
            ContentUnavailableView("No pull requests here", systemImage: "checkmark.circle", description: Text("There are no matching cards in this workspace."))
        }
    }

    private var statusLine: some View {
        HStack(spacing: 6) {
            Circle().frame(width: 6, height: 6).foregroundStyle(statusColor)
            Text(statusText).font(.caption).foregroundStyle(.secondary)
            Spacer()
            if queue.suppressedCount > 0 {
                Text("\(queue.suppressedCount) hidden on this device")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .padding(.horizontal).padding(.vertical, 8).background(.bar)
    }

    private var localStateSummary: some View {
        VStack(alignment: .leading, spacing: 5) {
            Text("LOCAL TO THIS DEVICE").font(.caption2.weight(.semibold)).foregroundStyle(.secondary)
            Text("Read and snoozed items stay quiet until something meaningful changes.")
                .font(.caption).foregroundStyle(.secondary)
            if queue.suppressedCount > 0 {
                Text("\(queue.suppressedCount) hidden in this view")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .padding()
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

    private var statusColor: Color {
        if case .failed = queue.phase { return .red }
        if case .stale = queue.phase { return .orange }
        return .indigo
    }

    @ViewBuilder private func cardActions(_ card: PullRequestCard) -> some View {
        Button(card.explanation.reasons.first?.nextAction.label ?? "Open pull request") {
            openNextAction(for: card)
        }
        Button("Copy link") { queue.copyText(card.url) }
        Divider()
        Button("Mark read") { acknowledgementTarget = card }
        Menu("Snooze") {
            ForEach(SnoozePreset.allCases) { preset in
                Button(preset.title) { Task { _ = await queue.snooze(card, preset: preset) } }
            }
        }
    }

    private func perform(_ action: DetailAction, for card: PullRequestCard) {
        switch action {
        case .openNextAction: openNextAction(for: card)
        case .openCanonical: if let url = URL(string: card.url) { queue.openURL(url) }
        case .copy: queue.copyText(card.url); copied = true
        case .acknowledge: acknowledgementTarget = card
        case .snooze(let preset): Task { _ = await queue.snooze(card, preset: preset) }
        }
    }

    private func openNextAction(for card: PullRequestCard) {
        let destination = card.explanation.reasons.first?.nextAction.url ?? card.url
        if let url = URL(string: destination) { queue.openURL(url) }
    }

    private func handleKeyboardAction(_ action: KeyboardAction) {
        switch action {
        case .moveNext: queue.moveNavigation(by: 1)
        case .movePrevious: queue.moveNavigation(by: -1)
        case .pageDown: queue.moveNavigation(by: 8)
        case .pageUp: queue.moveNavigation(by: -8)
        case .openDetails: queue.openNavigationDetails()
        case .closeDetails: queue.closeDetails()
        case .openCanonical:
            if let card = queue.selectedCard ?? queue.navigationCard, let url = URL(string: card.url) { queue.openURL(url) }
        case .acknowledge:
            acknowledgementTarget = queue.selectedCard ?? queue.navigationCard
        case .snooze:
            snoozeTarget = queue.selectedCard ?? queue.navigationCard
        case .copy:
            if let card = queue.selectedCard ?? queue.navigationCard { queue.copyText(card.url) }
        case .focusSearch:
            searchFocused = true
        case .resetWorkspace:
            queue.resetWorkspaceFocus()
            searchFocused = false
        case .showHelp: showShortcutHelp = true
        case .nextWorkspace: queue.nextWorkspace()
        case .selectWorkspace(let index): queue.selectWorkspace(index)
        }
    }
}

private struct CardRow: View {
    let card: PullRequestCard

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("\(card.repository) #\(card.number)").font(.caption).foregroundStyle(.secondary)
                Spacer()
                Text(Display.age(card.updatedAt)).font(.caption2).foregroundStyle(.tertiary)
            }
            Text(card.title).font(.headline).lineLimit(2)
            VStack(alignment: .leading, spacing: 4) {
                Text(card.explanation.heading).font(.subheadline.weight(.semibold))
                    .foregroundStyle(card.attentionRequired ? .indigo : .secondary)
                ForEach(card.explanation.reasons, id: \.code) { reason in
                    Text(reason.summary).font(.caption).foregroundStyle(.secondary).lineLimit(2)
                }
            }
            HealthSummary(health: card.explanation.health)
            HStack(spacing: 7) {
                Text(card.actionLabel).font(.caption.weight(.semibold)).padding(.horizontal, 7).padding(.vertical, 4)
                    .background(card.attentionRequired ? .indigo.opacity(0.12) : .quaternary, in: Capsule())
                Text(Display.humanize(card.lifecycle)).font(.caption).foregroundStyle(.secondary)
                FrictionLabel(friction: card.reviewFriction)
            }
        }
        .padding(.vertical, 7)
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(card.repository) pull request \(card.number). \(card.title). \(card.explanation.heading).")
    }
}

private enum DetailAction {
    case openNextAction, openCanonical, copy, acknowledge, snooze(SnoozePreset)
}

private struct DetailView: View {
    let card: PullRequestCard
    @Binding var copied: Bool
    let perform: (DetailAction) -> Void

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                Text("\(card.repository) #\(card.number)").font(.subheadline).foregroundStyle(.secondary)
                Text(card.title).font(.title2.weight(.semibold))
                HStack(spacing: 8) {
                    Text(Display.humanize(card.lifecycle)).badgeStyle()
                    Text(card.actionLabel).badgeStyle()
                }
                GroupBox(card.explanation.heading) {
                    VStack(alignment: .leading, spacing: 10) {
                        ForEach(card.explanation.reasons, id: \.code) { reason in
                            VStack(alignment: .leading, spacing: 4) {
                                Text(reason.summary)
                                Text("Evidence: \(reason.evidence.map(Display.humanize).joined(separator: ", "))")
                                    .font(.caption).foregroundStyle(.secondary)
                            }
                        }
                    }.frame(maxWidth: .infinity, alignment: .leading)
                }
                HStack {
                    Button(card.explanation.reasons.first?.nextAction.label ?? "Open pull request") {
                        perform(.openNextAction)
                    }.buttonStyle(.borderedProminent)
                    Button("Mark read") { perform(.acknowledge) }
                    Menu("Snooze") {
                        ForEach(SnoozePreset.allCases) { preset in
                            Button(preset.title) { perform(.snooze(preset)) }
                        }
                    }
                    Button(copied ? "Copied" : "Copy link") { perform(.copy) }
                }
                GroupBox("PR health") { HealthSummary(health: card.explanation.health).frame(maxWidth: .infinity, alignment: .leading) }
                GroupBox("Review friction") {
                    VStack(alignment: .leading, spacing: 8) {
                        FrictionLabel(friction: card.reviewFriction)
                        Text(Display.frictionDetail(card.reviewFriction)).font(.subheadline).foregroundStyle(.secondary)
                    }.frame(maxWidth: .infinity, alignment: .leading)
                }
                GroupBox("Activity") {
                    if card.events.isEmpty {
                        Text("No captured activity is available.").foregroundStyle(.secondary)
                    } else {
                        VStack(alignment: .leading, spacing: 10) {
                            ForEach(card.events) { event in
                                VStack(alignment: .leading, spacing: 2) {
                                    Text("\(Display.humanize(event.kind)) · \(event.actor ?? "Unknown actor")")
                                    Text([event.state.map(Display.humanize), Display.age(event.occurredAt)].compactMap { $0 }.joined(separator: " · "))
                                        .font(.caption).foregroundStyle(.secondary)
                                }
                            }
                        }
                    }
                }
                Button("Open full conversation on GitHub") { perform(.openCanonical) }
                Spacer(minLength: 8)
            }
            .padding(28)
        }
        .navigationTitle("Pull request")
    }
}

private struct HealthSummary: View {
    let health: PullRequestCard.Health

    var body: some View {
        HStack(spacing: 7) {
            if let review = health.reviewDecision {
                HealthChip(label: "Review", value: Display.humanize(review), tone: review == "APPROVED" ? .green : .orange)
            } else if health.isDraft {
                HealthChip(label: "Review", value: "Draft", tone: .secondary)
            }
            if let checks = health.checks {
                HealthChip(label: "Checks", value: Display.humanize(checks), tone: ["FAILURE", "ERROR"].contains(checks) ? .red : checks == "SUCCESS" ? .green : .orange)
            }
            HealthChip(label: "Merge", value: Display.humanize(health.mergeable), tone: health.mergeable == "MERGEABLE" ? .green : health.mergeable == "CONFLICTING" ? .orange : .secondary)
        }
        .accessibilityElement(children: .combine)
    }
}

private struct HealthChip: View {
    let label: String
    let value: String
    let tone: Color

    var body: some View {
        Text("\(label): \(value)").font(.caption)
            .padding(.horizontal, 7).padding(.vertical, 4)
            .foregroundStyle(tone)
            .background(tone.opacity(0.12), in: RoundedRectangle(cornerRadius: 6))
    }
}

private struct FrictionLabel: View {
    let friction: PullRequestCard.ReviewFriction

    var body: some View {
        Text("Friction: \(Display.humanize(friction.level ?? friction.status))")
            .font(.caption).foregroundStyle(.secondary)
            .padding(.horizontal, 7).padding(.vertical, 4)
            .background(.quaternary, in: RoundedRectangle(cornerRadius: 6))
    }
}

private extension Text {
    func badgeStyle() -> some View {
        font(.caption).foregroundStyle(.secondary).padding(.horizontal, 7).padding(.vertical, 4)
            .background(.quaternary, in: Capsule())
    }
}

private struct ShortcutHelp: View {
    @Environment(\.dismiss) private var dismiss

    private struct Shortcut: Identifiable {
        let keys: String
        let label: String
        var id: String { keys }
    }

    private struct ShortcutGroup: Identifiable {
        let title: String
        let shortcuts: [Shortcut]
        var id: String { title }
    }

    private let groups = [
        ShortcutGroup(title: "Move around", shortcuts: [Shortcut(keys: "J / K", label: "Next / previous pull request"), Shortcut(keys: "L / H", label: "Open / close details")]),
        ShortcutGroup(title: "Take action", shortcuts: [Shortcut(keys: "O", label: "Open in browser"), Shortcut(keys: "A", label: "Mark as read"), Shortcut(keys: "S", label: "Snooze"), Shortcut(keys: "Y", label: "Copy link")]),
        ShortcutGroup(title: "Workspace", shortcuts: [Shortcut(keys: "/", label: "Focus search"), Shortcut(keys: "Ctrl N", label: "Next workspace"), Shortcut(keys: "Ctrl 1–5", label: "Select workspace"), Shortcut(keys: "Ctrl D / Ctrl U", label: "Page queue"), Shortcut(keys: "Esc", label: "Close details and clear search")])
    ]

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("Keyboard shortcuts").font(.title2.weight(.semibold))
            Text("Single-key shortcuts work outside text fields.").foregroundStyle(.secondary)
            ForEach(groups) { group in
                VStack(alignment: .leading, spacing: 8) {
                    Text(group.title.uppercased()).font(.caption.weight(.semibold)).foregroundStyle(.secondary)
                    ForEach(group.shortcuts) { shortcut in
                        HStack { Text(shortcut.keys).font(.body.monospaced()).frame(width: 104, alignment: .leading); Text(shortcut.label); Spacer() }
                    }
                }
            }
            HStack { Spacer(); Button("Done") { dismiss() }.keyboardShortcut(.defaultAction) }
        }
        .padding(28).frame(minWidth: 440)
    }
}
