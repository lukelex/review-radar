pragma Singleton
import QtQuick

QtObject {
    readonly property color canvas: "#f6f7fb"
    readonly property color sidebar: "#f0f2f8"
    readonly property color ink: "#20283f"
    readonly property color secondary: "#596279"
    readonly property color muted: "#788197"
    readonly property color accent: "#635bdf"
    readonly property color tint: "#efedfc"
    readonly property color line: "#e4e7ef"
    readonly property color positive: "#287a55"
    readonly property color positiveTint: "#e8f6ee"
    readonly property color caution: "#94631b"
    readonly property color cautionTint: "#fff4dc"
    readonly property color negative: "#b33f4a"
    readonly property color negativeTint: "#fdecef"

    function humanize(value) {
        const text = String(value || "").replace(/[-_]/g, " ");
        return text.charAt(0).toUpperCase() + text.slice(1);
    }

    function age(value) {
        const time = Date.parse(value);
        if (!isFinite(time)) return "";
        const minutes = Math.max(0, Math.floor((Date.now() - time) / 60000));
        if (minutes < 1) return "Just now";
        if (minutes < 60) return minutes + "m ago";
        if (minutes < 1440) return Math.floor(minutes / 60) + "h ago";
        return Math.floor(minutes / 1440) + "d ago";
    }
}
