// SPDX-License-Identifier: MIT
// Pure dmabuf frame validation and single-buffer commit state (T-21.1d).
//
// This is the dependency-free half of the GPU chrome transport: it validates
// an exported dmabuf frame against the compositor's linux-dmabuf feedback and
// owns the busy/reuse rules for one reusable buffer. No Wayland, EGL, or Qt
// GUI state is involved, so the format/commit rules are unit-tested headlessly.
#pragma once

#include <QSize>
#include <QString>
#include <QVector>

#include <cstdint>

#include "gpuchromecapabilities.h"

// One exported dmabuf plane. `fd` is owned by the caller; a commit path dups
// it before handing it to `zwp_linux_buffer_params_v1`.
struct ChromeDmabufFrame {
    int fd = -1;
    QSize size;
    uint32_t format = 0;
    uint64_t modifier = 0;
    uint32_t stride = 0;
    uint32_t offset = 0;
    // EGL exports a GL texture with its origin at the bottom-left, so the
    // dmabuf rows are y-inverted relative to the top-left compositor
    // convention. The linux-dmabuf `y_invert` flag tells the compositor to
    // flip it when sampling.
    bool yInverted = false;
    // Monotonic identity of the producer-side export. A new FBO export gets a
    // new serial even when its size/format/modifier happen to match the old
    // one, so the shell recreates its wl_buffer instead of reusing a buffer
    // that points at deleted storage.
    uint64_t serial = 0;

    bool valid() const
    {
        return fd >= 0 && !size.isEmpty() && stride > 0;
    }
};

struct ChromeDmabufCheck {
    bool valid = false;
    // Empty when valid; otherwise a short fallback token (`no-format`,
    // `no-modifier`, `bad-export`, ...).
    QString reason;
};

namespace ChromeDmabuf {

// Validate an exported frame against the advertised format/modifier table.
// The compositor's `dmabuf_imported` accepts only an exact `(format, modifier)`
// pair from the feedback table, so an implicit or unlisted modifier is a
// fallback, not something to paper over.
ChromeDmabufCheck validate(
    const ChromeDmabufFrame &frame,
    const QVector<GpuChromeCapabilities::FormatModifier> &advertised);

// `zwp_linux_buffer_params_v1.add` takes the 64-bit modifier as two uint32s.
uint32_t modifierHigh(uint64_t modifier);
uint32_t modifierLow(uint64_t modifier);

} // namespace ChromeDmabuf

// The reuse state of one dmabuf-backed surface buffer. T186b commits a single
// buffer: it may be rendered into only while it is not held by the compositor.
// The ring and release-driven pooling are T186c.
class ChromeDmabufSlot
{
public:
    enum class State {
        Empty,   // no buffer has been created yet
        Ready,   // a buffer exists and is free to render into
        Busy,    // attached and not yet released by the compositor
        Failed,  // creation/import failed; the surface must fall back
    };

    State state() const { return m_state; }
    bool canRender() const { return m_state == State::Empty || m_state == State::Ready; }
    bool busy() const { return m_state == State::Busy; }
    bool failed() const { return m_state == State::Failed; }
    QString failureReason() const { return m_reason; }

    // A render/commit begins. False when the buffer is busy or failed.
    bool beginCommit();
    // The compositor returned the buffer through `wl_buffer.release`.
    void release();
    // Creation or import failed; the surface falls back to T185 shm.
    void fail(const QString &reason);
    void reset();

private:
    State m_state = State::Empty;
    QString m_reason;
};
