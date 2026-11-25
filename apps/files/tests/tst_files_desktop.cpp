// SPDX-License-Identifier: MIT
// Desktop input mapping (T-19.3): the compositor's raw `wl_pointer`/`wl_keyboard`
// values become Qt modifiers/keys before `FilesDesktop` injects them into the
// offscreen desktop scene. The mapping is pure, so it is exercised here without
// a compositor or libwayland.
//
// T339 adds the process-lifecycle half: one background layer per process, and
// a fatal compositor loss (EOF on the dispatch fd) that disables the notifier
// and exits non-zero instead of spinning. `--eof-exit-probe` runs the real
// process exit path in a child so the test observes the exit code.
#include <QtTest/QtTest>

#include <QCoreApplication>
#include <QProcess>
#include <QSocketNotifier>

#include <unistd.h>

#include "DesktopInput.h"
#include "DesktopLifecycle.h"

namespace {

// The child-process half of `eofExitsTheProcessNonZero`: wire a dispatch
// notifier to the production `DesktopLifecycle` (default exit = the process
// exit), then close the write end. With the old behaviour nothing exits and
// the notifier spins; the parent sees a timeout or a non-1 exit.
int runEofExitProbe()
{
    int fds[2] = {-1, -1};
    if (::pipe(fds) != 0)
        return 2;

    DesktopLifecycle lifecycle;
    QSocketNotifier notifier(fds[0], QSocketNotifier::Read);
    QObject::connect(&notifier, &QSocketNotifier::activated, &notifier, [&]() {
        lifecycle.exitOnCompositorLoss(&notifier);
    });

    ::close(fds[1]); // the compositor disconnects: EOF on the dispatch fd
    return QCoreApplication::exec();
}

} // namespace

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
    void oneBackgroundSurfacePerProcess();
    void eofDisablesDispatchAndRequestsOneExit();
    void eofExitsTheProcessNonZero();
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

void TestFilesDesktop::oneBackgroundSurfacePerProcess()
{
    DesktopLifecycle lifecycle;
    int creations = 0;
    const auto create = [&]() {
        ++creations;
        return true;
    };

    QVERIFY(lifecycle.createSurfaceOnce(create));
    QCOMPARE(creations, 1);
    QCOMPARE(lifecycle.surfaceCount(), 1);

    // `FilesDesktop::start` used to call `createDesktopSurface()` twice. The
    // gate makes the second request a no-op, so the process cannot stack a
    // second `background` layer.
    QVERIFY(lifecycle.createSurfaceOnce(create));
    QCOMPARE(creations, 1);
    QCOMPARE(lifecycle.surfaceCount(), 1);

    // A failed creation is not counted as the one surface.
    DesktopLifecycle failing;
    QVERIFY(!failing.createSurfaceOnce([]() { return false; }));
    QCOMPARE(failing.surfaceCount(), 0);
}

void TestFilesDesktop::eofDisablesDispatchAndRequestsOneExit()
{
    int fds[2] = {-1, -1};
    QVERIFY(::pipe(fds) == 0);

    int exitCode = 0;
    int exitRequests = 0;
    DesktopLifecycle lifecycle([&](int code) {
        exitCode = code;
        ++exitRequests;
    });
    {
        QSocketNotifier notifier(fds[0], QSocketNotifier::Read);
        int activations = 0;
        QObject::connect(&notifier, &QSocketNotifier::activated, &notifier, [&]() {
            ++activations;
            lifecycle.exitOnCompositorLoss(&notifier);
        });

        ::close(fds[1]); // the compositor disconnects

        QTRY_COMPARE_WITH_TIMEOUT(exitRequests, 1, 2000);
        QCOMPARE(exitCode, 1);
        QVERIFY(!notifier.isEnabled());
        QVERIFY(lifecycle.isExiting());
        QCOMPARE(lifecycle.surfaceCount(), 0);

        // The disabled notifier stops the spin on the readable-but-dead fd,
        // and a repeated fatal does not queue a second exit.
        lifecycle.exitOnCompositorLoss(&notifier);
        QTest::qWait(50);
        QCOMPARE(activations, 1);
        QCOMPARE(exitRequests, 1);
    }
    ::close(fds[0]);
}

void TestFilesDesktop::eofExitsTheProcessNonZero()
{
    QProcess process;
    process.start(QCoreApplication::applicationFilePath(),
                  {QStringLiteral("--eof-exit-probe")});
    QVERIFY(process.waitForStarted(5000));
    QVERIFY2(process.waitForFinished(5000),
             "the desktop did not exit on EOF; the dispatch notifier kept spinning");
    QCOMPARE(process.exitStatus(), QProcess::NormalExit);
    QCOMPARE(process.exitCode(), 1);
}

int main(int argc, char *argv[])
{
    QCoreApplication app(argc, argv);
    if (argc > 1 && qstrcmp(argv[1], "--eof-exit-probe") == 0)
        return runEofExitProbe();

    TestFilesDesktop test;
    return QTest::qExec(&test, argc, argv);
}

#include "tst_files_desktop.moc"
