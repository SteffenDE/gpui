import AppKit
import ApplicationServices
import CoreGraphics
import Carbon
import Foundation
import ScreenCaptureKit

struct DriverError: Error, CustomStringConvertible {
    let description: String
}

struct WindowInfo {
    let id: CGWindowID
    let ownerPID: pid_t
    let bounds: CGRect
    let title: String
}

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(1)
}

func windows() throws -> [WindowInfo] {
    guard let values = CGWindowListCopyWindowInfo(
        [.optionOnScreenOnly, .excludeDesktopElements],
        kCGNullWindowID
    ) as? [[String: Any]] else {
        throw DriverError(description: "unable to enumerate windows")
    }

    return values.compactMap { value in
        guard
            let number = value[kCGWindowNumber as String] as? NSNumber,
            let ownerPID = value[kCGWindowOwnerPID as String] as? NSNumber,
            let boundsValue = value[kCGWindowBounds as String] as? [String: Any],
            let bounds = CGRect(dictionaryRepresentation: boundsValue as CFDictionary),
            let layer = value[kCGWindowLayer as String] as? NSNumber,
            layer.intValue == 0
        else { return nil }

        return WindowInfo(
            id: CGWindowID(number.uint32Value),
            ownerPID: pid_t(ownerPID.int32Value),
            bounds: bounds,
            title: value[kCGWindowName as String] as? String ?? ""
        )
    }
}

func window(id: String) throws -> WindowInfo {
    guard let parsed = UInt32(id), let match = try windows().first(where: { $0.id == parsed }) else {
        throw DriverError(description: "window not found: \(id)")
    }
    return match
}

func activate(_ info: WindowInfo) {
    NSRunningApplication(processIdentifier: info.ownerPID)?.activate(options: [.activateAllWindows])
    let app = AXUIElementCreateApplication(info.ownerPID)
    var windowsValue: CFTypeRef?
    if AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &windowsValue) == .success,
       let axWindows = windowsValue as? [AXUIElement] {
        for axWindow in axWindows {
            var titleValue: CFTypeRef?
            AXUIElementCopyAttributeValue(axWindow, kAXTitleAttribute as CFString, &titleValue)
            if (titleValue as? String) == info.title {
                AXUIElementPerformAction(axWindow, kAXRaiseAction as CFString)
                break
            }
        }
    }
    Thread.sleep(forTimeInterval: 0.08)
}

func axPoint(_ value: CFTypeRef?) -> CGPoint? {
    guard let value else { return nil }
    var point = CGPoint.zero
    return AXValueGetValue(value as! AXValue, .cgPoint, &point) ? point : nil
}

func axSize(_ value: CFTypeRef?) -> CGSize? {
    guard let value else { return nil }
    var size = CGSize.zero
    return AXValueGetValue(value as! AXValue, .cgSize, &size) ? size : nil
}

func accessibilityWindow(_ info: WindowInfo) -> AXUIElement? {
    let app = AXUIElementCreateApplication(info.ownerPID)
    var windowsValue: CFTypeRef?
    guard AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &windowsValue) == .success,
          let axWindows = windowsValue as? [AXUIElement]
    else { return nil }

    return axWindows.first { axWindow in
        var titleValue: CFTypeRef?
        AXUIElementCopyAttributeValue(axWindow, kAXTitleAttribute as CFString, &titleValue)
        return (titleValue as? String) == info.title
    }
}

func contentBounds(_ info: WindowInfo) -> CGRect {
    guard let axWindow = accessibilityWindow(info) else { return info.bounds }
    var childrenValue: CFTypeRef?
    guard AXUIElementCopyAttributeValue(axWindow, kAXChildrenAttribute as CFString, &childrenValue) == .success,
          let children = childrenValue as? [AXUIElement]
    else { return info.bounds }

    for child in children {
        var roleValue: CFTypeRef?
        var positionValue: CFTypeRef?
        var sizeValue: CFTypeRef?
        AXUIElementCopyAttributeValue(child, kAXRoleAttribute as CFString, &roleValue)
        AXUIElementCopyAttributeValue(child, kAXPositionAttribute as CFString, &positionValue)
        AXUIElementCopyAttributeValue(child, kAXSizeAttribute as CFString, &sizeValue)
        guard (roleValue as? String) == kAXGroupRole as String,
              let origin = axPoint(positionValue),
              let size = axSize(sizeValue),
              abs(size.width - info.bounds.width) < 1
        else { continue }
        return CGRect(origin: origin, size: size)
    }
    return info.bounds
}

func screenPoint(_ info: WindowInfo, x: String, y: String) throws -> CGPoint {
    guard let x = Double(x), let y = Double(y) else {
        throw DriverError(description: "coordinates must be numbers")
    }
    let content = contentBounds(info)
    return CGPoint(x: content.minX + x, y: content.minY + y)
}

func postMouse(_ type: CGEventType, point: CGPoint, button: CGMouseButton = .left, flags: CGEventFlags = [], clickCount: Int64 = 1) throws {
    guard let source = CGEventSource(stateID: .hidSystemState),
          let event = CGEvent(mouseEventSource: source, mouseType: type, mouseCursorPosition: point, mouseButton: button) else {
        throw DriverError(description: "unable to create mouse event")
    }
    event.flags = flags
    event.setIntegerValueField(.mouseEventClickState, value: clickCount)
    event.post(tap: .cghidEventTap)
}

func move(windowID: String, x: String, y: String) throws {
    let info = try window(id: windowID)
    activate(info)
    try postMouse(.mouseMoved, point: screenPoint(info, x: x, y: y))
    Thread.sleep(forTimeInterval: 0.03)
}

func modifiers(windowID: String, description: String) throws {
    let info = try window(id: windowID)
    activate(info)
    var flags: CGEventFlags = []
    for modifier in description.lowercased().split(separator: "+").map(String.init) {
        switch modifier {
        case "super", "cmd", "command": flags.insert(.maskCommand)
        case "ctrl", "control": flags.insert(.maskControl)
        case "alt", "option": flags.insert(.maskAlternate)
        case "shift": flags.insert(.maskShift)
        case "none", "": break
        default: throw DriverError(description: "unsupported modifier: \(modifier)")
        }
    }
    guard let source = CGEventSource(stateID: .hidSystemState),
          let event = CGEvent(keyboardEventSource: source, virtualKey: 54, keyDown: !flags.isEmpty) else {
        throw DriverError(description: "unable to create modifier event")
    }
    event.type = .flagsChanged
    event.flags = flags
    event.postToPid(info.ownerPID)
    Thread.sleep(forTimeInterval: 0.03)
}

func scroll(windowID: String, x: String, y: String, deltaX: String, deltaY: String, modifiers: String = "") throws {
    let info = try window(id: windowID)
    activate(info)
    let point = try screenPoint(info, x: x, y: y)
    guard let deltaX = Int32(deltaX), let deltaY = Int32(deltaY),
          abs(Int64(deltaX)) <= 100_000, abs(Int64(deltaY)) <= 100_000,
          let source = CGEventSource(stateID: .hidSystemState),
          let event = CGEvent(
              scrollWheelEvent2Source: source,
              units: .pixel,
              wheelCount: 2,
              wheel1: deltaY,
              wheel2: deltaX,
              wheel3: 0
          )
    else { throw DriverError(description: "invalid scroll delta") }
    try postMouse(.mouseMoved, point: point)
    event.location = point
    var flags: CGEventFlags = []
    for modifier in modifiers.lowercased().split(separator: "+").map(String.init) {
        switch modifier {
        case "shift": flags.insert(.maskShift)
        case "super", "cmd", "command": flags.insert(.maskCommand)
        case "ctrl", "control": flags.insert(.maskControl)
        case "alt", "option": flags.insert(.maskAlternate)
        case "none", "": break
        default: throw DriverError(description: "unsupported modifier: \(modifier)")
        }
    }
    event.flags = flags
    event.post(tap: .cghidEventTap)
    Thread.sleep(forTimeInterval: 0.08)
}

func click(windowID: String, x: String, y: String, modifiers: String = "") throws {
    var flags: CGEventFlags = []
    for modifier in modifiers.split(separator: "+") {
        switch modifier {
        case "super", "cmd", "command": flags.insert(.maskCommand)
        case "ctrl", "control": flags.insert(.maskControl)
        case "alt", "option": flags.insert(.maskAlternate)
        case "shift": flags.insert(.maskShift)
        default: throw DriverError(description: "invalid mouse modifier")
        }
    }
    let info = try window(id: windowID)
    activate(info)
    let point = try screenPoint(info, x: x, y: y)
    try postMouse(.mouseMoved, point: point)
    Thread.sleep(forTimeInterval: 0.03)
    try postMouse(.leftMouseDown, point: point, flags: flags)
    Thread.sleep(forTimeInterval: 0.05)
    try postMouse(.leftMouseUp, point: point, flags: flags)
    Thread.sleep(forTimeInterval: 0.05)
}

let keyCodes: [String: CGKeyCode] = [
    "a": 0, "s": 1, "d": 2, "f": 3, "h": 4, "g": 5,
    "z": 6, "x": 7, "c": 8, "v": 9, "b": 11,
    "q": 12, "w": 13, "e": 14, "r": 15, "y": 16, "t": 17,
    "1": 18, "2": 19, "3": 20, "4": 21, "6": 22, "5": 23,
    "9": 25, "7": 26, "8": 28, "0": 29,
    "o": 31, "u": 32, "i": 34, "p": 35, "l": 37,
    "j": 38, "k": 40, "n": 45, "m": 46,
    ".": 47, "/": 44, "[": 33, "]": 30,
    "tab": 48, "space": 49, "return": 36, "enter": 36,
    "escape": 53, "esc": 53, "backspace": 51, "delete": 51,
    "end": 119, "home": 115,
    "left": 123, "right": 124, "down": 125, "up": 126
]

// Resolve logical shortcut keys against the active layout (including Command's
// alternate layout), rather than assuming US hardware positions.
func layoutKeyCode(_ name: String, flags: CGEventFlags, fallback: CGKeyCode) -> CGKeyCode {
    guard name.count == 1,
          let source = TISCopyCurrentKeyboardLayoutInputSource()?.takeRetainedValue(),
          let property = TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData)
    else { return fallback }
    let data = unsafeBitCast(property, to: CFData.self)
    let layout = UnsafeRawPointer(CFDataGetBytePtr(data)).assumingMemoryBound(to: UCKeyboardLayout.self)
    let modifiers = flags.contains(.maskCommand) ? UInt32(cmdKey >> 8) : 0
    for code in UInt16(0)..<UInt16(128) {
        var dead: UInt32 = 0
        var length = 0
        var chars = [UniChar](repeating: 0, count: 8)
        let status = UCKeyTranslate(layout, code, UInt16(kUCKeyActionDown), modifiers,
            UInt32(LMGetKbdType()), OptionBits(kUCKeyTranslateNoDeadKeysBit),
            &dead, chars.count, &length, &chars)
        if status == noErr && String(utf16CodeUnits: chars, count: length).lowercased() == name {
            return code
        }
    }
    return fallback
}

func key(windowID: String, description: String) throws {
    let info = try window(id: windowID)
    activate(info)
    try postKey(info, description: description)
}

func keys(windowID: String, descriptions: [String]) throws {
    guard !descriptions.isEmpty, descriptions.count <= 64 else {
        throw DriverError(description: "keys requires between 1 and 64 key descriptions")
    }
    let info = try window(id: windowID)
    activate(info)
    for description in descriptions { try postKey(info, description: description) }
}

func postKey(_ info: WindowInfo, description: String, phases: [Bool] = [true, false]) throws {
    let parts = description.lowercased().split(separator: "+").map(String.init)
    guard let name = parts.last, let code = keyCodes[name] else {
        throw DriverError(description: "unsupported key: \(description)")
    }
    var flags: CGEventFlags = []
    for modifier in parts.dropLast() {
        let resolved = modifier == "primary" ? "command" : modifier
        switch resolved {
        case "super", "cmd", "command": flags.insert(.maskCommand)
        case "ctrl", "control": flags.insert(.maskControl)
        case "alt", "option": flags.insert(.maskAlternate)
        case "shift": flags.insert(.maskShift)
        default: throw DriverError(description: "unsupported modifier: \(modifier)")
        }
    }
    let resolvedCode = layoutKeyCode(name, flags: flags, fallback: code)
    for down in phases {
        guard let source = CGEventSource(stateID: .hidSystemState),
              let event = CGEvent(keyboardEventSource: source, virtualKey: resolvedCode, keyDown: down) else {
            throw DriverError(description: "unable to create keyboard event")
        }
        event.flags = flags
        event.postToPid(info.ownerPID)
        Thread.sleep(forTimeInterval: 0.03)
    }
}

func typeText(windowID: String, text: String) throws {
    let info = try window(id: windowID)
    activate(info)
    for character in text {
        let units = Array(String(character).utf16)
        guard let source = CGEventSource(stateID: .hidSystemState),
              let down = CGEvent(keyboardEventSource: source, virtualKey: 0, keyDown: true),
              let up = CGEvent(keyboardEventSource: source, virtualKey: 0, keyDown: false)
        else { throw DriverError(description: "unable to create text event") }
        down.flags = []
        up.flags = []
        down.keyboardSetUnicodeString(stringLength: units.count, unicodeString: units)
        up.keyboardSetUnicodeString(stringLength: units.count, unicodeString: units)
        down.post(tap: .cghidEventTap)
        up.post(tap: .cghidEventTap)
        Thread.sleep(forTimeInterval: 0.03)
    }
}

func accessibilityTree(windowID: String) throws {
    let info = try window(id: windowID)
    guard let root = accessibilityWindow(info) else {
        throw DriverError(description: "unable to query accessibility window")
    }
    var remaining = 10000
    func snapshot(_ element: AXUIElement, depth: Int) -> [String: Any] {
        remaining -= 1
        var node: [String: Any] = [:]
        for (name, attribute) in [
            ("role", kAXRoleAttribute), ("title", kAXTitleAttribute),
            ("description", kAXDescriptionAttribute), ("value", kAXValueAttribute),
            ("focused", kAXFocusedAttribute), ("enabled", kAXEnabledAttribute),
            ("expanded", kAXExpandedAttribute)
        ] {
            var value: CFTypeRef?
            if AXUIElementCopyAttributeValue(element, attribute as CFString, &value) == .success,
               let value = value {
                if let text = value as? String { node[name] = text }
                else if let number = value as? NSNumber { node[name] = number }
            }
        }
        var actions: CFArray?
        if AXUIElementCopyActionNames(element, &actions) == .success,
           let names = actions as? [String] { node["actions"] = names }
        var children: CFTypeRef?
        if depth < 64 && remaining > 0,
           AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &children) == .success,
           let elements = children as? [AXUIElement] {
            node["children"] = elements.prefix(remaining).map { snapshot($0, depth: depth + 1) }
        }
        return node
    }
    let data = try JSONSerialization.data(withJSONObject: snapshot(root, depth: 0), options: [.sortedKeys])
    print(String(decoding: data, as: UTF8.self))
}

func accessibilityPress(windowID: String, label: String) throws {
    let info = try window(id: windowID)
    guard let root = accessibilityWindow(info) else {
        throw DriverError(description: "unable to query accessibility window")
    }
    func find(_ element: AXUIElement, depth: Int) -> AXUIElement? {
        for attribute in [kAXTitleAttribute, kAXDescriptionAttribute] {
            var value: CFTypeRef?
            if AXUIElementCopyAttributeValue(element, attribute as CFString, &value) == .success,
               value as? String == label {
                var actions: CFArray?
                if AXUIElementCopyActionNames(element, &actions) == .success,
                   let names = actions as? [String], names.contains(kAXPressAction as String) { return element }
            }
        }
        var children: CFTypeRef?
        if depth < 64,
           AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &children) == .success,
           let elements = children as? [AXUIElement] {
            for child in elements { if let match = find(child, depth: depth + 1) { return match } }
        }
        return nil
    }
    guard let target = find(root, depth: 0) else {
        throw DriverError(description: "accessible press target not found: \(label)")
    }
    let result = AXUIElementPerformAction(target, kAXPressAction as CFString)
    guard result == .success else { throw DriverError(description: "accessible press failed: \(result.rawValue)") }
}

func close(windowID: String) throws {
    let info = try window(id: windowID)
    activate(info)
    let app = AXUIElementCreateApplication(info.ownerPID)
    var windowsValue: CFTypeRef?
    guard AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &windowsValue) == .success,
          let axWindows = windowsValue as? [AXUIElement]
    else { throw DriverError(description: "unable to query accessibility windows") }

    for axWindow in axWindows {
        var titleValue: CFTypeRef?
        AXUIElementCopyAttributeValue(axWindow, kAXTitleAttribute as CFString, &titleValue)
        guard (titleValue as? String) == info.title else { continue }
        var closeValue: CFTypeRef?
        guard AXUIElementCopyAttributeValue(axWindow, kAXCloseButtonAttribute as CFString, &closeValue) == .success,
              let closeButton = closeValue
        else { throw DriverError(description: "window has no accessibility close button") }
        let result = AXUIElementPerformAction(closeButton as! AXUIElement, kAXPressAction as CFString)
        guard result == .success else { throw DriverError(description: "close action failed: \(result.rawValue)") }
        return
    }
    throw DriverError(description: "accessibility window not found")
}

func doubleClick(windowID: String, x: String, y: String) throws {
    let info = try window(id: windowID)
    activate(info)
    let point = try screenPoint(info, x: x, y: y)
    try postMouse(.mouseMoved, point: point)
    for count in 1...2 {
        try postMouse(.leftMouseDown, point: point, clickCount: Int64(count))
        Thread.sleep(forTimeInterval: 0.03)
        try postMouse(.leftMouseUp, point: point, clickCount: Int64(count))
        Thread.sleep(forTimeInterval: 0.03)
    }
}

func repeatClick(windowID: String, x: String, y: String, count: String) throws {
    guard let count = Int(count), count > 0 else {
        throw DriverError(description: "repeat count must be positive")
    }
    for _ in 0..<count { try click(windowID: windowID, x: x, y: y) }
}

func drag(windowID: String, fromX: String, fromY: String, toX: String, toY: String) throws {
    let info = try window(id: windowID)
    activate(info)
    let start = try screenPoint(info, x: fromX, y: fromY)
    let finish = try screenPoint(info, x: toX, y: toY)
    try postMouse(.mouseMoved, point: start)
    try postMouse(.leftMouseDown, point: start)
    Thread.sleep(forTimeInterval: 0.05)
    try postMouse(.leftMouseDragged, point: finish)
    Thread.sleep(forTimeInterval: 0.05)
    try postMouse(.leftMouseUp, point: finish)
}

func pointer(windowID: String, x: String, y: String, phase: String) throws {
    let info = try window(id: windowID)
    activate(info)
    let point = try screenPoint(info, x: x, y: y)
    if phase == "pointer-down" {
        try postMouse(.mouseMoved, point: point)
        Thread.sleep(forTimeInterval: 0.03)
    }
    let kind: CGEventType = phase == "pointer-down" ? .leftMouseDown : (phase == "pointer-up" ? .leftMouseUp : .leftMouseDragged)
    try postMouse(kind, point: point)
    Thread.sleep(forTimeInterval: 0.03)
}

func resize(windowID: String, width: String, height: String) throws {
    guard let width = Double(width), let height = Double(height),
          let axWindow = accessibilityWindow(try window(id: windowID))
    else { throw DriverError(description: "unable to resolve resizable window") }
    var size = CGSize(width: width, height: height)
    guard let value = AXValueCreate(.cgSize, &size) else {
        throw DriverError(description: "unable to encode window size")
    }
    let result = AXUIElementSetAttributeValue(axWindow, kAXSizeAttribute as CFString, value)
    guard result == .success else {
        throw DriverError(description: "resize failed: \(result.rawValue)")
    }
}

func quartzFrame(_ screen: NSScreen) -> CGRect {
    guard let id = screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? CGDirectDisplayID else {
        return screen.frame
    }
    return CGDisplayBounds(id)
}

func containingScreen(_ bounds: CGRect) -> NSScreen? {
    NSScreen.screens.max { left, right in
        quartzFrame(left).intersection(bounds).width * quartzFrame(left).intersection(bounds).height <
            quartzFrame(right).intersection(bounds).width * quartzFrame(right).intersection(bounds).height
    }
}

func windowInfo(windowID: String) throws {
    let info = try window(id: windowID)
    let content = contentBounds(info)
    guard let screen = containingScreen(info.bounds),
          let displayID = screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? CGDirectDisplayID
    else { throw DriverError(description: "target display not found") }
    let display = quartzFrame(screen)
    var values: [String] = []
    values.append(String(info.id))
    for value in [info.bounds.minX, info.bounds.minY, info.bounds.width, info.bounds.height] {
        values.append(String(Double(value)))
    }
    for value in [content.minX, content.minY, content.width, content.height] {
        values.append(String(Double(value)))
    }
    values.append(String(displayID))
    for value in [display.minX, display.minY, display.width, display.height] {
        values.append(String(Double(value)))
    }
    values.append(String(Double(screen.backingScaleFactor)))
    print(values.joined(separator: "\t"))
}

func pixel(windowID: String, path: String, x: String, y: String, raw: Bool = false) throws {
    let info = try window(id: windowID)
    guard let logicalX = Double(x), let logicalY = Double(y),
          let bitmap = NSBitmapImageRep(data: try Data(contentsOf: URL(fileURLWithPath: path))) else {
        throw DriverError(description: "invalid screenshot or pixel coordinates")
    }
    let px = Int(logicalX * Double(bitmap.pixelsWide) / info.bounds.width)
    let py = Int(logicalY * Double(bitmap.pixelsHigh) / info.bounds.height)
    guard px >= 0, py >= 0, px < bitmap.pixelsWide, py < bitmap.pixelsHigh,
          let sample = bitmap.colorAt(x: px, y: py),
          let color = raw ? sample : sample.usingColorSpace(.sRGB) else {
        throw DriverError(description: "screenshot pixel is outside its bounds")
    }
    let values = [color.redComponent, color.greenComponent, color.blueComponent, color.alphaComponent]
    print(values.map { String(Int(($0 * 255).rounded())) }.joined(separator: ","))
}

func capture(windowID: String, path: String) async throws {
    let info = try window(id: windowID)
    let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
    guard let window = content.windows.first(where: { $0.windowID == info.id }) else {
        throw DriverError(description: "capture window not found")
    }
    let display = containingScreen(info.bounds)
    let scale = display?.backingScaleFactor ?? 1
    let configuration = SCStreamConfiguration()
    configuration.width = Int(info.bounds.width * scale)
    configuration.height = Int(info.bounds.height * scale)
    configuration.showsCursor = false
    let filter = SCContentFilter(desktopIndependentWindow: window)
    let image = try await SCScreenshotManager.captureImage(
        contentFilter: filter,
        configuration: configuration
    )
    guard let destination = CGImageDestinationCreateWithURL(
        URL(fileURLWithPath: path) as CFURL,
        "public.png" as CFString,
        1,
        nil
    ) else { throw DriverError(description: "unable to create capture destination") }
    CGImageDestinationAddImage(destination, image, nil)
    guard CGImageDestinationFinalize(destination) else {
        throw DriverError(description: "unable to write capture")
    }
}

func main() async throws {
    let args = Array(CommandLine.arguments.dropFirst())
    switch args.count {
    case 2 where args[0] == "window-info": try windowInfo(windowID: args[1])
    case 2 where args[0] == "find-window":
        let title = args[1]
        let deadline = Date().addingTimeInterval(10)
        repeat {
            if let match = try windows().first(where: { $0.title == title }) {
                print(match.id)
                return
            }
            try await Task.sleep(for: .milliseconds(50))
        } while Date() < deadline
        throw DriverError(description: "window lookup timed out: \(title)")
    case 4 where ["pointer-down", "pointer-up", "pointer-move"].contains(args[0]): try pointer(windowID: args[1], x: args[2], y: args[3], phase: args[0])
    case 4 where args[0] == "move": try move(windowID: args[1], x: args[2], y: args[3])
    case 4 where args[0] == "double-click": try doubleClick(windowID: args[1], x: args[2], y: args[3])
    case 4 where args[0] == "click": try click(windowID: args[1], x: args[2], y: args[3])
    case 5 where args[0] == "click": try click(windowID: args[1], x: args[2], y: args[3], modifiers: args[4])
    case 6 where args[0] == "scroll": try scroll(windowID: args[1], x: args[2], y: args[3], deltaX: args[4], deltaY: args[5])
    case 7 where args[0] == "scroll": try scroll(windowID: args[1], x: args[2], y: args[3], deltaX: args[4], deltaY: args[5], modifiers: args[6])
    case 3 where args[0] == "modifiers": try modifiers(windowID: args[1], description: args[2])
    case 3 where args[0] == "key": try key(windowID: args[1], description: args[2])
    case 3 where args[0] == "key-down" || args[0] == "key-up":
        let info = try window(id: args[1])
        activate(info)
        try postKey(info, description: args[2], phases: [args[0] == "key-down"])

    case let count where count >= 3 && args[0] == "keys": try keys(windowID: args[1], descriptions: Array(args.dropFirst(2)))
    case 3 where args[0] == "type": try typeText(windowID: args[1], text: args[2])
    case 6 where args[0] == "drag": try drag(windowID: args[1], fromX: args[2], fromY: args[3], toX: args[4], toY: args[5])
    case 5 where args[0] == "repeat-click": try repeatClick(windowID: args[1], x: args[2], y: args[3], count: args[4])
    case 4 where args[0] == "resize": try resize(windowID: args[1], width: args[2], height: args[3])
    case 5 where args[0] == "pixel" || args[0] == "pixel-raw": try pixel(windowID: args[1], path: args[2], x: args[3], y: args[4], raw: args[0] == "pixel-raw")
    case 3 where args[0] == "capture": try await capture(windowID: args[1], path: args[2])
    case 3 where args[0] == "accessibility-press": try accessibilityPress(windowID: args[1], label: args[2])
    case 2 where args[0] == "accessibility-tree": try accessibilityTree(windowID: args[1])
    case 2 where args[0] == "close": try close(windowID: args[1])
    default:
        throw DriverError(description: "usage: gpui-desktop-driver find-window TITLE | accessibility-tree WINDOW | accessibility-press WINDOW LABEL | move WINDOW X Y | pointer-down WINDOW X Y | pointer-move WINDOW X Y | pointer-up WINDOW X Y | click WINDOW X Y [MODIFIERS] | double-click WINDOW X Y | scroll WINDOW X Y DELTA_X DELTA_Y [MODIFIERS] | modifiers WINDOW MODIFIERS | key WINDOW KEY | key-down WINDOW KEY | key-up WINDOW KEY | keys WINDOW KEY... | type WINDOW TEXT | capture WINDOW PATH | pixel WINDOW PATH X Y | pixel-raw WINDOW PATH X Y | close WINDOW")
    }
}

do {
    try await main()
} catch {
    fail(String(describing: error))
}
