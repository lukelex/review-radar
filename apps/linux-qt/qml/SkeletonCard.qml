import QtQuick
import QtQuick.Layouts

Rectangle {
    implicitHeight: 170
    color: "white"
    radius: 12
    border.color: Style.line

    ColumnLayout {
        anchors { fill: parent; margins: 20 }
        spacing: 12
        Rectangle { Layout.preferredWidth: 180; Layout.preferredHeight: 12; color: Style.sidebar; radius: 4 }
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 20; color: Style.sidebar; radius: 4 }
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: 12; color: Style.canvas; radius: 4 }
        Rectangle { Layout.preferredWidth: 120; Layout.preferredHeight: 12; color: Style.canvas; radius: 4 }
        Item { Layout.fillHeight: true }
    }
}
