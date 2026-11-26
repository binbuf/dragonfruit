// SPDX-License-Identifier: GPL-3.0-or-later
// T-21.2c: the shell's one application policy for the compositor's
// role-keyed material state. Pure: no Wayland, QML, or compositor.
#include <QtTest>

#include "materialpolicy.h"

class TestMaterialPolicy : public QObject
{
    Q_OBJECT

private slots:
    void defaultsAreTranslucentGlass();
    void roleStateWinsOverTheLegacySnapshot();
    void unknownRoleFallsBackToTheLegacySnapshot();
    void persistentRoleNeverSelectsBelowReduced();
    void drawerKeepsTheGlassFloor();
    void floorMissedUsesTheMilkyFallback();
    void explicitFallbackForcesTheMilkyFallback();
    void softwareKeepsMinimalButMilky();
    void transientRoleMayReachMinimal();
    void explicitFallbackWinsForTransientRoles();
    void loadSequenceKeepsTheAppliedProperties();
    void unknownRoleValuesAreIgnored();
};

void TestMaterialPolicy::defaultsAreTranslucentGlass()
{
    const MaterialPolicy policy;
    for (std::uint32_t role = 0; role < MaterialPolicy::RoleCount; ++role) {
        QVERIFY(!policy.roleKnown(role));
        const AppliedMaterial material = policy.applied(role);
        QCOMPARE(material.tier, std::uint32_t(0));
        QCOMPARE(material.blurActive, true);
        QCOMPARE(material.fallback, std::uint32_t(0));
    }
}

void TestMaterialPolicy::roleStateWinsOverTheLegacySnapshot()
{
    MaterialPolicy policy;
    policy.setGlobal(2, false, MaterialPolicy::Software);
    policy.setRole(MaterialPolicy::Chrome, 0, true, MaterialPolicy::NoFallback);
    policy.setRole(MaterialPolicy::Dock, 1, true, MaterialPolicy::NoFallback);

    QVERIFY(policy.roleKnown(MaterialPolicy::Chrome));
    QVERIFY(policy.roleKnown(MaterialPolicy::Dock));
    const AppliedMaterial chrome = policy.applied(MaterialPolicy::Chrome);
    QCOMPARE(chrome.tier, std::uint32_t(0));
    QCOMPARE(chrome.blurActive, true);
    QCOMPARE(chrome.fallback, std::uint32_t(0));
    const AppliedMaterial dock = policy.applied(MaterialPolicy::Dock);
    QCOMPARE(dock.tier, std::uint32_t(1));
    QCOMPARE(dock.blurActive, true);
    QCOMPARE(dock.fallback, std::uint32_t(0));

    // A later legacy event must not overwrite a role that has v4 state.
    policy.setGlobal(2, false, MaterialPolicy::Software);
    QCOMPARE(policy.applied(MaterialPolicy::Chrome).blurActive, true);
    QCOMPARE(policy.applied(MaterialPolicy::Dock).tier, std::uint32_t(1));
}

void TestMaterialPolicy::unknownRoleFallsBackToTheLegacySnapshot()
{
    MaterialPolicy policy;
    policy.setGlobal(1, true, MaterialPolicy::NoFallback);
    const AppliedMaterial material = policy.applied(MaterialPolicy::Chrome);
    QCOMPARE(material.tier, std::uint32_t(1));
    QCOMPARE(material.blurActive, true);
    QCOMPARE(material.fallback, std::uint32_t(0));

    policy.setGlobal(2, false, MaterialPolicy::Software);
    const AppliedMaterial software = policy.applied(MaterialPolicy::Chrome);
    QCOMPARE(software.tier, std::uint32_t(2));
    QCOMPARE(software.blurActive, false);
    QCOMPARE(software.fallback, std::uint32_t(MaterialPolicy::Software));
}

void TestMaterialPolicy::persistentRoleNeverSelectsBelowReduced()
{
    MaterialPolicy policy;
    // A malformed sampled state: Minimal with no fallback reason. The shell
    // refuses to draw a flat Minimal slab for persistent chrome.
    policy.setRole(MaterialPolicy::Chrome, 2, true, MaterialPolicy::NoFallback);
    policy.setRole(MaterialPolicy::Dock, 2, true, MaterialPolicy::NoFallback);

    for (std::uint32_t role : {std::uint32_t(MaterialPolicy::Chrome),
                               std::uint32_t(MaterialPolicy::Dock)}) {
        const AppliedMaterial material = policy.applied(role);
        QCOMPARE(material.tier, std::uint32_t(1));
        QCOMPARE(material.blurActive, false);
        QCOMPARE(material.fallback, std::uint32_t(MaterialPolicy::FloorMissed));
    }

    // The legacy v3 snapshot is guarded the same way for persistent chrome.
    MaterialPolicy legacy;
    legacy.setGlobal(2, false, MaterialPolicy::NoFallback);
    const AppliedMaterial dock = legacy.applied(MaterialPolicy::Dock);
    QCOMPARE(dock.tier, std::uint32_t(1));
    QCOMPARE(dock.blurActive, false);
    QCOMPARE(dock.fallback, std::uint32_t(MaterialPolicy::FloorMissed));
}

void TestMaterialPolicy::drawerKeepsTheGlassFloor()
{
    // The Applications drawer is transient, but it is a large launcher: a
    // malformed Minimal state must not swap its card for the opaque plate.
    MaterialPolicy policy;
    policy.setRole(MaterialPolicy::Drawer, 2, true, MaterialPolicy::NoFallback);
    const AppliedMaterial drawer = policy.applied(MaterialPolicy::Drawer);
    QCOMPARE(drawer.tier, std::uint32_t(1));
    QCOMPARE(drawer.blurActive, false);
    QCOMPARE(drawer.fallback, std::uint32_t(MaterialPolicy::FloorMissed));

    // The normal floored state the compositor sends is Reduced + blur on.
    policy.setRole(MaterialPolicy::Drawer, 1, true, MaterialPolicy::NoFallback);
    const AppliedMaterial floored = policy.applied(MaterialPolicy::Drawer);
    QCOMPARE(floored.tier, std::uint32_t(1));
    QCOMPARE(floored.blurActive, true);
    QCOMPARE(floored.fallback, std::uint32_t(0));

    QVERIFY(MaterialPolicy::keepsGlassFloor(MaterialPolicy::Drawer));
    QVERIFY(MaterialPolicy::keepsGlassFloor(MaterialPolicy::Chrome));
    QVERIFY(MaterialPolicy::keepsGlassFloor(MaterialPolicy::Dock));
    QVERIFY(!MaterialPolicy::keepsGlassFloor(MaterialPolicy::Popup));
    // The drawer is still not persistent chrome.
    QVERIFY(!MaterialPolicy::isPersistent(MaterialPolicy::Drawer));
}

void TestMaterialPolicy::floorMissedUsesTheMilkyFallback()
{
    MaterialPolicy policy;
    policy.setRole(MaterialPolicy::Chrome, 1, false, MaterialPolicy::FloorMissed);
    policy.setRole(MaterialPolicy::Dock, 1, false, MaterialPolicy::FloorMissed);
    policy.setRole(MaterialPolicy::Drawer, 1, false, MaterialPolicy::FloorMissed);
    for (std::uint32_t role : {std::uint32_t(MaterialPolicy::Chrome),
                               std::uint32_t(MaterialPolicy::Dock),
                               std::uint32_t(MaterialPolicy::Drawer)}) {
        const AppliedMaterial material = policy.applied(role);
        QCOMPARE(material.tier, std::uint32_t(1));
        QCOMPARE(material.blurActive, false);
        QCOMPARE(material.fallback, std::uint32_t(MaterialPolicy::FloorMissed));
    }
}

void TestMaterialPolicy::explicitFallbackForcesTheMilkyFallback()
{
    MaterialPolicy policy;
    policy.setRole(MaterialPolicy::Chrome, 0, true, MaterialPolicy::NoSampledBlur);
    const AppliedMaterial material = policy.applied(MaterialPolicy::Chrome);
    QCOMPARE(material.tier, std::uint32_t(0));
    QCOMPARE(material.blurActive, false);
    QCOMPARE(material.fallback, std::uint32_t(MaterialPolicy::NoSampledBlur));
}

void TestMaterialPolicy::softwareKeepsMinimalButMilky()
{
    MaterialPolicy policy;
    policy.setRole(MaterialPolicy::Chrome, 2, false, MaterialPolicy::Software);
    policy.setRole(MaterialPolicy::Dock, 2, false, MaterialPolicy::Software);
    policy.setRole(MaterialPolicy::Drawer, 2, false, MaterialPolicy::Software);
    for (std::uint32_t role : {std::uint32_t(MaterialPolicy::Chrome),
                               std::uint32_t(MaterialPolicy::Dock),
                               std::uint32_t(MaterialPolicy::Drawer)}) {
        const AppliedMaterial material = policy.applied(role);
        QCOMPARE(material.tier, std::uint32_t(2));
        QCOMPARE(material.blurActive, false);
        QCOMPARE(material.fallback, std::uint32_t(MaterialPolicy::Software));
    }
}

void TestMaterialPolicy::transientRoleMayReachMinimal()
{
    // Small transient popups still follow the ladder to Minimal; the drawer
    // is the one transient role with a glass floor.
    MaterialPolicy policy;
    policy.setRole(MaterialPolicy::Popup, 2, false, MaterialPolicy::NoFallback);
    const AppliedMaterial material = policy.applied(MaterialPolicy::Popup);
    QCOMPARE(material.tier, std::uint32_t(2));
    QCOMPARE(material.blurActive, false);
    QCOMPARE(material.fallback, std::uint32_t(0));
}

void TestMaterialPolicy::explicitFallbackWinsForTransientRoles()
{
    // T-21.2c: a popup keeps the compositor's transient tier (including
    // Minimal); any explicit reason still selects the milky complement for
    // both popups and the drawer.
    MaterialPolicy policy;
    policy.setRole(MaterialPolicy::Popup, 2, true, MaterialPolicy::NoSampledBlur);
    policy.setRole(MaterialPolicy::Drawer, 2, false, MaterialPolicy::Software);
    for (std::uint32_t role : {std::uint32_t(MaterialPolicy::Popup),
                               std::uint32_t(MaterialPolicy::Drawer)}) {
        const AppliedMaterial material = policy.applied(role);
        QCOMPARE(material.tier, std::uint32_t(2));
        QCOMPARE(material.blurActive, false);
    }
    QCOMPARE(policy.applied(MaterialPolicy::Popup).fallback,
             std::uint32_t(MaterialPolicy::NoSampledBlur));
    QCOMPARE(policy.applied(MaterialPolicy::Drawer).fallback,
             std::uint32_t(MaterialPolicy::Software));
}

void TestMaterialPolicy::loadSequenceKeepsTheAppliedProperties()
{
    // T-21.2c load sequence: Full -> Reduced -> Minimal -> Reduced -> Full.
    // The compositor emits one change-only event per role; the shell policy
    // must keep applying the matching tier/blur/fallback at every step.
    struct Step {
        std::uint32_t tier;
        bool blur;
        std::uint32_t fallback;
    };
    const Step sequence[] = {
        {0, true, MaterialPolicy::NoFallback},
        {1, true, MaterialPolicy::NoFallback},
        {1, false, MaterialPolicy::FloorMissed},
        {1, true, MaterialPolicy::NoFallback},
        {0, true, MaterialPolicy::NoFallback},
    };

    MaterialPolicy policy;
    for (const Step &step : sequence) {
        for (std::uint32_t role = 0; role < MaterialPolicy::RoleCount; ++role) {
            const bool floored = MaterialPolicy::keepsGlassFloor(role);
            // At Minimal the compositor sends the glass-floor state and the
            // transient Minimal state; the shell must not re-derive either.
            const Step expected = (step.tier == 1 && !step.blur && floored)
                ? step
                : (step.tier == 1 && !step.blur
                       ? Step{2, false, MaterialPolicy::NoFallback}
                       : step);
            policy.setRole(role, expected.tier, expected.blur, expected.fallback);
            const AppliedMaterial material = policy.applied(role);
            QCOMPARE(material.tier, expected.tier);
            QCOMPARE(material.blurActive, expected.blur);
            QCOMPARE(material.fallback, expected.fallback);
        }
    }
}

void TestMaterialPolicy::unknownRoleValuesAreIgnored()
{
    MaterialPolicy policy;
    policy.setRole(MaterialPolicy::RoleCount, 2, false, MaterialPolicy::Software);
    QVERIFY(!policy.roleKnown(MaterialPolicy::RoleCount));
    QCOMPARE(policy.applied(MaterialPolicy::RoleCount).tier, std::uint32_t(0));
}

QTEST_MAIN(TestMaterialPolicy)
#include "tst_materialpolicy.moc"
