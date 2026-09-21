import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ColumnLayout {
    property var entry: ({})
    width: parent ? parent.width : 0
    spacing: 12

    Label { text: "RECENT ACTIVITY"; color: Style.muted; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1 }
    Label { Layout.fillWidth: true; text: "Events from the latest captured history."; color: Style.muted; font.pixelSize: 11; wrapMode: Text.Wrap }
    Repeater {
        model: entry.events || []
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
    Label { visible: !(entry.events || []).length; text: "No captured activity yet."; color: Style.muted; font.pixelSize: 12 }
}
