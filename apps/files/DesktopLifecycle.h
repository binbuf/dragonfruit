// SPDX-License-Identifier: MIT
// The Files desktop's process lifecycle (T339): one background layer, and a
// fatal compositor loss that exits instead of spinning.
//
// Both invariants live here so the process and its regression test run the
// same code:
//
//  * `createSurfaceOnce()` gates the desktop's `background` layer surface.
//    The process creates exactly one (ADR 0169); a second request is a no-op
//    instead of stacking another layer.
//  * `exitOnCompositorLoss()` disables the dispatch `QSocketNotifier` before
//    exiting. A dead compositor leaves its fd readable, so an enabled notifier
//    would wake continuously at 100% CPU. The queued non-zero exit lets
//    systemd's `Restart=always` start a fresh process against the compositor's
//    reissued launch token.
#pragma once

#include <functional>
#include <utility>

#include <QCoreApplication>
#include <QSocketNotifier>
#include <QTimer>

class DesktopLifecycle
{
public:
    // The exit is injectable so a unit test can observe the request without
    // tearing down the test's own event loop. The default is the process exit.
    using ExitFunction = std::function<void(int)>;

    explicit DesktopLifecycle(ExitFunction exitFunction = ExitFunction())
        : m_exitFunction(std::move(exitFunction))
    {
        if (!m_exitFunction)
            m_exitFunction = [](int code) { QCoreApplication::exit(code); };
    }

    // Runs `create` only for the first request. A later request returns true
    // without creating anything, so the desktop cannot stack a second
    // `background` layer. `surfaceCount()` counts the successful creations.
    template <typename Create>
    bool createSurfaceOnce(Create create)
    {
        if (m_surfaceCount != 0)
            return true;
        if (!create())
            return false;
        ++m_surfaceCount;
        return true;
    }

    int surfaceCount() const { return m_surfaceCount; }

    bool isExiting() const { return m_exiting; }

    void exitOnCompositorLoss(QSocketNotifier *notifier)
    {
        // Disable first: the readable-but-dead fd must not re-enter this
        // handler while the exit is queued.
        if (notifier)
            notifier->setEnabled(false);
        if (m_exiting)
            return;
        m_exiting = true;
        const ExitFunction exit = m_exitFunction;
        QTimer::singleShot(0, [exit]() { exit(1); });
    }

private:
    ExitFunction m_exitFunction;
    int m_surfaceCount = 0;
    bool m_exiting = false;
};
