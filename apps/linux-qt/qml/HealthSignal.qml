import QtQuick
import QtQuick.Controls

Rectangle {
    id: signal
    required property string label
    required property string value
    required property string tone
    required property string icon
    implicitWidth: content.implicitWidth + 16
    implicitHeight: 27
    radius: 6
    color: tone === "positive" ? Style.positiveTint
         : tone === "negative" ? Style.negativeTint
         : tone === "caution" ? Style.cautionTint : Style.sidebar
    Accessible.name: signal.label + ": " + signal.value
    Accessible.role: Accessible.StaticText

    Row {
        id: content
        anchors.centerIn: parent
        spacing: 5
        Label { text: signal.label + ":"; color: Style.secondary; font.pixelSize: 11 }
        Label {
            text: signal.icon + " " + signal.value
            color: signal.tone === "positive" ? Style.positive
                 : signal.tone === "negative" ? Style.negative
                 : signal.tone === "caution" ? Style.caution : Style.secondary
            font.pixelSize: 11
            font.weight: Font.DemiBold
        }
    }
}
