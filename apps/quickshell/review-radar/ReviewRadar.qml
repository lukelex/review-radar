import QtQuick
import QtQuick.Controls
import Quickshell.Io

// A presentation adapter only. All counts and sync decisions come from the app.
Button {
    id: root
    property var snapshot: null
    property string actionError: ""
    property double lastResponse: 0
    readonly property bool connected: snapshot !== null
    readonly property var workspaceNames: ({ "tailored": "Tailored to you", "action": "Action", "my-prs": "My PRs", "following": "Following", "recent": "Recent" })
    readonly property string summary: !connected ? "Review Radar unavailable"
        : !snapshot.available ? "Review Radar · " + snapshot.syncState
        : (workspaceNames[snapshot.workspace] || snapshot.workspace) + ": " + snapshot.attentionCount
          + " needing attention · " + snapshot.syncState
    text: !connected ? "Radar —" : !snapshot.available ? "Radar …"
        : "Radar " + snapshot.attentionCount + (snapshot.syncState === "ready" ? "" : " · " + snapshot.syncState)
    Accessible.name: summary
    ToolTip.visible: hovered
    ToolTip.text: summary + (connected && snapshot.capturedAt ? "\nLast capture: " + snapshot.capturedAt : "")
        + (!connected ? "\nOpen Review Radar and enable Preferences → Desktop integration → Bar integration." : "\nClick to open. Right-click for actions.")
        + (actionError ? "\n" + actionError : "")
    hoverEnabled: true
    implicitHeight: 30
    implicitWidth: Math.max(80, contentItem.implicitWidth + 24)
    padding: 8
    contentItem: Text { text: root.text; color: "#20283f"; font.pixelSize: 12; verticalAlignment: Text.AlignVCenter; horizontalAlignment: Text.AlignHCenter }
    background: Rectangle {
        radius: 7
        color: root.hovered ? "#e5e3fa" : "#f0f2f8"
        border.width: root.activeFocus ? 2 : 0
        border.color: "#635bdf"
    }
    function command(method) {
        return ["busctl", "--user", "--timeout=2s", "--auto-start=no", "--json=short", "call",
                "org.reviewradar.App", "/org/reviewradar/Bar", "org.reviewradar.Bar1", method]
    }
    function invoke(method) {
        if (!connected || action.running) return
        actionError = ""
        action.command = command(method)
        action.running = true
    }
    function readSnapshot() {
        if (!read.running) read.running = true
    }
    onClicked: invoke("OpenWorkspace")
    Component.onCompleted: readSnapshot()
    Timer {
        interval: 3000; repeat: true; running: true
        onTriggered: {
            if (Date.now() - root.lastResponse > 10000) root.snapshot = null
            root.readSnapshot()
        }
    }
    Process {
        id: read
        command: root.command("GetSnapshot")
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const envelope = JSON.parse(text)
                    const value = JSON.parse(envelope.data[0])
                    if (value.version !== 1 || typeof value.available !== "boolean"
                        || typeof value.syncState !== "string" || typeof value.workspace !== "string"
                        || typeof value.capturedAt !== "string" || !Number.isInteger(value.attentionCount)
                        || value.attentionCount < 0) throw new Error("Unsupported status")
                    root.snapshot = value
                    root.lastResponse = Date.now()
                } catch (_) { root.snapshot = null }
            }
        }
        onExited: function(exitCode, exitStatus) {
            if (exitCode !== 0 || exitStatus !== 0) root.snapshot = null
        }
    }
    Process {
        id: action
        onExited: function(exitCode, exitStatus) {
            if (exitCode !== 0 || exitStatus !== 0) {
                root.actionError = "Could not contact Review Radar."
                root.snapshot = null
            }
            root.readSnapshot()
        }
    }
    TapHandler { acceptedButtons: Qt.RightButton; onTapped: menu.popup() }
    Keys.onMenuPressed: menu.popup()
    Menu {
        id: menu
        MenuItem { text: "Open Review Radar"; enabled: root.connected; onTriggered: root.invoke("OpenWorkspace") }
        MenuItem { text: "Refresh"; enabled: root.connected; onTriggered: root.invoke("Refresh") }
        MenuItem { text: "Preferences"; enabled: root.connected; onTriggered: root.invoke("OpenPreferences") }
    }
}
