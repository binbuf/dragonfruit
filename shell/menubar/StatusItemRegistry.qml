// SPDX-License-Identifier: MIT
import QtQuick

// One registry for every menu-bar extra (T231).
//
// The shell controller supplies raw entries; this object normalizes them into
// the documented status-item shape and owns the two decisions the row must not
// scatter across delegates:
//
//   * order — system items sort by `priority` (lower is closer to the menu
//     side), third-party StatusNotifier items always sort after every system
//     item, then by priority and registration order;
//   * overflow — when the row has a finite `maxVisible`, third-party items
//     leave the row first, then optional system items. A system item is never
//     pushed out while a third-party item is still visible (T231/T233).
//
// An item is rendered by `StatusItem`; the row delegate still receives the
// `available` flag so an absent daemon keeps its stable slot index (hidden with
// zero width) instead of shifting every later item.
QtObject {
    id: registry

    // Raw entries from the shell controller. Shape:
    //   { id, icon, iconSource, label, tooltip, menu, priority, visible,
    //     available, enabled, selected, state, accessibleName, level, kind,
    //     essential, thirdParty }
    property var items: []
    // -1 keeps every item in the row. A non-negative value is the slot budget;
    // T233 will feed this from the measured bar width.
    property int maxVisible: -1
    // Priorities at or above this value are third-party items even when the
    // producer forgot the explicit `thirdParty` flag.
    property int thirdPartyPriority: 1000

    // The overflow entry is synthesized, never supplied by a service. It sorts
    // after every real item and is always available.
    readonly property var overflowItem: ({
        id: "overflow",
        icon: "overflow",
        iconSource: "",
        label: "",
        tooltip: qsTr("More menu bar items"),
        menu: "overflow",
        priority: 100000,
        visible: true,
        available: true,
        enabled: true,
        selected: false,
        state: "ok",
        accessibleName: qsTr("More menu bar items"),
        level: 0.8,
        kind: "overflow",
        essential: true,
        thirdParty: false
    })

    function _normalize(raw, index) {
        var source = raw || {};
        var id = source.id !== undefined ? String(source.id) : "";
        var state = source.state !== undefined ? String(source.state) : "ok";
        var item = {
            id: id,
            icon: source.icon !== undefined ? String(source.icon) : "",
            iconSource: source.iconSource !== undefined ? source.iconSource : "",
            label: source.label !== undefined ? String(source.label) : "",
            tooltip: source.tooltip !== undefined ? String(source.tooltip)
                                                    : (source.accessibleName !== undefined
                                                       ? String(source.accessibleName) : id),
            menu: source.menu !== undefined ? String(source.menu) : "",
            priority: source.priority !== undefined ? Number(source.priority) : 0,
            visible: source.visible !== false,
            // A user-hidden item and an absent service both render as a
            // hidden zero-width slot; the difference is retained in `state`.
            available: source.available !== false && source.visible !== false
                       && state !== "unavailable",
            enabled: source.enabled !== false,
            selected: source.selected === true || state === "attention",
            state: state,
            accessibleName: source.accessibleName !== undefined
                            ? String(source.accessibleName) : "",
            level: source.level !== undefined ? Number(source.level) : 0.8,
            kind: source.kind !== undefined ? String(source.kind) : "status",
            essential: source.essential === true,
            thirdParty: source.thirdParty === true
                        || (source.priority !== undefined
                            && Number(source.priority) >= registry.thirdPartyPriority),
            _index: index
        };
        if (item.accessibleName.length === 0)
            item.accessibleName = item.tooltip.length > 0 ? item.tooltip : item.id;
        if (item.tooltip.length === 0)
            item.tooltip = item.accessibleName;
        return item;
    }

    // System first (stable priority order), third-party last. The comparison
    // is explicit so a later task can add a second system family without
    // re-deriving the third-party rule.
    function _compare(a, b) {
        if (a.thirdParty !== b.thirdParty)
            return a.thirdParty ? 1 : -1;
        if (a.priority !== b.priority)
            return a.priority - b.priority;
        return a._index - b._index;
    }

    readonly property var orderedItems: {
        var list = [];
        for (var i = 0; i < items.length; ++i)
            list.push(_normalize(items[i], i));
        list.sort(_compare);
        return list;
    }

    function _split() {
        var ordered = orderedItems;
        // A hidden/absent slot draws zero width, so it never consumes the
        // budget and never appears in the overflow list. It stays in the row
        // to keep later indices stable.
        var visibleCount = 0;
        for (var v = 0; v < ordered.length; ++v) {
            if (ordered[v].available)
                visibleCount++;
        }
        if (maxVisible < 0 || visibleCount <= maxVisible)
            return { row: ordered, overflow: [] };

        var row = ordered.slice();
        var overflow = [];
        // Third-party items leave first, from the end of their block.
        for (var i = row.length - 1; i >= 0 && visibleCount > maxVisible; --i) {
            if (row[i].thirdParty && row[i].available) {
                overflow.unshift(row[i]);
                row.splice(i, 1);
                visibleCount--;
            }
        }
        // Only then do optional system items leave. `essential` items
        // (clock, Quick Settings, the overflow affordance itself) stay put
        // while there is room.
        for (var j = row.length - 1; j >= 0 && visibleCount > maxVisible; --j) {
            if (!row[j].essential && row[j].available) {
                overflow.unshift(row[j]);
                row.splice(j, 1);
                visibleCount--;
            }
        }
        // T233 hard notch/width fallback: when even the essential system
        // items cannot fit, the lowest-priority ones join the overflow list.
        // The row never clips, and the synthesized overflow affordance still
        // makes every removed item reachable. A budget of zero therefore
        // renders only the overflow affordance.
        for (var k = row.length - 1; k >= 0 && visibleCount > maxVisible; --k) {
            if (row[k].available) {
                overflow.unshift(row[k]);
                row.splice(k, 1);
                visibleCount--;
            }
        }
        // Re-sort the overflow block so items removed by the two passes keep
        // the registry's deterministic system-first order.
        overflow.sort(_compare);
        return { row: row, overflow: overflow };
    }

    readonly property var _splitResult: _split()
    readonly property var rowItems: _splitResult.row
    readonly property var overflowItems: _splitResult.overflow
    readonly property bool hasOverflow: overflowItems.length > 0

    // What the status-row Repeater actually renders: the row plus the
    // synthesized overflow affordance when anything was pushed out.
    readonly property var statusRowItems: {
        if (!hasOverflow)
            return rowItems;
        var out = rowItems.slice();
        out.push(overflowItem);
        return out;
    }

    function itemById(id) {
        var ordered = orderedItems;
        for (var i = 0; i < ordered.length; ++i) {
            if (ordered[i].id === id)
                return ordered[i];
        }
        if (id === overflowItem.id)
            return overflowItem;
        return null;
    }
}
