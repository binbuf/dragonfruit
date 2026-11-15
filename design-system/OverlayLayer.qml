// SPDX-License-Identifier: MIT
pragma Singleton
import QtQuick

// One registry of window overlay layers (T266).
//
// Every window that hosts popups declares a `WindowOverlay`; the overlay
// registers itself here. A `Select` popup, a `ContextMenu`, or a `Dialog`
// finds the layer for its own window without a pane-local `popupParent`
// opt-in, so a surface inside a clipped `ScrollView` is never cut off.
//
// The registry is a list rather than a map because a QML singleton cannot use
// a `QQuickWindow` as an object key. The last registered visible layer for a
// window wins, which is the topmost overlay in the declaration order.
QtObject {
    id: root

    property var layers: []

    function register(layer) {
        if (!layer)
            return;
        if (root.layers.indexOf(layer) >= 0)
            return;
        var next = root.layers.slice();
        next.push(layer);
        root.layers = next;
    }

    function unregister(layer) {
        var at = root.layers.indexOf(layer);
        if (at < 0)
            return;
        var next = root.layers.slice();
        next.splice(at, 1);
        root.layers = next;
    }

    function layerFor(item) {
        if (!item)
            return null;
        var hostWindow = item.Window.window;
        if (!hostWindow)
            return null;
        var best = null;
        for (var i = 0; i < root.layers.length; ++i) {
            var layer = root.layers[i];
            if (!layer || !layer.visible)
                continue;
            if (layer.Window.window !== hostWindow)
                continue;
            best = layer;
        }
        return best;
    }

    // The layer's own rectangle, in layer coordinates. A popup passes this as
    // its `availableRect` so it clamps and flips inside the window.
    function availableRect(item) {
        var layer = root.layerFor(item);
        if (!layer)
            return null;
        return { x: 0, y: 0, w: layer.width, h: layer.height };
    }
}
