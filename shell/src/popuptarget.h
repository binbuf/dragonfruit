// SPDX-License-Identifier: MIT
// Content-sized popup render targets (T238 popup-target work).
//
// A chrome scene used to carry its popup as a sub-rect of the strip's scene, so
// opening a narrow menu grew every attachment in the shared ring. This budget
// gives the popup its own target: the target is the popup content size clamped
// to the output, grows only while one interaction session is active, and stays
// warm for a short grace period after the session ends. That keeps a sweep
// between nearby popups on one allocation without holding the largest menu for
// the rest of the session. Pure and unit-tested (tst_dockcore).
#pragma once

#include <QSize>

#include <cstdint>

class PopupTargetBudget
{
public:
    // How long a closed popup keeps its target before it is released. Long
    // enough to cover the close animation and a retarget, short enough that an
    // idle desktop returns to the strip-only allocation.
    static constexpr qint64 kWarmRetentionMs = 750;

    // Advance the budget for one frame. `content` is the popup content size in
    // logical pixels (empty when nothing is shown); `output` bounds the result
    // and may be invalid; `sessionActive` keeps the target warm while the
    // pointer, a button, or the popup itself is still engaged.
    QSize update(const QSize &content, const QSize &output, bool sessionActive,
                 qint64 nowMs);

    QSize size() const { return m_size; }
    bool empty() const { return m_size.isEmpty(); }
    // True while the target is retained after the session ended.
    bool warm() const { return m_warm; }
    void reset();

private:
    QSize m_size;
    qint64 m_lastActiveMs = -1;
    bool m_warm = false;
};

// The single-frame size: content clamped to the output. Empty content yields
// an empty size. Exposed for the controller and tests.
QSize popupTargetSize(const QSize &content, const QSize &output);
