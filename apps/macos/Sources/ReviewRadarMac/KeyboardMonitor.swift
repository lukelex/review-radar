import AppKit

enum KeyboardAction {
    case moveNext
    case movePrevious
    case pageDown
    case pageUp
    case openDetails
    case closeDetails
    case openCanonical
    case acknowledge
    case snooze
    case copy
    case focusSearch
    case resetWorkspace
    case showHelp
    case nextWorkspace
    case selectWorkspace(Int)
}

final class KeyboardMonitor {
    private var monitor: Any?

    func start(_ handler: @escaping @MainActor (KeyboardAction) -> Void,
               controlChanged: @escaping @MainActor (Bool) -> Void) {
        guard monitor == nil else { return }
        monitor = NSEvent.addLocalMonitorForEvents(matching: [.keyDown, .flagsChanged]) { event in
            if event.type == .flagsChanged {
                let held = event.modifierFlags.contains(.control)
                Task { @MainActor in controlChanged(held) }
                return event
            }
            guard !Self.isEditingText(), let action = Self.action(for: event) else { return event }
            Task { @MainActor in handler(action) }
            return nil
        }
    }

    func stop() {
        if let monitor { NSEvent.removeMonitor(monitor) }
        monitor = nil
    }

    deinit { stop() }

    private static func isEditingText() -> Bool {
        NSApp.keyWindow?.firstResponder is NSTextView
    }

    private static func action(for event: NSEvent) -> KeyboardAction? {
        let modifiers = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        let characters = event.charactersIgnoringModifiers?.lowercased()
        if modifiers == [.control] {
            switch characters {
            case "n": return .nextWorkspace
            case "d": return .pageDown
            case "u": return .pageUp
            case "1": return .selectWorkspace(0)
            case "2": return .selectWorkspace(1)
            case "3": return .selectWorkspace(2)
            case "4": return .selectWorkspace(3)
            case "5": return .selectWorkspace(4)
            default: return nil
            }
        }
        guard modifiers.isEmpty else { return nil }
        switch characters {
        case "j": return .moveNext
        case "k": return .movePrevious
        case "l": return .openDetails
        case "h": return .closeDetails
        case "o": return .openCanonical
        case "a": return .acknowledge
        case "s": return .snooze
        case "y": return .copy
        case "/": return .focusSearch
        case "?": return .showHelp
        case "\u{1b}": return .resetWorkspace
        default: return nil
        }
    }
}
