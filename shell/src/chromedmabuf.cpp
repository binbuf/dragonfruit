// SPDX-License-Identifier: MIT
#include "chromedmabuf.h"

namespace ChromeDmabuf {

ChromeDmabufCheck validate(
    const ChromeDmabufFrame &frame,
    const QVector<GpuChromeCapabilities::FormatModifier> &advertised)
{
    ChromeDmabufCheck check;
    if (!frame.valid()) {
        check.reason = QStringLiteral("bad-export");
        return check;
    }
    constexpr int kMaxDimension = 16384;
    if (frame.size.width() > kMaxDimension || frame.size.height() > kMaxDimension) {
        check.reason = QStringLiteral("oversized");
        return check;
    }

    bool formatSeen = false;
    for (const GpuChromeCapabilities::FormatModifier &pair : advertised) {
        if (pair.format != frame.format)
            continue;
        formatSeen = true;
        if (pair.modifier == frame.modifier) {
            check.valid = true;
            return check;
        }
    }
    check.reason = formatSeen ? QStringLiteral("no-modifier")
                              : QStringLiteral("no-format");
    return check;
}

uint32_t modifierHigh(uint64_t modifier)
{
    return static_cast<uint32_t>(modifier >> 32);
}

uint32_t modifierLow(uint64_t modifier)
{
    return static_cast<uint32_t>(modifier & 0xffffffffu);
}

} // namespace ChromeDmabuf

bool ChromeDmabufSlot::beginCommit()
{
    if (m_state == State::Empty || m_state == State::Ready) {
        m_state = State::Busy;
        return true;
    }
    return false;
}

void ChromeDmabufSlot::release()
{
    if (m_state == State::Busy)
        m_state = State::Ready;
}

void ChromeDmabufSlot::fail(const QString &reason)
{
    m_state = State::Failed;
    m_reason = reason;
}

void ChromeDmabufSlot::reset()
{
    m_state = State::Empty;
    m_reason.clear();
}
