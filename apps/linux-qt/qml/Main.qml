import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: root
    width: 1180
    height: 760
    minimumWidth: 860
    minimumHeight: 560
    visible: true
    title: "Review Radar"

    property string search: ""
    property var views: [
        { id: "tailored", label: "Tailored to you" },
        { id: "action", label: "Action" },
        { id: "my-prs", label: "My PRs" },
        { id: "following", label: "Following" },
        { id: "recent", label: "Recent" }
    ]

    header: ToolBar {
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 18
            anchors.rightMargin: 18
            spacing: 12
            Label { text: "Review Radar"; font.bold: true; font.pixelSize: 19 }
            TextField {
                Layout.fillWidth: true
                placeholderText: "Search title, repository, or PR number"
                onTextChanged: root.search = text.toLowerCase()
            }
            ComboBox {
                model: ["tailored", "newest-activity", "highest-friction"]
                currentIndex: model.indexOf(queue.ranking)
                onActivated: queue.ranking = currentText
            }
            Button {
                text: queue.loading ? "Refreshing…" : "Refresh"
                enabled: !queue.loading
                onClicked: queue.refresh()
            }
        }
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0
        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: 220
            color: "#f5f5f5"
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 6
                Label { text: "WORKSPACE"; font.bold: true; color: "#666" }
                Repeater {
                    model: root.views
                    delegate: Button {
                        required property var modelData
                        Layout.fillWidth: true
                        text: modelData.label
                        highlighted: queue.view === modelData.id
                        onClicked: queue.view = modelData.id
                    }
                }
                Item { Layout.fillHeight: true }
                Label { text: queue.sourceCount + " in this view"; color: "#666" }
                Label {
                    visible: queue.suppressedCount > 0
                    text: queue.suppressedCount + " locally hidden"
                    color: "#666"
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0
            Label {
                Layout.fillWidth: true
                Layout.margins: 16
                text: queue.status
                color: queue.status.startsWith("Could not") ? "#9c2a2a" : "#666"
                wrapMode: Text.Wrap
            }
            ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                ListView {
                    id: cards
                    anchors.fill: parent
                    anchors.margins: 16
                    model: queue.pullRequests
                    spacing: 10
                    delegate: Rectangle {
                        id: card
                        required property string repository
                        required property int number
                        required property string title
                        required property string url
                        required property string actionLabel
                        required property bool attentionRequired
                        required property string currentFingerprint
                        required property string explanationHeading
                        required property var reasons
                        required property string health
                        required property string nextActionLabel
                        required property string nextActionUrl
                        required property string frictionStatus
                        required property string frictionLevel
                        required property string frictionDetail
                        required property string pullRequestId
                        required property var events
                        property bool expanded: false
                        property bool matchesSearch: root.search.length === 0
                            || title.toLowerCase().includes(root.search)
                            || repository.toLowerCase().includes(root.search)
                            || String(number).includes(root.search)
                        width: cards.width
                        height: matchesSearch ? content.implicitHeight + 24 : 0
                        visible: matchesSearch
                        color: attentionRequired ? "#fff9ee" : "white"
                        border.color: attentionRequired ? "#d98b2b" : "#d4d4d4"
                        radius: 6
                        ColumnLayout {
                            id: content
                            anchors.fill: parent
                            anchors.margins: 12
                            spacing: 7
                            RowLayout {
                                Layout.fillWidth: true
                                Label { text: repository + " #" + number; font.bold: true }
                                Item { Layout.fillWidth: true }
                                Label { text: actionLabel; color: attentionRequired ? "#9a5900" : "#666" }
                            }
                            Label { Layout.fillWidth: true; text: title; font.pixelSize: 16; wrapMode: Text.Wrap }
                            Label { Layout.fillWidth: true; text: explanationHeading; font.bold: true; color: "#555" }
                            Repeater {
                                model: reasons
                                delegate: Label {
                                    required property string modelData
                                    Layout.fillWidth: true
                                    text: modelData
                                    wrapMode: Text.Wrap
                                    color: "#333"
                                }
                            }
                            Label { visible: health.length > 0; Layout.fillWidth: true; text: health; wrapMode: Text.Wrap; color: "#666" }
                            Label {
                                visible: frictionStatus.length > 0
                                text: "Review friction: " + (frictionLevel.length > 0 ? frictionLevel : frictionStatus)
                                      + (frictionDetail.length > 0 ? " · " + frictionDetail : "")
                                color: "#666"
                            }
                            RowLayout {
                                Button { text: nextActionLabel || "Open PR"; onClicked: queue.openUrl(nextActionUrl || url) }
                                Button { text: "Mark read"; onClicked: queue.acknowledge(pullRequestId, currentFingerprint) }
                                Button { text: "Snooze"; onClicked: snoozeMenu.open() }
                                Menu {
                                    id: snoozeMenu
                                    MenuItem { text: "Later today"; onTriggered: queue.snooze(pullRequestId, currentFingerprint, "later-today") }
                                    MenuItem { text: "Tomorrow"; onTriggered: queue.snooze(pullRequestId, currentFingerprint, "tomorrow") }
                                    MenuItem { text: "Next week"; onTriggered: queue.snooze(pullRequestId, currentFingerprint, "next-week") }
                                }
                                Button { text: "Copy link"; onClicked: queue.copyText(url) }
                                Item { Layout.fillWidth: true }
                                Button { text: expanded ? "Hide activity" : "Activity"; onClicked: expanded = !expanded }
                            }
                            Repeater {
                                model: expanded ? events : []
                                delegate: Label {
                                    required property var modelData
                                    Layout.fillWidth: true
                                    leftPadding: 14
                                    text: modelData.occurredAt + "  " + modelData.kind + "  " + (modelData.state || "")
                                    color: "#666"
                                    wrapMode: Text.Wrap
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
