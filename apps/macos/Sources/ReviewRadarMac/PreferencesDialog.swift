import SwiftUI

struct PreferencesDialog: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject private var preferences: PreferencesStore
    let testNotification: () async -> Bool

    @State private var draft = Preferences()
    @State private var section = "Notifications"
    @State private var error: String?
    @State private var testResult: String?
    @State private var confirmDiscard = false

    private let sections = ["General", "Workspace", "Notifications", "Desktop integration", "Appearance", "Keyboard", "Account & sync", "Local data", "Advanced"]

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("REVIEW RADAR").font(.caption.weight(.bold)).foregroundStyle(.indigo)
                    Text("Preferences").font(.title2.weight(.semibold))
                    Text("Make Review Radar work your way.").font(.subheadline).foregroundStyle(.secondary)
                }
                Spacer()
                Button("Close") { requestClose() }
            }
            .padding(24)
            Divider()
            HStack(spacing: 0) {
                List(sections, id: \.self, selection: $section) { Text($0) }
                    .frame(width: 200)
                Divider()
                ScrollView {
                    VStack(alignment: .leading, spacing: 16) {
                        Text(section).font(.title3.weight(.semibold))
                        Text(description).foregroundStyle(.secondary)
                        if section == "Notifications" { notificationSettings }
                        else if section == "Desktop integration" { desktopSettings }
                        else { plannedSettings }
                        if let error { Text(error).foregroundStyle(.red).accessibilityLabel(error) }
                    }
                    .padding(24).frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            Divider()
            HStack {
                Text(draft == preferences.value ? "Changes apply to this device." : "Unsaved changes")
                    .font(.caption).foregroundStyle(.secondary)
                Spacer()
                Button("Cancel") { requestClose() }
                Button("Save changes") { save() }.buttonStyle(.borderedProminent).disabled(draft == preferences.value)
            }
            .padding(18)
        }
        .frame(minWidth: 780, minHeight: 560)
        .onAppear { draft = preferences.value }
        .confirmationDialog("Discard preference changes?", isPresented: $confirmDiscard, titleVisibility: .visible) {
            Button("Discard changes", role: .destructive) { dismiss() }
            Button("Keep editing", role: .cancel) {}
        } message: {
            Text("Your saved preferences will stay as they are.")
        }
    }

    private var description: String {
        switch section {
        case "Notifications": "Useful updates, with room to focus."
        case "Desktop integration": "Choose where Review Radar appears on your desktop."
        default: "This setting is planned and does not change Review Radar yet."
        }
    }

    private var notificationSettings: some View {
        VStack(alignment: .leading, spacing: 14) {
            GroupBox("Desktop alerts") {
                VStack(alignment: .leading, spacing: 12) {
                    Toggle("Show desktop notifications", isOn: $draft.notificationsEnabled)
                    Text(draft.notificationsEnabled
                         ? "Get an alert when a pull request newly needs your attention. Existing activity establishes a quiet baseline."
                         : "Desktop alerts are off. Your workspace keeps updating and observed transitions will not replay when alerts return.")
                        .font(.caption).foregroundStyle(.secondary)
                    Button("Test notification") {
                        Task {
                            testResult = await testNotification()
                                ? "Test sent to Notification Center. System focus settings may still hide it."
                                : "Could not send the test notification. Check Notification Center permissions."
                        }
                    }
                    if let testResult { Text(testResult).font(.caption).foregroundStyle(.secondary) }
                }.frame(maxWidth: .infinity, alignment: .leading)
            }
            GroupBox("What to notify about") {
                VStack(alignment: .leading) {
                    Toggle("Review requests", isOn: $draft.notifyReviewRequests)
                    Toggle("Feedback requiring your response", isOn: $draft.notifyFeedback)
                    Toggle("Failed checks on your PRs", isOn: $draft.notifyChecks)
                    Toggle("Merge conflicts", isOn: $draft.notifyConflicts)
                }.disabled(!draft.notificationsEnabled)
            }
            GroupBox("Quiet hours") {
                VStack(alignment: .leading, spacing: 10) {
                    Toggle("Pause desktop alerts during a local-time window", isOn: $draft.quietHours)
                    Text("Attention tracking continues while alerts are paused.").font(.caption).foregroundStyle(.secondary)
                    HStack {
                        TextField("18:00", text: $draft.quietHoursStart).frame(width: 72)
                        Text("to")
                        TextField("09:00", text: $draft.quietHoursEnd).frame(width: 72)
                        Text("Local time · HH:mm").font(.caption).foregroundStyle(.secondary)
                    }.disabled(!draft.quietHours || !draft.notificationsEnabled)
                }.frame(maxWidth: .infinity, alignment: .leading)
            }
            Text("Category filters and quiet hours affect notification delivery only; classification and deduplication continue for every eligible update.")
                .font(.caption).foregroundStyle(.secondary)
        }
    }

    private var desktopSettings: some View {
        GroupBox("Menu bar") {
            VStack(alignment: .leading, spacing: 10) {
                Toggle("Show menu-bar item", isOn: $draft.menuBarEnabled)
                Toggle("Keep running when the last window closes", isOn: $draft.keepRunningWhenWindowCloses)
                    .disabled(!draft.menuBarEnabled)
                Text("The menu-bar item offers Open Review Radar, Refresh, Preferences, and Quit. If it is off, closing the last window exits normally.")
                    .font(.caption).foregroundStyle(.secondary)
            }.frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private var plannedSettings: some View {
        GroupBox("Planned") {
            Text("This option is a design preview only. It cannot change local state, GitHub settings, ranking, or notification eligibility.")
                .foregroundStyle(.secondary)
        }
    }

    private func requestClose() {
        if draft == preferences.value { dismiss() } else { confirmDiscard = true }
    }

    private func save() {
        do {
            try preferences.save(draft)
            dismiss()
        } catch {
            self.error = error.localizedDescription
        }
    }
}
