// SPDX-License-Identifier: MIT
// Pure chrome scale geometry (T-21.1b).
//
// The chrome surface's *logical* size is the contract with the compositor:
// layer configure, reserved zones, panel rects, input regions, and pointer
// events all stay logical. This helper owns the other half only: the physical
// buffer size and the `wl_surface.set_buffer_scale` / `wp_viewport` pair used
// to commit it. No Wayland, QML, or QGuiApplication is involved, so the
// rounding rules are unit-testable on their own.
#pragma once

#include <QPointF>
#include <QRect>
#include <QSize>
#include <QtGlobal>

namespace ChromeScale {

// The scale a buffer is rendered at. Non-finite or non-positive values fall
// back to 1.0 so a bad preferred-scale event can never poison the buffer
// arithmetic.
qreal sanitize(qreal scale);

// Physical buffer extent: ceil(logical * scale), with a small epsilon so an
// exact integer product (1920 * 1.0) never rounds up to 1921.
QSize bufferSize(const QSize &logical, qreal scale);

// A logical rect mapped into physical buffer coordinates. The origin floors
// and the far edge ceils, so the crop covers every pixel the QML item paints.
QRect bufferRect(const QRect &logical, qreal scale);

// Continuous pointer coordinates: a logical surface coordinate multiplied by
// the scale (and the inverse). A round trip is exact within floating-point
// tolerance; pointer hit-testing keeps its logical contract.
QPointF logicalToBuffer(const QPointF &logical, qreal scale);
QPointF bufferToLogical(const QPointF &buffer, qreal scale);

struct Geometry {
    QSize logical;
    QSize buffer;
    // 1 for fractional scales (the viewport does the mapping), the integer
    // scale for 1x/2x/other exact integers.
    int bufferScale = 1;
    bool useViewport = false;
    // In buffer coordinates for the viewport source; in logical surface
    // coordinates for the destination.
    QSize viewportSource;
    QSize viewportDestination;
    qreal scale = 1.0;
};

// True when `scale` is an exact integer within the protocol's fixed-point
// resolution (1/256): 1.0, 2.0, 3.0. A fractional scale such as 1.5 uses the
// viewport path instead.
bool isExactInteger(qreal scale);

// The one mapping used by every chrome surface: 1x and exact integers use
// `wl_surface.set_buffer_scale`; everything else commits a physical-size
// buffer and maps it onto the logical surface with `wp_viewport`.
Geometry geometry(const QSize &logical, qreal scale);

} // namespace ChromeScale
