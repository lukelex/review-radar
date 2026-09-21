import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    property string label
    property string description
    property var keys: []
    implicitHeight: 42
    Layout.fillWidth: true

    RowLayout {
        anchors.fill: parent
        spacing: 16

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Label {
                Layout.fillWidth: true
                text: label
                color: Style.ink
                font.pixelSize: 12
                font.weight: Font.DemiBold
            }
            Label {
                Layout.fillWidth: true
                text: description
                color: Style.muted
                font.pixelSize: 11
                elide: Text.ElideRight
            }
        }

        RowLayout {
            spacing: 5
            Repeater {
                model: keys
                delegate: Rectangle {
                    required property string modelData
                    implicitWidth: Math.max(30, keyLabel.implicitWidth + 16)
                    implicitHeight: 28
                    radius: 6
                    color: Style.canvas
                    border.color: Style.line
                    border.width: 1
                    Label {
                        id: keyLabel
                        anchors.centerIn: parent
                        text: modelData
                        color: Style.secondary
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                    }
                }
            }
        }
    }
}
