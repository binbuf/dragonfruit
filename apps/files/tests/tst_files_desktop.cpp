// SPDX-License-Identifier: MIT
// Desktop input mapping (T-19.3): the compositor's raw `wl_pointer`/`wl_keyboard`
// values become Qt modifiers/keys before `FilesDesktop` injects them into the
// offscreen desktop scene. The mapping is pure, so it is exercised here without
// a compositor or libwayland.
#include <QtTest/QtTest>

#include "DesktopInput.h"

class TestFilesDesktop : public QObject
{
    Q_OBJECT

private slots:
    void xkbModifiersCoverCommandAndShift();
    void modifierMaskWithoutShiftLeavesShiftOff();
    void evdevKeysMapNavigationAndActivation();
    void unknownEvdevKeysAreIgnored();
    void mouseButtonsMapLeftRightMiddle();
    void doubleClickDetectsASecondPressInWindow();
};

void TestFilesDesktop::xkbModifiersCoverCommandAndShift()
{
    // Shift 0x1, Control 0x4, Mod1/Alt 0x8, Mod4/Super 0x40.
    const auto both = desktopModifiersFromXkb(0x1u | 0x40u);
    QVERIFY(both & Qt::ShiftModifier);
    QVERIFY(both & Qt::MetaModifier);
    QVERIFY(!(both & Qt::AltModifier));

    const auto ctrlAlt = desktopModifiersFromXkb(0x4u | 0x8u);
    QVERIFY(ctrlAlt & Qt::ControlModifier);
    QVERIFY(ctrlAlt & Qt::AltModifier);
    QVERIFY(!(ctrlAlt & Qt::ShiftModifier));
}

void TestFilesDesktop::modifierMaskWithoutShiftLeavesShiftOff()
{
    QCOMPARE(desktopModifiersFromXkb(0u), Qt::KeyboardModifiers());
}

void TestFilesDesktop::evdevKeysMapNavigationAndActivation()
{
    QCOMPARE(desktopKeyFromEvdev(1), Qt::Key_Escape);
    QCOMPARE(desktopKeyFromEvdev(28), Qt::Key_Return);
    QCOMPARE(desktopKeyFromEvdev(103), Qt::Key_Up);
    QCOMPARE(desktopKeyFromEvdev(108), Qt::Key_Down);
    QCOMPARE(desktopKeyFromEvdev(105), Qt::Key_Left);
    QCOMPARE(desktopKeyFromEvdev(106), Qt::Key_Right);
    QCOMPARE(desktopKeyFromEvdev(111), Qt::Key_Delete);
    QCOMPARE(desktopKeyFromEvdev(29), Qt::Key_Control);
    QCOMPARE(desktopKeyFromEvdev(125), Qt::Key_Meta);
}

void TestFilesDesktop::unknownEvdevKeysAreIgnored()
{
    QCOMPARE(desktopKeyFromEvdev(0), Qt::Key_unknown);
    QCOMPARE(desktopKeyFromEvdev(999), Qt::Key_unknown);
}

void TestFilesDesktop::mouseButtonsMapLeftRightMiddle()
{
    QCOMPARE(desktopMouseButton(0x110), Qt::LeftButton);
    QCOMPARE(desktopMouseButton(0x111), Qt::RightButton);
    QCOMPARE(desktopMouseButton(0x112), Qt::MiddleButton);
    QCOMPARE(desktopMouseButton(0x113), Qt::NoButton);
}

void TestFilesDesktop::doubleClickDetectsASecondPressInWindow()
{
    const QPointF here(40, 90);
    const QPointF nearby(42, 92);
    const QPointF far(400, 300);

    // No previous press: never a double-click.
    QVERIFY(!desktopIsDoubleClick(false, 100, 400, here, nearby, 6));
    // Within the interval and distance: a double-click.
    QVERIFY(desktopIsDoubleClick(true, 120, 400, here, nearby, 6));
    // Too slow, too far, or a nonsensical negative elapsed: not one.
    QVERIFY(!desktopIsDoubleClick(true, 500, 400, here, nearby, 6));
    QVERIFY(!desktopIsDoubleClick(true, 120, 400, here, far, 6));
    QVERIFY(!desktopIsDoubleClick(true, -1, 400, here, nearby, 6));
}

QTEST_MAIN(TestFilesDesktop)
#include "tst_files_desktop.moc"