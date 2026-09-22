import QtQuick
import QtQuick.Controls

Label {
    id: badge
    required property string level
    required property string status
    readonly property string normalizedLevel: String(level || "").toLowerCase()
    readonly property bool assessed: normalizedLevel === "low" || normalizedLevel === "moderate" || normalizedLevel === "high"
    readonly property string value: assessed ? Style.humanize(normalizedLevel)
                                            : Style.humanize(status || "unknown")
    readonly property string icon: normalizedLevel === "low" ? "▁"
                               : normalizedLevel === "moderate" ? "▂"
                               : normalizedLevel === "high" ? "▃" : "?"
    text: "Friction · " + icon + " " + value
    color: normalizedLevel === "low" ? Style.frictionLow
         : normalizedLevel === "moderate" ? Style.frictionMedium
         : normalizedLevel === "high" ? Style.frictionHigh : Style.frictionUnknown
    padding: 8
    topPadding: 5
    bottomPadding: 5
    font.pixelSize: 11
    font.weight: Font.DemiBold
    textFormat: Text.PlainText
    Accessible.name: "Review friction: " + value + ". " + (status || "Not assessed")
    Accessible.role: Accessible.StaticText
    background: Rectangle {
        radius: 6
        color: badge.normalizedLevel === "low" ? Style.frictionLowTint
             : badge.normalizedLevel === "moderate" ? Style.frictionMediumTint
             : badge.normalizedLevel === "high" ? Style.frictionHighTint : Style.frictionUnknownTint
    }
}
