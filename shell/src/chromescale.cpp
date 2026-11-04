// SPDX-License-Identifier: MIT
#include "chromescale.h"

#include <QtMath>

namespace {

// A logical extent times the scale, rounded up but treating an exact integer
// product as exact. `qCeil(1920.0)` is fine; the epsilon protects against a
// scale such as 1.0 represented with a tiny positive error.
int physicalExtent(int logical, qreal scale)
{
    const qreal scaled = qreal(logical) * scale;
    const qreal rounded = qRound(scaled);
    if (qAbs(scaled - rounded) < 1e-6)
        return rounded;
    return qCeil(scaled);
}

} // namespace

namespace ChromeScale {

qreal sanitize(qreal scale)
{
    if (!qIsFinite(scale) || scale <= 0.0)
        return 1.0;
    return scale;
}

QSize bufferSize(const QSize &logical, qreal scale)
{
    const qreal safe = sanitize(scale);
    if (logical.width() <= 0 || logical.height() <= 0)
        return QSize();
    return QSize(physicalExtent(logical.width(), safe),
                 physicalExtent(logical.height(), safe));
}

QRect bufferRect(const QRect &logical, qreal scale)
{
    const qreal safe = sanitize(scale);
    if (logical.width() <= 0 || logical.height() <= 0)
        return QRect();
    const int left = qFloor(qreal(logical.x()) * safe);
    const int top = qFloor(qreal(logical.y()) * safe);
    const int right = qCeil(qreal(logical.x() + logical.width()) * safe);
    const int bottom = qCeil(qreal(logical.y() + logical.height()) * safe);
    return QRect(left, top, right - left, bottom - top);
}

QPointF logicalToBuffer(const QPointF &logical, qreal scale)
{
    return logical * sanitize(scale);
}

QPointF bufferToLogical(const QPointF &buffer, qreal scale)
{
    return buffer / sanitize(scale);
}

bool isExactInteger(qreal scale)
{
    const qreal safe = sanitize(scale);
    // wl_fixed_t has 1/256 resolution; 1.5 is deliberately not an integer.
    return qAbs(safe - qRound(safe)) < (1.0 / 256.0);
}

Geometry geometry(const QSize &logical, qreal scale)
{
    Geometry result;
    result.scale = sanitize(scale);
    result.logical = logical;
    result.buffer = bufferSize(logical, result.scale);
    if (result.buffer.isEmpty())
        return result;

    if (isExactInteger(result.scale)) {
        result.bufferScale = qRound(result.scale);
        result.useViewport = false;
        return result;
    }

    // Fractional scale: the buffer is physical, the surface stays logical.
    // The viewport source is the whole buffer; the destination is the
    // logical surface. 1.5x thus commits 2880x188 and maps it to 1920x125.
    result.bufferScale = 1;
    result.useViewport = true;
    result.viewportSource = result.buffer;
    result.viewportDestination = logical;
    return result;
}

} // namespace ChromeScale
