import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: panel
    objectName: "detail-panel"
    required property var entry
    required property var controller
    signal closeRequested()
    color: "white"; radius: 12; border.color: Style.line
    ColumnLayout {
        anchors.fill: parent; anchors.margins: 20; spacing: 16
        RowLayout {
            Layout.fillWidth: true
            Label { Layout.fillWidth: true; text: "PULL REQUEST DETAILS"; color: Style.muted; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1 }
            RadarButton { objectName: "close-details"; text: "×"; quiet: true; Accessible.name: "Close details"; onClicked: panel.closeRequested() }
        }
        ScrollView {
            id: scroll
            Layout.fillWidth: true; Layout.fillHeight: true
            contentWidth: availableWidth
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ColumnLayout {
                width: scroll.availableWidth; spacing: 20
                Label { Layout.fillWidth: true; text: panel.entry.repository + "   #" + panel.entry.number; textFormat: Text.PlainText; color: Style.muted; wrapMode: Text.Wrap; font.pixelSize: 12 }
                Label { Layout.fillWidth: true; text: panel.entry.title; textFormat: Text.PlainText; color: Style.ink; font.pixelSize: 22; font.weight: Font.DemiBold; wrapMode: Text.Wrap }
                Flow {
                    Layout.fillWidth: true; spacing: 8
                    Badge { text: Style.humanize(panel.entry.lifecycle) }
                    Badge { text: panel.entry.actionLabel; tint: Style.sidebar; color: Style.secondary }
                }
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: why.implicitHeight + 32
                    color: panel.entry.attentionRequired ? Style.tint : Style.canvas; radius: 9
                    ColumnLayout {
                        id: why
                        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 16 }
                        spacing: 10
                        Label { Layout.fillWidth: true; text: panel.entry.attentionRequired ? "WHY THIS NEEDS YOUR ATTENTION" : "WHY THIS IS HERE"; color: Style.accent; font.pixelSize: 10; font.weight: Font.Bold; wrapMode: Text.Wrap }
                        Label { Layout.fillWidth: true; text: panel.entry.explanationHeading; textFormat: Text.PlainText; color: Style.ink; font.pixelSize: 15; font.weight: Font.DemiBold; wrapMode: Text.Wrap }
                        Repeater {
                            model: panel.entry.reasons || []
                            Label { required property string modelData; Layout.fillWidth: true; text: modelData; textFormat: Text.PlainText; color: Style.secondary; font.pixelSize: 12; wrapMode: Text.Wrap }
                        }
                    }
                }
                Flow {
                    Layout.fillWidth: true; spacing: 8
                    RadarButton { objectName: "detail-open"; text: (panel.entry.nextActionLabel || "Open PR") + "  ↗"; primary: true; onClicked: panel.controller.openUrl(panel.entry.nextActionUrl || panel.entry.url) }
                    RadarButton { objectName: "detail-read"; text: "Mark read"; onClicked: panel.controller.acknowledge(panel.entry.pullRequestId, panel.entry.currentFingerprint) }
                    RadarButton {
                        text: "Snooze  ▾"
                        onClicked: snooze.open()
                        Menu {
                            id: snooze
                            MenuItem { text: "Later today"; onTriggered: panel.controller.snooze(panel.entry.pullRequestId, panel.entry.currentFingerprint, "later-today") }
                            MenuItem { text: "Tomorrow"; onTriggered: panel.controller.snooze(panel.entry.pullRequestId, panel.entry.currentFingerprint, "tomorrow") }
                            MenuItem { text: "Next week"; onTriggered: panel.controller.snooze(panel.entry.pullRequestId, panel.entry.currentFingerprint, "next-week") }
                        }
                    }
                    RadarButton {
                        id: copy
                        text: copied ? "Copied ✓" : "Copy link"
                        property bool copied: false
                        onClicked: { panel.controller.copyText(panel.entry.url); copied = true; resetCopy.restart() }
                        Timer { id: resetCopy; interval: 1800; onTriggered: copy.copied = false }
                    }
                }
                Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Style.line }
                Label { text: "PR HEALTH"; color: Style.muted; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1 }
                Label { Layout.fillWidth: true; text: panel.entry.health || "No health information available."; textFormat: Text.PlainText; color: Style.secondary; font.pixelSize: 12; wrapMode: Text.Wrap }
                Label { text: "REVIEW FRICTION"; color: Style.muted; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1 }
                Badge { text: Style.humanize(panel.entry.frictionLevel || panel.entry.frictionStatus || "not assessed"); color: Style.secondary; tint: Style.sidebar }
                Label { Layout.fillWidth: true; text: panel.entry.frictionDetail || "No additional review history available."; textFormat: Text.PlainText; color: Style.secondary; font.pixelSize: 12; wrapMode: Text.Wrap }
                Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Style.line }
                Label { text: "RECENT ACTIVITY"; color: Style.muted; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1 }
                Label { Layout.fillWidth: true; text: "Events from the latest captured history."; color: Style.muted; font.pixelSize: 11; wrapMode: Text.Wrap }
                Repeater {
                    model: panel.entry.events || []
                    RowLayout {
                        required property var modelData
                        Layout.fillWidth: true; spacing: 14
                        Rectangle { Layout.alignment: Qt.AlignTop; Layout.topMargin: 5; implicitWidth: 7; implicitHeight: 7; radius: 4; color: Style.accent }
                        ColumnLayout {
                            Layout.fillWidth: true; spacing: 5
                            Label { Layout.fillWidth: true; text: Style.humanize(modelData.kind) + (modelData.state ? " · " + Style.humanize(modelData.state) : ""); textFormat: Text.PlainText; color: Style.ink; font.pixelSize: 12; font.weight: Font.DemiBold; wrapMode: Text.Wrap }
                            Label { Layout.fillWidth: true; text: (modelData.actor || "GitHub") + " · " + Qt.formatDateTime(new Date(modelData.occurredAt), "MMM d, yyyy · HH:mm"); textFormat: Text.PlainText; color: Style.muted; font.pixelSize: 11; wrapMode: Text.Wrap }
                        }
                    }
                }
                Label { visible: !(panel.entry.events || []).length; text: "No captured activity yet."; color: Style.muted; font.pixelSize: 12 }
                RadarButton { text: "Open full conversation on GitHub  ↗"; quiet: true; onClicked: panel.controller.openUrl(panel.entry.url) }
                Item { implicitHeight: 8 }
            }
        }
    }
}
