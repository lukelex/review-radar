import QtQuick
import QtQuick.Controls

Button {
    id: control
    property bool primary: false
    property bool quiet: false
    hoverEnabled: true
    implicitHeight: 36
    implicitWidth: Math.max(36, contentItem.implicitWidth + 26)
    padding: 12
    verticalPadding: 7
    font.pixelSize: 12
    font.weight: Font.DemiBold
    Accessible.name: text
    contentItem: Text {
        text: control.text
        font: control.font
        textFormat: Text.PlainText
        color: !control.enabled ? Style.muted : control.primary ? "white" : Style.ink
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }
    background: Rectangle {
        radius: 7
        color: control.primary ? (control.down ? "#4840bb" : control.hovered ? "#554dce" : Style.accent)
                               : control.down ? "#e5e3fa" : control.hovered ? Style.tint
                               : control.quiet ? "transparent" : "white"
        border.color: control.activeFocus ? Style.accent : control.primary || control.quiet ? "transparent" : Style.line
        border.width: control.activeFocus ? 2 : 1
        opacity: control.enabled ? 1 : 0.6
        Behavior on color { ColorAnimation { duration: 100 } }
    }
}
