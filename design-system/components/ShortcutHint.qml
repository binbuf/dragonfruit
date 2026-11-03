// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit

// The one renderer for a menu row's shortcut hint. A chord string such as
// `Super+Shift+Z` is parsed into the canonical modifier glyphs plus the key
// label, so menus never print the raw `Ctrl+Alt+Delete` text. A string that is
// not a chord (a Dock window row's workspace name, `Space 2`) falls back to
// plain trailing text in the same cell.
//
// The glyph vocabulary follows the desktop's keymap roles (docs/keymap.md):
// Super/Command -> `⌘`, Control -> `⌃`, Option/Alt -> `⌥`, Shift -> `⇧`,
// Function -> `fn`. Key aliases map to their compact symbols (`⌫`, `↵`, …).
// The glyphs come from the bundled Inter face, so no extra icon asset is
// needed and the hint tints with the row like text.
Item {
    id: root

    property string shortcut: ""
    property color modifierColor: Theme.color.textTertiary
    property color keyColor: Theme.color.textSecondary

    // Parsed rows are plain JS objects: { id, glyph, modifier }.
    readonly property var keys: root.parse(root.shortcut)
    readonly property bool hasShortcut: root.keys.length > 0
    // True for a chord or a plain trailing label (a Dock Space name). The row
    // reserves the trailing cell in both cases.
    readonly property bool hasContent: root.shortcut.length > 0

    function _modifierFor(token) {
        switch (token.toLowerCase()) {
        case "super":
        case "cmd":
        case "command":
        case "meta":
        case "win":
        case "windows":
        case "⌘":
        case "❖":
        case "◆":
            return { id: "super", glyph: "⌘", modifier: true };
        case "ctrl":
        case "control":
        case "⌃":
            return { id: "ctrl", glyph: "⌃", modifier: true };
        case "alt":
        case "option":
        case "opt":
        case "⌥":
            return { id: "alt", glyph: "⌥", modifier: true };
        case "shift":
        case "⇧":
            return { id: "shift", glyph: "⇧", modifier: true };
        case "fn":
        case "function":
            return { id: "fn", glyph: "fn", modifier: true };
        }
        return null;
    }

    function _keyFor(token) {
        var lower = token.toLowerCase();
        switch (lower) {
        case "delete":
        case "del":
        case "backspace":
        case "⌫":
            return { id: "delete", glyph: "⌫", modifier: false };
        case "return":
        case "enter":
        case "↵":
        case "⏎":
            return { id: "return", glyph: "↵", modifier: false };
        case "escape":
        case "esc":
        case "⎋":
            return { id: "escape", glyph: "⎋", modifier: false };
        case "tab":
        case "⇥":
            return { id: "tab", glyph: "⇥", modifier: false };
        case "space":
        case "spacebar":
        case "␣":
            return { id: "space", glyph: "␣", modifier: false };
        case "capslock":
        case "caps":
        case "⇪":
            return { id: "caps-lock", glyph: "⇪", modifier: false };
        case "pageup":
        case "pgup":
        case "⇞":
            return { id: "page-up", glyph: "⇞", modifier: false };
        case "pagedown":
        case "pgdn":
        case "⇟":
            return { id: "page-down", glyph: "⇟", modifier: false };
        case "up":
        case "arrowup":
        case "↑":
            return { id: "up", glyph: "↑", modifier: false };
        case "down":
        case "arrowdown":
        case "↓":
            return { id: "down", glyph: "↓", modifier: false };
        case "left":
        case "arrowleft":
        case "←":
            return { id: "left", glyph: "←", modifier: false };
        case "right":
        case "arrowright":
        case "→":
            return { id: "right", glyph: "→", modifier: false };
        }
        // A single printable character is its own key label. Letters are
        // uppercased; punctuation and digits are already the label.
        if (token.length === 1)
            return { id: token, glyph: token.toUpperCase(), modifier: false };
        // Function keys are the one multi-character key label we keep.
        if (/^f([1-9]|1[0-2])$/i.test(token))
            return { id: lower, glyph: lower.toUpperCase(), modifier: false };
        return null;
    }

    function _addModifier(list, modifier) {
        for (var i = 0; i < list.length; ++i) {
            if (list[i].id === modifier.id)
                return;
        }
        list.push(modifier);
    }

    function _finish(modifiers, key) {
        var order = { fn: 0, ctrl: 1, alt: 2, shift: 3, super: 4 };
        modifiers.sort(function (a, b) { return order[a.id] - order[b.id]; });
        if (key)
            modifiers.push(key);
        return modifiers;
    }

    function _parsePlus(text) {
        var parts = text.split("+");
        var modifiers = [];
        var key = null;
        for (var i = 0; i < parts.length; ++i) {
            var token = parts[i].trim();
            if (token.length === 0)
                return [];
            var modifier = root._modifierFor(token);
            if (modifier) {
                root._addModifier(modifiers, modifier);
                continue;
            }
            var keyValue = root._keyFor(token);
            if (!keyValue || key)
                return [];
            key = keyValue;
        }
        return root._finish(modifiers, key);
    }

    function _parseCluster(text) {
        var modifiers = [];
        var rest = "";
        for (var i = 0; i < text.length; ++i) {
            var ch = text.charAt(i);
            var modifier = root._modifierFor(ch);
            if (modifier)
                root._addModifier(modifiers, modifier);
            else
                rest += ch;
        }
        if (rest.length === 0)
            return root._finish(modifiers, null);
        var key = root._keyFor(rest);
        if (!key)
            return [];
        return root._finish(modifiers, key);
    }

    function parse(text) {
        var raw = (text || "").trim();
        if (raw.length === 0)
            return [];
        // A chord is either plus-separated or a macOS-style glyph cluster
        // (`⇧⌘N`). A plain phrase with spaces is trailing metadata, not a
        // chord: the Dock's window rows use the same field for a Space name.
        if (raw.indexOf("+") >= 0)
            return root._parsePlus(raw);
        if (raw.indexOf(" ") >= 0)
            return [];
        var wholeModifier = root._modifierFor(raw);
        if (wholeModifier)
            return root._finish([wholeModifier], null);
        var wholeKey = root._keyFor(raw);
        if (wholeKey)
            return root._finish([], wholeKey);
        return root._parseCluster(raw);
    }

    visible: root.hasContent
    implicitWidth: root.hasShortcut ? keyRow.implicitWidth : plainText.implicitWidth
    implicitHeight: Math.max(keyRow.implicitHeight, plainText.implicitHeight)

    Row {
        id: keyRow
        visible: root.hasShortcut
        spacing: Theme.controls.shortcutHint.gap

        Repeater {
            model: root.keys
            delegate: Text {
                required property var modelData
                text: modelData.glyph
                textFormat: Text.PlainText
                color: modelData.modifier ? root.modifierColor : root.keyColor
                font.pixelSize: Theme.controls.shortcutHint.fontSize
                font.weight: modelData.modifier
                             ? Theme.controls.shortcutHint.modifierWeight
                             : Theme.controls.shortcutHint.keyWeight
            }
        }
    }

    Text {
        id: plainText
        visible: root.hasContent && !root.hasShortcut
        text: root.shortcut
        textFormat: Text.PlainText
        color: root.keyColor
        font.pixelSize: Theme.controls.shortcutHint.fontSize
        // A workspace name or other trailing metadata is not a key: keep it
        // regular so it does not read as a chord.
        font.weight: Theme.controls.shortcutHint.modifierWeight
    }

    Accessible.ignored: true
}
