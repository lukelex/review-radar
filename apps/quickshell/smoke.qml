import QtQuick
import Quickshell
import "review-radar" as Radar

ShellRoot {
    Radar.ReviewRadar { id: radar }
    property int attempts: 0
    property bool requested: false
    Timer {
        interval: 100; running: true; repeat: true
        onTriggered: {
            attempts++
            if (attempts > 50) { console.error("Bar connection/disconnection timed out"); Qt.exit(1); return }
            if (requested) {
                if (!radar.connected) Qt.quit()
                return
            }
            if (!radar.connected) return
            if (radar.snapshot.attentionCount !== 3 || radar.snapshot.workspace !== "action"
                || radar.snapshot.syncState !== "syncing") {
                console.error("Unexpected bar snapshot"); Qt.exit(1); return
            }
            requested = true
            radar.invoke("Refresh")
        }
    }
}
