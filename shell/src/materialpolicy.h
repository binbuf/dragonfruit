// SPDX-License-Identifier: MIT
// The shell's one application policy for the compositor's role-keyed material
// state (T-21.2c). The compositor owns tier selection and the persistent
// floor; this class is the only place the shell turns that state into the
// properties the Dock and menu bar draw. It never runs the degrade ladder or
// invents token values.
#pragma once

#include <cstdint>

// The effective state a chrome surface applies. Wire values are fixed by
// `df_output.material_role_state`: tier 0=full/1=reduced/2=minimal and
// fallback 0=none/1=no-sampled-blur/2=software/3=multi-gpu/4=floor-missed.
struct AppliedMaterial {
    std::uint32_t tier = 0;
    bool blurActive = true;
    std::uint32_t fallback = 0;

    bool operator==(const AppliedMaterial &) const = default;
};

class MaterialPolicy
{
public:
    enum Role : std::uint32_t {
        Chrome = 0,
        Dock = 1,
        Popup = 2,
        Drawer = 3,
        RoleCount = 4,
    };

    enum Fallback : std::uint32_t {
        NoFallback = 0,
        NoSampledBlur = 1,
        Software = 2,
        MultiGpu = 3,
        FloorMissed = 4,
    };

    // The compositor's persistent/transient split. The shell uses it only for
    // the defensive floor guard below, never to re-run the ladder.
    static bool isPersistent(std::uint32_t role);

    // The legacy v3 `material_state` event. It is the compatibility path for a
    // compositor without the role event; a role that has received a v4 state
    // keeps that state.
    void setGlobal(std::uint32_t tier, bool blurActive, std::uint32_t fallback);

    // The v4 `material_role_state` event.
    void setRole(std::uint32_t role, std::uint32_t tier, bool blurActive,
                 std::uint32_t fallback);

    // The state a surface in `role` applies. Unknown roles fall back to the
    // legacy global snapshot; a known role always wins.
    AppliedMaterial applied(std::uint32_t role) const;

    bool roleKnown(std::uint32_t role) const;

private:
    struct State {
        std::uint32_t tier = 0;
        bool blurActive = true;
        std::uint32_t fallback = 0;
    };

    State m_global;
    State m_roles[RoleCount];
    bool m_roleKnown[RoleCount] = {false, false, false, false};
};
