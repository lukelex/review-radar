import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: preferences
    required property var controller
    property bool draftNotifications: true
    property bool draftTray: false
    property bool draftCloseToTray: false
    property bool draftAttentionDot: true
    property bool draftBar: false
    property string errorMessage: ""
    property string testMessage: ""
    property bool testSucceeded: false
    property string section: "Notifications"
    readonly property bool dirty: draftNotifications !== controller.notificationsEnabled
        || draftTray !== controller.trayEnabled || draftCloseToTray !== controller.closeToTray
        || draftAttentionDot !== controller.trayAttentionDot
        || draftBar !== controller.barEnabled
    readonly property bool compact: width < 900
    readonly property var sections: ["General", "Workspace", "Notifications", "Desktop integration", "Appearance", "Keyboard", "Account & sync", "Local data", "Advanced"]
    readonly property var descriptions: ({
        "General": "A familiar starting point, every time you open Review Radar.",
        "Workspace": "Make the next action easy to find.",
        "Notifications": "Useful updates, with room to focus.",
        "Desktop integration": "Choose where Review Radar appears on your desktop.",
        "Appearance": "A calm workspace that feels like yours.",
        "Keyboard": "Keep your hands on the keyboard.",
        "Account & sync": "Stay connected without losing your place.",
        "Local data": "Your workspace history, stored on this device.",
        "Advanced": "Tools for understanding how Review Radar is running."
    })
    readonly property var planned: ({
        "General": [{ title: "WHEN YOU OPEN THE APP", rows: ["Startup workspace", "Restore previous selection"] }, { title: "STARTUP", rows: ["Launch at login", "Start minimized"] }],
        "Workspace": [{ title: "ORDERING", rows: ["Default ranking"] }, { title: "CARDS", rows: ["Card density", "Show health chips", "Show review friction", "Activity preview"] }],
        "Appearance": [{ title: "THEME", rows: ["Color theme"] }, { title: "READABILITY", rows: ["Text size", "Increase contrast", "Reduce motion"] }],
        "Keyboard": [{ title: "NAVIGATION PREFERENCES", rows: ["Vim-style navigation", "Workspace shortcut hints"] }],
        "Account & sync": [{ title: "REFRESH PREFERENCES", rows: ["Refresh on open", "Refresh interval"] }],
        "Local data": [{ title: "STORAGE MANAGEMENT", rows: ["Capture history", "Device state", "Reset local preferences"] }],
        "Advanced": [{ title: "DIAGNOSTICS", rows: ["Diagnostic logging", "Diagnostic summary", "Platform capabilities"] }]
    })
    objectName: "preferences-dialog"
    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(1000, parent.width - 40)
    height: Math.min(780, parent.height - 40)
    padding: 0
    spacing: 0
    modal: true
    focus: true
    closePolicy: Popup.NoAutoClose
    background: Rectangle { color: "white"; radius: 16; border.color: Style.line }
    Overlay.modal: Rectangle { color: "#6620283f" }
    onAboutToShow: {
        draftNotifications = controller.notificationsEnabled
        draftTray = controller.trayEnabled
        draftCloseToTray = controller.closeToTray
        draftAttentionDot = controller.trayAttentionDot
        draftBar = controller.barEnabled
        errorMessage = ""; testMessage = ""; section = "Notifications"
    }
    onOpened: toggle.forceActiveFocus()
    function requestClose() { if (dirty) discard.open(); else close() }
    Shortcut { sequence: "Escape"; enabled: preferences.opened && !discard.opened; onActivated: preferences.requestClose() }

    component SettingSwitch: Rectangle {
        id: row
        required property string label
        required property string description
        property bool checked: false
        signal changed(bool value)
        Layout.fillWidth: true
        implicitHeight: settingRow.implicitHeight + 36
        color: "white"; radius: 11; border.color: Style.line
        RowLayout {
            id: settingRow
            anchors.fill: parent; anchors.margins: 18; spacing: 18
            ColumnLayout {
                Layout.fillWidth: true; spacing: 6
                Label { Layout.fillWidth: true; text: row.label; color: Style.ink; font.pixelSize: 13; font.weight: Font.DemiBold; wrapMode: Text.Wrap }
                Label { Layout.fillWidth: true; text: row.description; color: Style.secondary; font.pixelSize: 11; wrapMode: Text.Wrap }
            }
            Switch {
                id: settingToggle
                checked: row.checked
                onToggled: row.changed(checked)
                Accessible.name: row.label
                padding: 3
                indicator: Rectangle {
                    implicitWidth: 36; implicitHeight: 21; x: 3; y: 3; radius: 11
                    color: !settingToggle.enabled ? "#d3d7e1" : settingToggle.checked ? Style.accent : "#b4bbca"
                    border.width: settingToggle.activeFocus ? 2 : 0; border.color: Style.ink
                    Rectangle { x: settingToggle.checked ? 18 : 3; y: 3; width: 15; height: 15; radius: 8; color: "white" }
                }
            }
        }
    }

    header: Rectangle {
        implicitHeight: 112
        color: "transparent"
        Column {
            anchors.left: parent.left; anchors.leftMargin: 30
            anchors.top: parent.top; anchors.topMargin: 22; spacing: 6
            Label { text: "REVIEW RADAR"; color: Style.accent; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1.5 }
            Label { text: "Preferences"; color: Style.ink; font.pixelSize: 25; font.weight: Font.DemiBold }
            Label { text: "Make Review Radar work your way."; color: Style.secondary; font.pixelSize: 12 }
        }
        RadarButton { anchors.right: parent.right; anchors.rightMargin: 20; anchors.top: parent.top; anchors.topMargin: 20; text: "×"; quiet: true; Accessible.name: "Close preferences"; onClicked: preferences.requestClose() }
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Style.line }
    }

    contentItem: Item {
        Rectangle {
            id: rail
            width: preferences.compact ? parent.width : 218
            height: preferences.compact ? 54 : parent.height
            color: "#f8f9fc"
            Label { visible: !preferences.compact; x: 26; y: 25; text: "ON THIS DEVICE"; color: Style.secondary; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1.1 }
            ListView {
                id: navigation
                objectName: "preferences-navigation"
                anchors.fill: parent
                anchors.topMargin: preferences.compact ? 6 : 52
                anchors.bottomMargin: preferences.compact ? 6 : 85
                anchors.leftMargin: 14; anchors.rightMargin: 14
                clip: true; spacing: 3
                orientation: preferences.compact ? ListView.Horizontal : ListView.Vertical
                model: preferences.sections
                delegate: Button {
                    required property string modelData
                    width: preferences.compact ? implicitWidth + 16 : navigation.width
                    height: 40
                    text: modelData
                    Accessible.name: modelData
                    highlighted: preferences.section === modelData
                    onClicked: { preferences.section = modelData; scroll.contentItem.contentY = 0 }
                    background: Rectangle { radius: 8; color: parent.highlighted ? "#e5e3fa" : parent.hovered ? "#eef0f6" : "transparent"; border.width: parent.activeFocus ? 2 : 0; border.color: Style.accent }
                    contentItem: Label { text: parent.text; leftPadding: 12; verticalAlignment: Text.AlignVCenter; color: parent.highlighted ? "#4940b2" : Style.secondary; font.pixelSize: 12; font.weight: parent.highlighted ? Font.DemiBold : Font.Normal }
                }
            }
            Column {
                visible: !preferences.compact
                anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom; anchors.margins: 24; spacing: 12
                Rectangle { width: parent.width; height: 1; color: Style.line }
                Label { width: parent.width; text: "Your preferences are local.\nGitHub settings are unaffected."; color: Style.secondary; font.pixelSize: 11; lineHeight: 1.4 }
            }
        }
        ScrollView {
            id: scroll
            anchors.left: preferences.compact ? parent.left : rail.right
            anchors.top: preferences.compact ? rail.bottom : parent.top
            anchors.right: parent.right; anchors.bottom: parent.bottom
            clip: true
            contentWidth: availableWidth
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ColumnLayout {
                width: scroll.availableWidth
                spacing: 12
                Item { Layout.preferredHeight: 14 }
                Label { Layout.leftMargin: 30; Layout.rightMargin: 30; Layout.fillWidth: true; text: preferences.section; color: Style.ink; font.pixelSize: 21; font.weight: Font.DemiBold; wrapMode: Text.Wrap }
                Label { Layout.leftMargin: 30; Layout.rightMargin: 30; Layout.fillWidth: true; text: preferences.descriptions[preferences.section]; color: Style.secondary; font.pixelSize: 12; wrapMode: Text.Wrap; Layout.bottomMargin: 8 }

                ColumnLayout {
                    visible: preferences.section === "Notifications"
                    Layout.fillWidth: true; Layout.leftMargin: 30; Layout.rightMargin: 30; spacing: 12
                    Label { text: "DESKTOP ALERTS"; color: Style.secondary; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1.2 }
                    Rectangle {
                        Layout.fillWidth: true; implicitHeight: alertRow.implicitHeight + 36
                        radius: 11; color: "white"; border.color: Style.line
                        RowLayout {
                            id: alertRow
                            anchors.fill: parent; anchors.margins: 18; spacing: 18
                            ColumnLayout {
                                Layout.fillWidth: true; spacing: 6
                                Label { Layout.fillWidth: true; text: "Show desktop notifications"; color: Style.ink; font.pixelSize: 13; font.weight: Font.DemiBold; wrapMode: Text.Wrap }
                                Label { Layout.fillWidth: true; text: "Get an alert when a pull request newly needs your attention."; color: Style.secondary; font.pixelSize: 11; wrapMode: Text.Wrap; lineHeight: 1.4 }
                            }
                            Switch {
                                id: toggle
                                objectName: "notifications-toggle"
                                checked: preferences.draftNotifications
                                onToggled: preferences.draftNotifications = checked
                                Accessible.name: "Show desktop notifications"
                                padding: 3
                                indicator: Rectangle {
                                    implicitWidth: 36; implicitHeight: 21; x: 3; y: 3; radius: 11
                                    color: toggle.checked ? Style.accent : "#b4bbca"
                                    border.width: toggle.activeFocus ? 2 : 0; border.color: Style.ink
                                    Rectangle { x: toggle.checked ? 18 : 3; y: 3; width: 15; height: 15; radius: 8; color: "white" }
                                }
                            }
                        }
                    }
                    Rectangle {
                        Layout.fillWidth: true; implicitHeight: note.implicitHeight + 28; radius: 9; color: Style.canvas
                        Label { id: note; anchors.fill: parent; anchors.margins: 14; text: preferences.draftNotifications ? "Alerts use your desktop notification service while Review Radar is running. Existing activity establishes a quiet baseline." : "Desktop alerts are off. Your workspace keeps updating. Turning alerts back on will not replay updates observed while muted."; wrapMode: Text.Wrap; color: Style.secondary; font.pixelSize: 11; lineHeight: 1.5 }
                    }
                    RowLayout {
                        Layout.fillWidth: true; Layout.topMargin: 10; spacing: 12
                        Label { Layout.fillWidth: true; text: "NOTIFICATION PREVIEW"; color: Style.secondary; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1.2; wrapMode: Text.Wrap }
                        RadarButton {
                            objectName: "test-notification"
                            text: "Test notification"
                            onClicked: {
                                preferences.testSucceeded = preferences.controller.testNotification()
                                preferences.testMessage = preferences.testSucceeded
                                    ? "Test sent to your desktop. If it does not appear, check your desktop notification settings."
                                    : "Could not send the test notification. Check that your desktop notification service is available."
                            }
                        }
                    }
                    Label { Layout.fillWidth: true; text: "Sends one test even when alerts are off. No preferences are saved."; color: Style.secondary; font.pixelSize: 11; wrapMode: Text.Wrap }
                    Label {
                        Layout.fillWidth: true; visible: preferences.testMessage.length > 0
                        text: preferences.testMessage; color: preferences.testSucceeded ? Style.secondary : Style.negative
                        font.pixelSize: 11; wrapMode: Text.Wrap; Accessible.name: text
                    }
                    Rectangle {
                        Layout.fillWidth: true; implicitHeight: preview.implicitHeight + 34; color: Style.canvas; radius: 10
                        ColumnLayout {
                            id: preview
                            anchors.fill: parent; anchors.margins: 17; spacing: 10
                            Rectangle {
                                Layout.fillWidth: true; implicitHeight: notification.implicitHeight + 32; color: "white"; radius: 12; border.color: Style.line
                                RowLayout {
                                    id: notification
                                    anchors.fill: parent; anchors.margins: 16; spacing: 12
                                    Image { Layout.alignment: Qt.AlignTop; source: "qrc:/assets/logo.svg"; Layout.preferredWidth: 32; Layout.preferredHeight: 32; Accessible.ignored: true }
                                    ColumnLayout {
                                        Layout.fillWidth: true; spacing: 6
                                        Label { text: "REVIEW RADAR · now"; color: Style.secondary; font.pixelSize: 10 }
                                        Label { text: "Review requested"; color: Style.ink; font.pixelSize: 12; font.weight: Font.DemiBold }
                                        Label { Layout.fillWidth: true; text: "example/api #142\nYour review is outstanding."; color: Style.secondary; font.pixelSize: 11; wrapMode: Text.Wrap; lineHeight: 1.5 }
                                    }
                                }
                            }
                            Label { Layout.fillWidth: true; text: "Illustrative preview · Desktop styling may vary"; color: Style.secondary; font.pixelSize: 10; wrapMode: Text.Wrap }
                        }
                    }
                    Label { Layout.topMargin: 10; text: "MORE OPTIONS"; color: Style.secondary; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1.2 }
                    Label { Layout.fillWidth: true; text: "Category filters, sound, preview detail, and quiet hours are planned. The master switch above is available now."; color: Style.secondary; font.pixelSize: 11; wrapMode: Text.Wrap; lineHeight: 1.5 }
                }
                ColumnLayout {
                    visible: preferences.section === "Desktop integration"
                    Layout.fillWidth: true; Layout.leftMargin: 30; Layout.rightMargin: 30; spacing: 12
                    Label { text: "SYSTEM TRAY"; color: Style.secondary; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1.2 }
                    Label {
                        Layout.fillWidth: true; wrapMode: Text.Wrap; font.pixelSize: 12; color: Style.secondary
                        text: preferences.controller.trayAvailable
                            ? "Your desktop supports a system-tray icon."
                            : "No system tray is available on this desktop. Closing the window will exit normally."
                    }
                    SettingSwitch {
                        objectName: "tray-setting"
                        label: "Show system-tray icon"
                        description: "Keep your review queue within reach. Click to open; use the menu to refresh, open Preferences, or quit."
                        checked: preferences.draftTray
                        enabled: preferences.controller.trayAvailable || preferences.draftTray
                        onChanged: function(value) { preferences.draftTray = value; if (!value) preferences.draftCloseToTray = false }
                    }
                    SettingSwitch {
                        label: "Show attention dot"
                        description: "Highlight attention-required PRs in the current workspace. The tooltip shows their count."
                        checked: preferences.draftAttentionDot; enabled: preferences.draftTray
                        onChanged: function(value) { preferences.draftAttentionDot = value }
                    }
                    Label { Layout.topMargin: 10; text: "WINDOW BEHAVIOR"; color: Style.secondary; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1.2 }
                    SettingSwitch {
                        label: "Keep running when the window closes"
                        description: "Continue refreshing and notifying from the tray. Use Quit in the tray menu to exit. Requires an available tray."
                        checked: preferences.draftCloseToTray; enabled: preferences.draftTray && preferences.controller.trayAvailable
                        onChanged: function(value) { preferences.draftCloseToTray = value }
                    }
                    Label { Layout.topMargin: 10; text: "QUICKSHELL"; color: Style.secondary; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1.2 }
                    SettingSwitch {
                        objectName: "bar-setting"
                        label: "Enable bar integration"
                        description: "Let Quickshell display this workspace's attention count and sync state. Review Radar must remain running."
                        checked: preferences.draftBar
                        onChanged: function(value) { preferences.draftBar = value }
                    }
                    Label {
                        Layout.fillWidth: true; color: Style.secondary; font.pixelSize: 12; wrapMode: Text.Wrap
                        text: preferences.draftBar !== preferences.controller.barEnabled
                            ? "Save to apply your bar integration choice."
                            : !preferences.controller.barEnabled ? "Bar integration is off."
                            : preferences.controller.barActive ? "Status interface available. Add the ReviewRadar component to your Quickshell bar."
                            : "Could not publish the status interface. Check the session bus and whether another Review Radar instance is running; toggle off and on to retry."
                    }
                    Label {
                        Layout.fillWidth: true; text: "Setup: copy apps/quickshell/review-radar into your Quickshell configuration, import that directory, and add ReviewRadar {} to your bar. Requires busctl. See apps/quickshell/README.md for an example."; color: Style.secondary; font.pixelSize: 11; wrapMode: Text.Wrap; lineHeight: 1.5
                    }
                }
                ColumnLayout {
                    visible: preferences.section !== "Notifications" && preferences.section !== "Desktop integration"
                    Layout.fillWidth: true; Layout.leftMargin: 30; Layout.rightMargin: 30; spacing: 12
                    Label { Layout.fillWidth: true; text: "Planned preferences · These options are not configurable yet."; color: Style.secondary; font.pixelSize: 12; wrapMode: Text.Wrap }
                    Repeater {
                        model: preferences.planned[preferences.section] || []
                        delegate: ColumnLayout {
                            required property var modelData
                            Layout.fillWidth: true; spacing: 10
                            Label { Layout.topMargin: 10; text: modelData.title; color: Style.secondary; font.pixelSize: 10; font.weight: Font.Bold; font.letterSpacing: 1.2 }
                            Rectangle {
                                Layout.fillWidth: true; implicitHeight: rows.implicitHeight; radius: 11; color: "white"; border.color: Style.line
                                ColumnLayout {
                                    id: rows
                                    width: parent.width; spacing: 0
                                    Repeater {
                                        model: modelData.rows
                                        delegate: RowLayout {
                                            required property string modelData
                                            Layout.fillWidth: true; Layout.margins: 18
                                            Label { Layout.fillWidth: true; text: modelData; color: Style.secondary; font.pixelSize: 13; wrapMode: Text.Wrap }
                                            Label { text: "Planned"; color: Style.secondary; font.pixelSize: 10; padding: 5; background: Rectangle { radius: 5; color: Style.sidebar } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Label { Layout.fillWidth: true; Layout.leftMargin: 30; Layout.rightMargin: 30; visible: preferences.errorMessage.length > 0; text: preferences.errorMessage; color: Style.negative; font.pixelSize: 12; wrapMode: Text.Wrap; Accessible.name: text }
                Item { Layout.preferredHeight: 20 }
            }
        }
    }
    footer: Rectangle {
        implicitHeight: 74; color: "transparent"
        Rectangle { width: parent.width; height: 1; color: Style.line }
        RowLayout {
            anchors.fill: parent; anchors.margins: 20; spacing: 9
            Label { Layout.fillWidth: true; text: preferences.dirty ? "Unsaved changes" : "Changes apply to this device."; color: Style.secondary; font.pixelSize: 11; wrapMode: Text.Wrap }
            RadarButton { text: "Cancel"; onClicked: preferences.requestClose() }
            RadarButton {
                objectName: "save-preferences"; text: "Save changes"; primary: true; enabled: preferences.dirty
                onClicked: {
                    if (preferences.controller.saveIntegrationPreferences(preferences.draftNotifications, preferences.draftTray, preferences.draftCloseToTray, preferences.draftAttentionDot, preferences.draftBar)) preferences.close()
                    else preferences.errorMessage = "Could not save preferences. Your changes have not been applied. Try again."
                }
            }
        }
    }
    RadarModal { id: discard; objectName: "discard-preferences"; title: "Discard preference changes?"; subtitle: "Your saved preferences will stay as they are."; confirmation: true; actionText: "Discard changes"; onAccepted: preferences.close() }
}
