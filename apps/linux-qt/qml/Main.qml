import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: root
    width: 1440; height: 940
    minimumWidth: 860; minimumHeight: 600
    visible: true
    title: "Review Radar"
    color: Style.canvas
    font.family: "DejaVu Sans"
    font.pixelSize: 13
    palette.text: Style.ink
    palette.windowText: Style.ink
    palette.highlight: Style.accent
    palette.base: "white"
    FocusScope {
        id: workspaceFocus
        anchors.fill: parent
        focus: true
        z: -1
        Shortcut { sequence: "Ctrl+K"; onActivated: { searchField.forceActiveFocus(); searchField.selectAll() } }
        Shortcut { sequence: "Ctrl+R"; onActivated: queue.refresh() }
        Shortcut { sequence: "Ctrl+N"; onActivated: root.nextWorkspace() }
    Shortcut { sequence: "Ctrl+1"; context: Qt.ApplicationShortcut; onActivated: root.selectWorkspace(0) }
    Shortcut { sequence: "Ctrl+2"; context: Qt.ApplicationShortcut; onActivated: root.selectWorkspace(1) }
    Shortcut { sequence: "Ctrl+3"; context: Qt.ApplicationShortcut; onActivated: root.selectWorkspace(2) }
    Shortcut { sequence: "Ctrl+4"; context: Qt.ApplicationShortcut; onActivated: root.selectWorkspace(3) }
    Shortcut { sequence: "Ctrl+5"; context: Qt.ApplicationShortcut; onActivated: root.selectWorkspace(4) }
        Shortcut { sequence: "Escape"; onActivated: root.resetWorkspaceFocus() }
        Shortcut { sequence: "/"; enabled: !searchField.activeFocus; onActivated: { searchField.forceActiveFocus(); searchField.selectAll() } }
        Shortcut { sequence: "Ctrl+D"; enabled: !searchField.activeFocus; onActivated: root.pageList(1) }
        Shortcut { sequence: "Ctrl+U"; enabled: !searchField.activeFocus; onActivated: root.pageList(-1) }
        Shortcut { sequence: "j"; enabled: !searchField.activeFocus; onActivated: root.moveNavigation(1) }
        Shortcut { sequence: "k"; enabled: !searchField.activeFocus; onActivated: root.moveNavigation(-1) }
        Shortcut { sequence: "l"; enabled: !searchField.activeFocus; onActivated: root.openNavigationDetails() }
        Shortcut { sequence: "h"; enabled: !searchField.activeFocus && !!root.selected; onActivated: root.selected = null }
        Shortcut { sequence: "o"; enabled: !searchField.activeFocus && !!root.selected; onActivated: queue.openUrl(root.selected.url) }
        Shortcut { sequence: "a"; enabled: !searchField.activeFocus && !!root.selected; onActivated: acknowledgeDialog.open() }
        Shortcut { sequence: "s"; enabled: !searchField.activeFocus && !!root.selected; onActivated: detailLoader.item.openSnoozeMenu() }
        Shortcut { sequence: "y"; enabled: !searchField.activeFocus && !!root.selected; onActivated: queue.copyText(root.selected.url) }
        Shortcut { sequence: "?"; enabled: !searchField.activeFocus; onActivated: shortcutsDialog.open() }
    }
    property string search: searchField.text.trim().toLowerCase()
    property var selected: null
    property int revision: 0
    property int navigationIndex: -1
    property int matchingCount: { revision; return queue.pullRequests.matchingCount(search) }
    property var views: [
        { key: "tailored", label: "Tailored to you", icon: "◎", shortcut: "1", subtitle: "Your next move, in focus." },
        { key: "action", label: "Action", icon: "⚑", shortcut: "2", subtitle: "The work that needs you." },
        { key: "my-prs", label: "My PRs", icon: "◇", shortcut: "3", subtitle: "Your work, from draft to done." },
        { key: "following", label: "Following", icon: "◉", shortcut: "4", subtitle: "Stay close to the conversation." },
        { key: "recent", label: "Recent", icon: "◷", shortcut: "5", subtitle: "A little perspective on what shipped. The last 14 days." }
    ]
    property var activeView: views.filter(v => v.key === queue.view)[0] || views[0]
    property bool narrow: width < 1250
    property bool showSkeleton: queue.pullRequests.matchingCount("") === 0 && !root.search && (queue.loading || queue.refreshing)
    property bool ctrlHeld: !!queue.controlHeld
    property var shortcutGroups: [
        { title: "Move around", items: [
            { keys: ["J"], label: "Next pull request", description: "Move down the queue" },
            { keys: ["K"], label: "Previous pull request", description: "Move up the queue" },
            { keys: ["L"], label: "Open details", description: "Inspect the selected pull request" },
            { keys: ["H"], label: "Close details", description: "Return to the queue" }
        ]},
        { title: "Take action", items: [
            { keys: ["O"], label: "Open in browser", description: "Open the selected pull request" },
            { keys: ["A"], label: "Mark as read", description: "Quiet this pull request" },
            { keys: ["S"], label: "Snooze", description: "Choose a later reminder" },
            { keys: ["Y"], label: "Copy link", description: "Copy the pull request URL" }
        ]},
        { title: "Workspace", items: [
            { keys: ["/"], label: "Focus search", description: "Search the current workspace" },
            { keys: ["Ctrl", "N"], label: "Next workspace", description: "Move to the next view" },
            { keys: ["Esc"], label: "Close or clear", description: "Close details, then clear search" }
        ]}
    ]

    function matches(index) {
        const entry = queue.pullRequests.get(index)
        return !root.search || entry.title.toLowerCase().includes(root.search)
            || entry.repository.toLowerCase().includes(root.search)
            || String(entry.number).includes(root.search)
    }

    function moveNavigation(direction) {
        const count = queue.pullRequests.matchingCount("")
        if (count === 0) return
        let index = root.navigationIndex
        if (index < 0) index = direction > 0 ? -1 : count
        if (index >= count) index = count
        for (let step = 0; step < count; step++) {
            index += direction
            if (index < 0) index = count - 1
            if (index >= count) index = 0
            if (root.matches(index)) {
                root.navigationIndex = index
                cards.positionViewAtIndex(index, ListView.Contain)
                if (root.selected) root.selected = queue.pullRequests.get(index)
                return
            }
        }
    }

    function openNavigationDetails() {
        if (root.navigationIndex >= 0 && root.matches(root.navigationIndex))
            root.selected = queue.pullRequests.get(root.navigationIndex)
    }

    function selectedEntry() { return root.selected || ({}) }
    function nextWorkspace() {
        const index = root.views.findIndex(view => view.key === queue.view)
        queue.view = root.views[(index + 1) % root.views.length].key
    }
    function selectWorkspace(index) {
        if (index >= 0 && index < root.views.length) queue.view = root.views[index].key
    }

    function switchWorkspace(index) {
        if (index >= 0 && index < root.views.length) queue.view = root.views[index].key
    }

    function pageList(direction) {
        const page = Math.max(1, cards.height * 0.85)
        const maximum = Math.max(0, cards.contentHeight - cards.height)
        cards.contentY = Math.max(0, Math.min(maximum, cards.contentY + direction * page))
    }

    function resetWorkspaceFocus() {
        if (root.selected) root.selected = null
        searchField.clear()
        workspaceFocus.forceActiveFocus()
    }

    RadarModal {
        id: acknowledgeDialog
        objectName: "acknowledge-dialog"
        title: "Mark pull request as read?"
        confirmation: true
        actionText: "Mark read"
        preferredWidth: 460
        property var target: null
        onAboutToShow: target = root.selected
        Label {
            Layout.fillWidth: true
            text: "This PR will stay quiet until a meaningful event changes."
            color: Style.secondary
            wrapMode: Text.Wrap
        }
        onAccepted: if (target) queue.acknowledge(target.pullRequestId, target.currentFingerprint)
    }

    RadarModal {
        id: shortcutsDialog
        objectName: "shortcuts-dialog"
        title: "Keyboard shortcuts"
        subtitle: "Navigate your workspace. Single-key shortcuts work outside search."
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 18
                Layout.margins: 0
                Repeater {
                    model: root.shortcutGroups
                    delegate: ColumnLayout {
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: 8
                        Label { text: modelData.title.toUpperCase(); color: Style.muted; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1 }
                        Repeater {
                            model: modelData.items
                            delegate: ShortcutRow {
                                required property var modelData
                                keys: modelData.keys
                                label: modelData.label
                                description: modelData.description
                            }
                        }
                    }
                }
            }
        }
    }
    Connections {
        target: queue.pullRequests
        function onModelReset() {
            root.revision++;
            if (!root.selected) return;
            const id = root.selected.pullRequestId;
            let updated = null;
            for (let i = 0; i < queue.pullRequests.matchingCount(""); i++) {
                const entry = queue.pullRequests.get(i);
                if (entry.pullRequestId === id) { updated = entry; break; }
            }
            root.selected = updated;
        }
    }
    Connections { target: queue; function onViewChanged() { root.selected = null; cards.positionViewAtBeginning() } }

    RowLayout {
        anchors.fill: parent; spacing: 0
        Rectangle {
            Layout.preferredWidth: root.width < 1050 ? 194 : 238
            Layout.fillHeight: true
            color: Style.sidebar
            ColumnLayout {
                anchors.fill: parent; anchors.margins: 20; spacing: 8
                RowLayout {
                    Layout.topMargin: 6; Layout.bottomMargin: 38; spacing: 12
                    Image {
                        Layout.preferredWidth: 34; Layout.preferredHeight: 34
                        source: "qrc:/assets/logo.svg"
                        sourceSize: Qt.size(68, 68)
                        fillMode: Image.PreserveAspectFit
                        Accessible.ignored: true
                    }
                    Label { text: "Review Radar"; color: Style.ink; font.pixelSize: 16; font.weight: Font.Bold }
                }
                Label { text: "WORKSPACE"; color: Style.muted; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1; Layout.bottomMargin: 10 }
                Repeater {
                    model: root.views
                    Button {
                        id: nav
                        objectName: "view-" + modelData.key
                        required property var modelData
                        Layout.fillWidth: true; implicitHeight: 43
                        hoverEnabled: true
                         property bool current: queue.view === modelData.key
                         Accessible.name: modelData.label
                         Accessible.role: Accessible.PageTab
                         onClicked: queue.view = modelData.key
                         background: Rectangle { radius: 8; color: nav.current ? "#e5e3fa" : nav.hovered ? "#e9ecf3" : "transparent"; border.color: nav.activeFocus ? Style.accent : "transparent" }
                         Label {
                             anchors.right: parent.left
                             anchors.rightMargin: 8
                             anchors.verticalCenter: parent.verticalCenter
                             text: nav.modelData.shortcut
                             color: nav.current ? Style.accent : Style.muted
                             font.pixelSize: 11
                             font.weight: Font.DemiBold
                             visible: root.ctrlHeld
                         }
                         contentItem: RowLayout {
                             spacing: 12
                             Label { text: nav.modelData.icon; color: nav.current ? Style.accent : Style.muted; font.pixelSize: 18; Layout.leftMargin: 8 }
                            Label { Layout.fillWidth: true; text: nav.modelData.label; color: nav.current ? Style.accent : Style.secondary; font.pixelSize: 12; font.weight: nav.current ? Font.DemiBold : Font.Normal }
                        }
                    }
                }
                Rectangle { Layout.fillWidth: true; Layout.topMargin: 26; Layout.bottomMargin: 12; implicitHeight: 1; color: Style.line }
                Label { text: "LOCAL TO THIS DEVICE"; color: Style.muted; font.pixelSize: 9; font.weight: Font.Bold; font.letterSpacing: 0.5 }
                Label { Layout.fillWidth: true; text: "Read and snoozed items stay quiet until something meaningful changes."; color: Style.muted; font.pixelSize: 11; wrapMode: Text.Wrap; lineHeight: 1.3 }
                Label { visible: queue.suppressedCount > 0; text: queue.suppressedCount + " hidden in this view"; color: Style.secondary; font.pixelSize: 11 }
                Item { Layout.fillHeight: true }
                Label { text: "●  github.com"; color: Style.secondary; font.pixelSize: 12; font.weight: Font.DemiBold }
                Label { Layout.fillWidth: true; text: "A quieter place for pull requests."; color: Style.muted; font.pixelSize: 10; wrapMode: Text.Wrap; Layout.bottomMargin: 4 }
            }
        }
        ColumnLayout {
            Layout.fillWidth: true; Layout.fillHeight: true; spacing: 0
            Rectangle {
                Layout.fillWidth: true; implicitHeight: 84; color: "white"
                RowLayout {
                    anchors.fill: parent; anchors.margins: 24; spacing: 18
                    TextField {
                        id: searchField
                        objectName: "search"
                        Layout.fillWidth: true; Layout.maximumWidth: 520
                        implicitHeight: 38; leftPadding: 14; rightPadding: 62
                        placeholderText: "Search title, repository, or PR number"
                        font.pixelSize: 12; color: Style.ink; placeholderTextColor: Style.muted
                        Accessible.name: "Search pull requests"
                        selectByMouse: true
                        background: Rectangle { color: Style.canvas; radius: 8; border.color: searchField.activeFocus ? Style.accent : Style.line }
                        Label { anchors.right: parent.right; anchors.rightMargin: 12; anchors.verticalCenter: parent.verticalCenter; text: "Ctrl K"; color: Style.muted; font.pixelSize: 10 }
                    }
                    Item { Layout.fillWidth: true }
                    Label {
                        visible: root.width >= 1100
                         text: queue.refreshing && queue.stale ? "●  Cached data · syncing" : queue.stale ? "●  Cached data" : queue.loading ? "●  Loading" : "●  Local workspace"
                         color: queue.stale ? "#a35b2a" : queue.refreshing ? Style.accent : Style.secondary
                        font.pixelSize: 12
                    }
                    RadarButton { text: queue.refreshing ? "Syncing…" : "↻  Refresh"; enabled: !queue.refreshing && !queue.loading; onClicked: queue.refresh(); ToolTip.visible: hovered; ToolTip.text: "Refresh from GitHub · Ctrl+R" }
                }
            }
            ColumnLayout {
                Layout.fillWidth: true; Layout.fillHeight: true
                Layout.margins: root.width < 1050 ? 20 : 32
                spacing: 18
                Label { text: "WORKSPACE  /  " + root.activeView.label.toUpperCase(); color: Style.muted; font.pixelSize: 10; font.letterSpacing: 1 }
                RowLayout {
                    Layout.fillWidth: true
                    ColumnLayout {
                        Layout.fillWidth: true; spacing: 8
                        Label { text: root.activeView.label; color: Style.ink; font.pixelSize: 30; font.weight: Font.Bold }
                        Label { Layout.fillWidth: true; text: root.activeView.subtitle; color: Style.muted; font.pixelSize: 13; wrapMode: Text.Wrap }
                    }
                    Badge { text: root.matchingCount + (root.matchingCount === 1 ? " pull request" : " pull requests"); color: Style.secondary; tint: "#e9ecf3" }
                }
                RowLayout {
                    Layout.fillWidth: true; Layout.topMargin: 6
                    Label { Layout.fillWidth: true; text: root.search ? "SEARCH RESULTS" : "PULL REQUESTS"; color: Style.muted; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1 }
                    ComboBox {
                        id: sort
                        objectName: "sort"
                        implicitWidth: 205; implicitHeight: 36
                        textRole: "label"; valueRole: "key"
                        model: [{key: "tailored", label: "Priority + newest"}, {key: "newest-activity", label: "Newest activity"}, {key: "highest-friction", label: "Highest friction"}]
                        currentIndex: model.findIndex(item => item.key === queue.ranking)
                        onActivated: queue.ranking = currentValue
                        font.pixelSize: 12
                        Accessible.name: "Sort pull requests"
                        background: Rectangle { color: "white"; radius: 7; border.color: sort.activeFocus ? Style.accent : Style.line }
                    }
                }
                RowLayout {
                    Layout.fillWidth: true; Layout.fillHeight: true; spacing: 20
                    Item {
                        Layout.fillWidth: true; Layout.fillHeight: true
                        visible: !root.selected || !root.narrow
                         ListView {
                             id: cards
                             objectName: "pull-request-list"
                            anchors.fill: parent; clip: true
                            model: queue.pullRequests
                            boundsBehavior: Flickable.StopAtBounds
                            ScrollBar.vertical: ScrollBar {}
                            delegate: Item {
                                id: row
                                required property int index
                                required property string title
                                required property string repository
                                required property int number
                                property bool matches: !root.search || title.toLowerCase().includes(root.search) || repository.toLowerCase().includes(root.search) || String(number).includes(root.search)
                                property var entry: { root.revision; return queue.pullRequests.get(index) }
                                width: cards.width - 12
                                height: matches ? pr.implicitHeight + 16 : 0
                                visible: matches
                                PrCard {
                                    id: pr
                                    objectName: "card-" + row.index
                                    width: parent.width
                                     entry: row.entry
                                     updating: queue.refreshing && queue.stale
                                     selected: !!root.selected && root.selected.pullRequestId === entry.pullRequestId
                                     keyboardActive: root.navigationIndex === row.index
                                     onSelectedRequested: { root.navigationIndex = row.index; root.selected = entry }
                                    onOpenRequested: url => queue.openUrl(url)
                                }
                            }
                         }
                         Column {
                             anchors { left: parent.left; right: parent.right; top: parent.top }
                             spacing: 16
                             visible: root.showSkeleton
                             Repeater { model: 4; SkeletonCard { width: parent.width } }
                         }
                         ColumnLayout {
                            anchors.centerIn: parent; width: Math.min(parent.width - 40, 360); spacing: 14
                             visible: root.matchingCount === 0 && !root.showSkeleton
                            Label { Layout.alignment: Qt.AlignHCenter; text: queue.loading || queue.refreshing ? "◎" : root.search ? "⌕" : "✓"; font.pixelSize: 42; color: Style.accent }
                            Label { Layout.fillWidth: true; text: root.search ? "No matching pull requests" : queue.loading || queue.refreshing ? "Preparing your workspace" : "Nothing here right now"; font.pixelSize: 19; font.weight: Font.DemiBold; color: Style.ink; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.Wrap }
                            Label { Layout.fillWidth: true; text: root.search ? "Try a different title, repository, or PR number." : queue.loading || queue.refreshing ? "Your pull requests will appear as soon as the data is ready." : "Refresh to check GitHub, or explore another workspace."; color: Style.muted; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.Wrap; font.pixelSize: 12 }
                            RadarButton { Layout.alignment: Qt.AlignHCenter; visible: root.search.length > 0; text: "Clear search"; onClicked: searchField.clear() }
                        }
                    }
                        Loader {
                            id: detailLoader
                            Layout.fillHeight: true
                        Layout.fillWidth: root.narrow
                        Layout.preferredWidth: root.narrow ? -1 : Math.min(520, root.width * 0.39)
                        active: !!root.selected; visible: active
                        objectName: "detail-loader"
                        sourceComponent: DetailPanel { entry: root.selected || ({}); controller: queue; onCloseRequested: root.selected = null }
                    }
                }
                RowLayout {
                    Layout.fillWidth: true; spacing: 8
                    Rectangle { implicitWidth: 6; implicitHeight: 6; radius: 3; color: queue.stale || queue.status.startsWith("Could not") ? "#a35b2a" : Style.accent }
                    Label { Layout.fillWidth: true; text: queue.status; textFormat: Text.PlainText; color: queue.stale || queue.status.startsWith("Could not") ? "#a35b2a" : Style.muted; font.pixelSize: 11; elide: Text.ElideRight; ToolTip.visible: statusHover.hovered; ToolTip.text: text; HoverHandler { id: statusHover } }
                    Label { visible: root.width >= 1100; text: "LOCAL FIRST  ·  SYNC EVERY 5 MIN"; color: Style.muted; font.pixelSize: 9; font.letterSpacing: 0.5 }
                }
            }
        }
    }
}
