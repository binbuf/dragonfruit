// SPDX-License-Identifier: MIT
// The on-screen-display model (T-11.4a).
//
// A volume or brightness change presents a brief, non-interactive overlay on
// the active output. The model owns only that presentation state -- what is
// shown, for how long, and its fade -- so the "OSD appears within one frame of
// the trigger and auto-dismisses" contract (legacy FR-5) is unit-testable with
// no QML, Wayland, or timer:
//
//   * `present` (re)arms the display for a value change; the last change wins,
//     so simultaneous volume+brightness triggers coalesce.
//   * `tick` is the auto-dismiss; it returns false once the deadline passes.
//   * `setFullscreen` suppresses the OSD (legacy OSD policy: brief overlays are
//     hidden while a fullscreen surface owns the output) and hides a visible
//     one immediately.
//   * `fade` is the animation curve; the reduced-motion path is exactly 1.0,
//     so the state change is legible but not interpolated (FR-5).
//
// The shell controller drives the fade from this model each frame and commits
// the QML scene; the model is deliberately not a QObject and holds no timer.
#pragma once

#include <QString>
#include <QtGlobal>

class OsdModel
{
public:
    enum class Kind { None, Volume, Brightness };

    // How long the OSD stays visible in total, and the fade-in/out ramps
    // inside it. The reduced-motion path ignores the ramps (see `fade`).
    static constexpr qint64 kDismissMs = 1400;
    static constexpr qint64 kFadeInMs = 120;
    static constexpr qint64 kFadeOutMs = 220;

    // Present a change at monotonic-ish `nowMs` (ms since epoch). Returns true
    // when the OSD is now visible; false when a fullscreen surface suppressed
    // it. The last-presented value wins (concurrent triggers coalesce).
    bool present(Kind kind, double value, bool muted, qint64 nowMs);
    bool presentVolume(double value, bool muted, qint64 nowMs);
    bool presentBrightness(double value, qint64 nowMs);

    // Auto-dismiss: hide once `nowMs` reaches the deadline. Returns `visible`.
    bool tick(qint64 nowMs);

    // Hide without waiting for the deadline (a fullscreen entry, restart).
    void hide();

    // Update the fullscreen suppression policy. A visible OSD hides
    // immediately when the output becomes fullscreen; a later `present` is
    // refused while it stays fullscreen.
    void setFullscreen(bool fullscreen);
    bool fullscreen() const { return m_fullscreen; }

    bool visible() const { return m_visible; }
    Kind kind() const { return m_kind; }
    QString kindName() const;
    double value() const { return m_value; }
    bool muted() const { return m_muted; }
    qint64 shownAt() const { return m_shownAt; }
    qint64 deadline() const { return m_deadline; }

    // The fade progress [0, 1] at `nowMs`: ramp in, hold, ramp out. Reduced
    // motion jumps straight to 1.0 (no interpolation) so the state is still
    // legible without motion.
    double fade(qint64 nowMs, bool reducedMotion) const;

private:
    Kind m_kind = Kind::None;
    double m_value = 0.0;
    bool m_muted = false;
    bool m_visible = false;
    bool m_fullscreen = false;
    qint64 m_shownAt = 0;
    qint64 m_deadline = 0;
};