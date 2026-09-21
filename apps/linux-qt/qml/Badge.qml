import QtQuick
import QtQuick.Controls

Label {
    property color tint: Style.tint
    padding: 8
    topPadding: 5
    bottomPadding: 5
    color: Style.accent
    font.pixelSize: 11
    font.weight: Font.DemiBold
    textFormat: Text.PlainText
    background: Rectangle { color: parent.tint; radius: 6 }
}
