// SPDX-License-Identifier: MIT
#include "materialpolicy.h"

bool MaterialPolicy::isPersistent(std::uint32_t role)
{
    return role == Chrome || role == Dock;
}

void MaterialPolicy::setGlobal(std::uint32_t tier, bool blurActive,
                               std::uint32_t fallback)
{
    m_global = State{tier, blurActive, fallback};
}

void MaterialPolicy::setRole(std::uint32_t role, std::uint32_t tier,
                             bool blurActive, std::uint32_t fallback)
{
    if (role >= RoleCount)
        return;
    m_roles[role] = State{tier, blurActive, fallback};
    m_roleKnown[role] = true;
}

bool MaterialPolicy::roleKnown(std::uint32_t role) const
{
    return role < RoleCount && m_roleKnown[role];
}

AppliedMaterial MaterialPolicy::applied(std::uint32_t role) const
{
    const State &state = roleKnown(role) ? m_roles[role] : m_global;
    AppliedMaterial material{state.tier, state.blurActive, state.fallback};

    // Any explicit fallback reason means the sampled backdrop pass is not
    // running, so the surface must draw the token-driven milky complement even
    // if a producer accidentally set blur_active. This is the one place the
    // shell resolves "milky or glass"; QML only reads the result.
    if (material.fallback != NoFallback)
        material.blurActive = false;

    // Defensive floor guard. The compositor is authoritative, but a v3 global
    // snapshot or a malformed v4 state must never make persistent chrome a
    // flat Minimal slab while the sampled path is available. A persistent
    // role below Reduced with no fallback reason becomes the explicit
    // floor-missed milky fallback instead.
    if (isPersistent(role) && material.fallback == NoFallback
            && material.tier > 1) {
        material.tier = 1;
        material.blurActive = false;
        material.fallback = FloorMissed;
    }
    return material;
}
