// SPDX-License-Identifier: MIT
import QtQuick

// The shared keyboard map (T267). It is a pure table, not a second key
// handler: each component asks this singleton which action a key means in its
// scope, so the documented map and the implemented map cannot drift apart.
//
// `modifiers` are the exact required modifier set for a binding. A binding
// with `Qt.NoModifier` therefore does not also fire for Ctrl+Down, while a
// binding with `Qt.ControlModifier` accepts Ctrl+Shift+F (the extra modifier
// is a caller refinement, not a different command).
pragma Singleton
QtObject {
    id: root

    readonly property var bindings: [
        // Application shell. These work from any focus owner because the
        // shell handles them after the focused item declines them.
        { scope: "shell", key: Qt.Key_F, modifiers: Qt.ControlModifier,
          action: "focus-search", label: qsTr("Focus search") },
        { scope: "shell", key: Qt.Key_Left, modifiers: Qt.AltModifier,
          action: "back", label: qsTr("Back") },
        { scope: "shell", key: Qt.Key_Right, modifiers: Qt.AltModifier,
          action: "forward", label: qsTr("Forward") },
        { scope: "shell", key: Qt.Key_Escape, modifiers: Qt.NoModifier,
          action: "dismiss", label: qsTr("Clear search or close the sub-pane") },

        // Sidebar list. Headers are skipped by the component.
        { scope: "sidebar", key: Qt.Key_Down, modifiers: Qt.NoModifier,
          action: "next", label: qsTr("Next item") },
        { scope: "sidebar", key: Qt.Key_Up, modifiers: Qt.NoModifier,
          action: "previous", label: qsTr("Previous item") },
        { scope: "sidebar", key: Qt.Key_Home, modifiers: Qt.NoModifier,
          action: "first", label: qsTr("First item") },
        { scope: "sidebar", key: Qt.Key_End, modifiers: Qt.NoModifier,
          action: "last", label: qsTr("Last item") },
        { scope: "sidebar", key: Qt.Key_Return, modifiers: Qt.NoModifier,
          action: "activate", label: qsTr("Open the selected item") },
        { scope: "sidebar", key: Qt.Key_Enter, modifiers: Qt.NoModifier,
          action: "activate", label: qsTr("Open the selected item") },
        { scope: "sidebar", key: Qt.Key_Space, modifiers: Qt.NoModifier,
          action: "activate", label: qsTr("Open the selected item") },

        // A ranked list of results.
        { scope: "list", key: Qt.Key_Down, modifiers: Qt.NoModifier,
          action: "next", label: qsTr("Next item") },
        { scope: "list", key: Qt.Key_Up, modifiers: Qt.NoModifier,
          action: "previous", label: qsTr("Previous item") },
        { scope: "list", key: Qt.Key_Home, modifiers: Qt.NoModifier,
          action: "first", label: qsTr("First item") },
        { scope: "list", key: Qt.Key_End, modifiers: Qt.NoModifier,
          action: "last", label: qsTr("Last item") },
        { scope: "list", key: Qt.Key_Return, modifiers: Qt.NoModifier,
          action: "activate", label: qsTr("Open the selected item") },
        { scope: "list", key: Qt.Key_Enter, modifiers: Qt.NoModifier,
          action: "activate", label: qsTr("Open the selected item") },
        { scope: "list", key: Qt.Key_Escape, modifiers: Qt.NoModifier,
          action: "dismiss", label: qsTr("Dismiss the list") },

        // Search field. Down/Up hand focus to the result list; Escape clears
        // the field and then returns focus to the sidebar.
        { scope: "search-field", key: Qt.Key_Down, modifiers: Qt.NoModifier,
          action: "enter-list", label: qsTr("Move into the list") },
        { scope: "search-field", key: Qt.Key_Up, modifiers: Qt.NoModifier,
          action: "enter-list", label: qsTr("Move into the list") },
        { scope: "search-field", key: Qt.Key_Escape, modifiers: Qt.NoModifier,
          action: "dismiss", label: qsTr("Clear the search") },

        // Standard traversal. Qt owns the actual focus chain; these rows keep
        // the map complete for documentation and accessibility help text.
        { scope: "any", key: Qt.Key_Tab, modifiers: Qt.NoModifier,
          action: "next-focus", label: qsTr("Next control") },
        { scope: "any", key: Qt.Key_Tab, modifiers: Qt.ShiftModifier,
          action: "previous-focus", label: qsTr("Previous control") },
        { scope: "any", key: Qt.Key_Backtab, modifiers: Qt.ShiftModifier,
          action: "previous-focus", label: qsTr("Previous control") }
    ]

    // The action for one key in one scope, or an empty string. "any" bindings
    // are consulted last so a scope-specific meaning always wins.
    function actionFor(scope, key, modifiers) {
        var exact = root.modifiersOf(modifiers);
        var scoped = root.find(scope, key, exact);
        if (scoped.length > 0)
            return scoped;
        return root.find("any", key, exact);
    }

    function find(scope, key, exact) {
        for (var i = 0; i < root.bindings.length; ++i) {
            var binding = root.bindings[i];
            if (binding.scope !== scope || binding.key !== key)
                continue;
            var required = root.modifiersOf(binding.modifiers);
            if (required.length === 0) {
                if (exact.length === 0)
                    return binding.action;
                continue;
            }
            if (exact.length === 0)
                continue;
            var matched = true;
            for (var m = 0; m < required.length; ++m) {
                if (exact.indexOf(required[m]) < 0) {
                    matched = false;
                    break;
                }
            }
            if (matched)
                return binding.action;
        }
        return "";
    }

    // The modifier names present in a Qt modifier flag, in a stable order.
    function modifiersOf(flags) {
        var out = [];
        if (flags & Qt.ShiftModifier)
            out.push("shift");
        if (flags & Qt.ControlModifier)
            out.push("control");
        if (flags & Qt.AltModifier)
            out.push("alt");
        if (flags & Qt.MetaModifier)
            out.push("meta");
        return out;
    }

    // A stable, human-readable map for tests, help text, and documentation.
    // One row per binding, in table order.
    readonly property var rows: root.bindings.map(function(binding) {
        var parts = [];
        if (binding.modifiers & Qt.ControlModifier)
            parts.push("Ctrl");
        if (binding.modifiers & Qt.AltModifier)
            parts.push("Alt");
        if (binding.modifiers & Qt.ShiftModifier)
            parts.push("Shift");
        if (binding.modifiers & Qt.MetaModifier)
            parts.push("Meta");
        parts.push(root.keyName(binding.key));
        return { scope: binding.scope, keys: parts.join("+"),
                 action: binding.action, label: binding.label };
    })

    function keyName(key) {
        switch (key) {
        case Qt.Key_F: return "F";
        case Qt.Key_Left: return "Left";
        case Qt.Key_Right: return "Right";
        case Qt.Key_Up: return "Up";
        case Qt.Key_Down: return "Down";
        case Qt.Key_Home: return "Home";
        case Qt.Key_End: return "End";
        case Qt.Key_Return: return "Return";
        case Qt.Key_Enter: return "Enter";
        case Qt.Key_Space: return "Space";
        case Qt.Key_Escape: return "Escape";
        case Qt.Key_Tab: return "Tab";
        case Qt.Key_Backtab: return "Backtab";
        default: return String(key);
        }
    }
}
