// SPDX-License-Identifier: MIT
// Scene-graph frame-commit gate (T-10 FR-14).
//
// The shell commits chrome frames from `QQuickWindow::afterRendering` so a
// continuously animating scene reaches the compositor without a sampling
// timer. Reading a rendered frame back (`grabWindow`) renders the scene a
// second time and therefore emits `afterRendering` again; this gate
// suppresses that re-entrant schedule so the commit path cannot loop while
// still counting the scene-graph frames the window produced.
#ifndef DRAGONFRUIT_FRAMECOMMITGATE_H
#define DRAGONFRUIT_FRAMECOMMITGATE_H

#include <QtGlobal>

class FrameCommitGate
{
public:
    // A scene-graph frame finished. Returns true when the shell should
    // schedule a commit for it; false for the re-render caused by an
    // in-flight commit.
    bool frameRendered()
    {
        ++m_sceneFrames;
        return !m_committing;
    }

    // Bracket the `grabWindow()` readback. `grabWindow()` renders the scene
    // again, so the nested `frameRendered()` must not schedule another
    // commit.
    void beginCommit() { m_committing = true; }
    void endCommit()
    {
        m_committing = false;
        ++m_committedFrames;
    }

    bool committing() const { return m_committing; }
    quint64 sceneFrames() const { return m_sceneFrames; }
    quint64 committedFrames() const { return m_committedFrames; }

    void reset()
    {
        m_committing = false;
        m_sceneFrames = 0;
        m_committedFrames = 0;
    }

private:
    bool m_committing = false;
    quint64 m_sceneFrames = 0;
    quint64 m_committedFrames = 0;
};

#endif // DRAGONFRUIT_FRAMECOMMITGATE_H
