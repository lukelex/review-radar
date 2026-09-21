import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: modal
    default property alias body: bodyLayout.data
    property string subtitle: ""
    property string actionText: "Done"
    property bool confirmation: false
    property real preferredWidth: 540
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(preferredWidth, parent.width - 40)
    height: Math.min(implicitHeight, parent.height - 40)
    modal: true
    focus: true
    padding: 24
    spacing: 20
    closePolicy: Popup.CloseOnEscape
    background: Rectangle { color: "white"; radius: 12; border.color: Style.line }
    Overlay.modal: Rectangle { color: "#6620283f" }

    header: ColumnLayout {
        spacing: 8
        Label {
            Layout.fillWidth: true
            Layout.leftMargin: 24; Layout.rightMargin: 24; Layout.topMargin: 24
            text: modal.title; textFormat: Text.PlainText
            color: Style.ink; font.pixelSize: 22; font.weight: Font.DemiBold
            wrapMode: Text.Wrap
        }
        Label {
            Layout.fillWidth: true
            Layout.leftMargin: 24; Layout.rightMargin: 24
            visible: text.length > 0
            text: modal.subtitle; textFormat: Text.PlainText
            color: Style.secondary; font.pixelSize: 12; wrapMode: Text.Wrap
        }
    }
    contentItem: ScrollView {
        id: scroll
        implicitHeight: bodyLayout.implicitHeight
        contentWidth: availableWidth
        clip: true
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        ColumnLayout { id: bodyLayout; width: scroll.availableWidth; spacing: 20 }
    }
    footer: ColumnLayout {
        spacing: 0
        Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Style.line }
        RowLayout {
            Layout.fillWidth: true; Layout.margins: 20; spacing: 8
            Label { Layout.fillWidth: true; text: "Esc to close"; color: Style.muted; font.pixelSize: 11 }
            RadarButton { id: cancel; visible: modal.confirmation; text: "Cancel"; onClicked: modal.reject() }
            RadarButton { id: action; text: modal.actionText; primary: true; onClicked: modal.accept() }
        }
    }
    onOpened: {
        if (confirmation) cancel.forceActiveFocus()
        else action.forceActiveFocus()
    }
}
