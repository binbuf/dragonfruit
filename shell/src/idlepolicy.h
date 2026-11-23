// SPDX-License-Identifier: MIT
// The shell's view of the settingsd idle/power keys (T317). Pure: it maps
// `idle.dim`, `idle.blank`, `idle.lock`, `idle.suspend`, and
// `power.preventSleep` onto the compositor's `set_idle_policy` request and
// owns the stage-to-effect transitions. No Wayland, no QML, no bus — the
// stage rules and policy updates are unit-testable in isolation.
#pragma once

#include <QString>
#include <QVariantMap>

#include <functional>

// The four delays are whole seconds; 0 disables a stage. The defaults mirror
// the settings schema so a shell without settingsd still forwards a coherent
// policy.
struct IdleSettings {
    int dim = 150;
    int blank = 300;
    int lock = 600;
    int suspend = 0;
    bool preventSleep = false;

    bool operator==(const IdleSettings &) const = default;
};

// Read the idle/power keys out of a settings value map. Missing keys fall back
// to the schema defaults; every delay is clamped to 0..86400.
IdleSettings idleSettingsFromValues(const QVariantMap &values);

// The policy the compositor receives. `power.preventSleep` disables the
// suspend stage (0), so the compositor never arms it; the shell also ignores
// an `idle-suspend` that races a policy change.
struct IdleCompositorPolicy {
    int dim = 0;
    int blank = 0;
    int lock = 0;
    int suspend = 0;

    bool operator==(const IdleCompositorPolicy &) const = default;
};

IdleCompositorPolicy idleCompositorPolicy(const IdleSettings &settings);

// The dim stage's brightness fraction until T319 owns the panel backlight.
// The shell also draws a full-output dim overlay so the stage is visible in
// nested/headless sessions where the brightness value is state-only.
inline constexpr double kIdleDimFactor = 0.5;

// The shell-owned effects of one idle stage. The controller never touches a
// protocol or a bus itself; the shell binds these to `ShellProtocol`,
// `lockScreen()`, and logind. Tests bind recording mocks.
struct IdleEffects {
    // Forward the normalized policy to the compositor (`set_idle_policy`).
    std::function<void(const IdleCompositorPolicy &)> setPolicy;
    // Dim/undim: the overlay plus the pre-dim brightness factor.
    std::function<void(bool)> setDimmed;
    // The compositor's current DPMS state (used to restore it on wake).
    std::function<bool()> dpmsOn;
    // Blank/wake every output (`df_output.set_dpms`).
    std::function<void(bool)> setDpms;
    // The existing fail-secure lock path (`lockScreen()`).
    std::function<void()> lock;
    // The existing logind suspend path.
    std::function<void()> suspend;
};

// Turns the settings snapshot into one compositor policy and the compositor's
// idle actions into effects. One instance per shell; the compositor owns the
// timer and inhibitor suppression, so the controller only reacts to actions it
// actually receives.
class IdleController
{
public:
    explicit IdleController(IdleEffects effects);

    // Apply a settings snapshot. Forwards the compositor policy on the first
    // call and whenever the effective policy changes; an unchanged snapshot is
    // a no-op.
    void setSettings(const IdleSettings &settings);

    // One compositor idle action (`idle-dim`, `idle-blank`, `idle-lock`,
    // `idle-suspend`, `idle-awake`). Unknown actions and disabled stages are
    // ignored. Repeated stages are idempotent.
    void handleAction(const QString &action);

    const IdleSettings &settings() const { return m_settings; }
    const IdleCompositorPolicy &policy() const { return m_policy; }
    bool policySent() const { return m_policySent; }
    bool dimmed() const { return m_dimmed; }
    bool blanked() const { return m_blanked; }

private:
    IdleEffects m_effects;
    IdleSettings m_settings;
    IdleCompositorPolicy m_policy;
    bool m_policySent = false;
    bool m_dimmed = false;
    bool m_blanked = false;
    // The DPMS state before the blank stage, restored on wake. True (on) is
    // the normal pre-blank state; a screen already off must stay off.
    bool m_preBlankDpmsOn = true;
};
