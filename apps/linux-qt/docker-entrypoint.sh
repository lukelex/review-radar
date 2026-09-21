#!/bin/sh
set -eu

# Prefer Wayland when its mounted socket is available (Hyprland), otherwise use
# XCB through the mounted X11 socket (i3 and other X11 sessions).
if [ -z "${QT_QPA_PLATFORM:-}" ]; then
    if [ -n "${WAYLAND_DISPLAY:-}" ] && [ -S "${XDG_RUNTIME_DIR:-}/$WAYLAND_DISPLAY" ]; then
        export QT_QPA_PLATFORM=wayland
    elif [ -n "${DISPLAY:-}" ]; then
        display_number=${DISPLAY#*:}
        display_number=${display_number%%.*}
        if [ ! -S "/tmp/.X11-unix/X$display_number" ]; then
            echo "X11 display socket for $DISPLAY was not mounted." >&2
            exit 1
        fi
        export QT_QPA_PLATFORM=xcb
    else
        echo "No usable Wayland or X11 display was mounted." >&2
        exit 1
    fi
fi

# A standard user-session bus resides alongside the Wayland socket on both
# Wayland and X11 desktops. It enables desktop notifications and portal-backed
# host URL opening without exposing the host D-Bus system bus.
if [ -z "${DBUS_SESSION_BUS_ADDRESS:-}" ] && [ -S "${XDG_RUNTIME_DIR:-}/bus" ]; then
    export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
fi

exec review-radar-linux "$@"
