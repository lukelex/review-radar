import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: card
    required property var entry
    property bool selected: false
    property bool keyboardActive: false
    property bool updating: false
    signal selectedRequested()
    signal openRequested(string url)
    implicitHeight: content.implicitHeight + 38
    color: "white"
    radius: 12
    border.color: selected || keyboardActive || hit.activeFocus ? Style.accent : hit.hovered ? "#c3bedf" : Style.line
    border.width: selected || keyboardActive || hit.activeFocus ? 2 : 1

    Button {
        id: hit
        anchors.fill: parent
        hoverEnabled: true
        background: Item {}
        contentItem: Item {}
        Accessible.name: card.entry.title + ". " + card.entry.explanationHeading + ". Show details"
        onClicked: card.selectedRequested()
    }
    ColumnLayout {
        id: content
        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 20 }
        spacing: 12
        RowLayout {
            Layout.fillWidth: true
            Label {
                Layout.fillWidth: true
                text: card.entry.repository + "   #" + card.entry.number
                textFormat: Text.PlainText
                color: Style.muted; font.pixelSize: 12; elide: Text.ElideMiddle
            }
            Label { text: Style.age(card.entry.updatedAt); color: Style.muted; font.pixelSize: 11 }
            Label { visible: card.updating; text: "Updating…"; color: Style.accent; font.pixelSize: 11 }
        }
        Label {
            Layout.fillWidth: true
            text: card.entry.title || ""; textFormat: Text.PlainText
            color: Style.ink; font.pixelSize: 17; font.weight: Font.DemiBold
            wrapMode: Text.Wrap
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: 11
            Rectangle { Layout.fillHeight: true; implicitWidth: 3; radius: 2; color: card.entry.attentionRequired ? "#aba5f3" : Style.line }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 5
                Label {
                    Layout.fillWidth: true
                    text: card.entry.explanationHeading || ""; textFormat: Text.PlainText
                    color: card.entry.attentionRequired ? Style.accent : Style.secondary
                    font.pixelSize: 12; font.weight: Font.DemiBold; wrapMode: Text.Wrap
                }
                Label {
                    Layout.fillWidth: true
                    text: (card.entry.reasons || []).join("\n"); textFormat: Text.PlainText
                    visible: text.length > 0
                    color: Style.secondary; font.pixelSize: 12; wrapMode: Text.Wrap
                }
            }
        }
        Label {
            Layout.fillWidth: true
            text: card.entry.health || ""; textFormat: Text.PlainText
            visible: text.length > 0
            color: Style.secondary; font.pixelSize: 11; wrapMode: Text.Wrap
        }
        Flow {
            Layout.fillWidth: true
            spacing: 8
            Badge {
                text: Style.humanize(card.entry.lifecycle)
                color: card.entry.lifecycle === "merged" ? Style.accent : Style.secondary
                tint: card.entry.lifecycle === "merged" ? Style.tint : Style.sidebar
            }
            Badge {
                text: "Friction · " + Style.humanize(card.entry.frictionLevel || card.entry.frictionStatus || "not assessed")
                color: card.entry.frictionLevel === "high" ? "#a35b2a" : Style.secondary
                tint: card.entry.frictionLevel === "high" ? "#fff0e4" : Style.sidebar
            }
        }
        RowLayout {
            Layout.fillWidth: true
            Label { Layout.fillWidth: true; text: card.selected ? "Viewing details" : "Select for details & activity"; color: Style.muted; font.pixelSize: 11; elide: Text.ElideRight }
            RadarButton {
                text: (card.entry.nextActionLabel || "Open PR") + "  ↗"
                primary: !!card.entry.attentionRequired
                onClicked: card.openRequested(card.entry.nextActionUrl || card.entry.url)
            }
        }
    }
}
