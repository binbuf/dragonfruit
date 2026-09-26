// SPDX-License-Identifier: MIT
// Lock input tests (T-12.3c): the pure evdev -> lock-key mapping the shell
// feeds its password buffer from. No Wayland, QML, or compositor.
#include <QtTest>

#include "lockinput.h"

using namespace dragonfruit;

class TestLockInput : public QObject
{
    Q_OBJECT

private slots:
    void controlKeysMap();
    void lettersRespectShift();
    void digitsAndSymbolsRespectShift();
    void unknownKeysAreIgnored();
};

void TestLockInput::controlKeysMap()
{
    QCOMPARE(lockKeyFromEvdev(1, false).kind, LockKeyKind::Escape);
    QCOMPARE(lockKeyFromEvdev(14, false).kind, LockKeyKind::Backspace);
    QCOMPARE(lockKeyFromEvdev(28, false).kind, LockKeyKind::Return);
    QCOMPARE(lockKeyFromEvdev(96, false).kind, LockKeyKind::Return);
    QCOMPARE(lockKeyFromEvdev(57, false).kind, LockKeyKind::Character);
    QCOMPARE(lockKeyFromEvdev(57, false).character, QChar(' '));
}

void TestLockInput::lettersRespectShift()
{
    QCOMPARE(lockKeyFromEvdev(30, false).character, QChar('a'));
    QCOMPARE(lockKeyFromEvdev(30, true).character, QChar('A'));
    QCOMPARE(lockKeyFromEvdev(50, false).character, QChar('m'));
    QCOMPARE(lockKeyFromEvdev(50, true).character, QChar('M'));
}

void TestLockInput::digitsAndSymbolsRespectShift()
{
    QCOMPARE(lockKeyFromEvdev(2, false).character, QChar('1'));
    QCOMPARE(lockKeyFromEvdev(2, true).character, QChar('!'));
    QCOMPARE(lockKeyFromEvdev(11, true).character, QChar(')'));
    QCOMPARE(lockKeyFromEvdev(53, true).character, QChar('?'));
    QCOMPARE(lockKeyFromEvdev(12, false).character, QChar('-'));
    QCOMPARE(lockKeyFromEvdev(12, true).character, QChar('_'));
}

void TestLockInput::unknownKeysAreIgnored()
{
    // Shift, Ctrl, Alt, and the function keys are not text.
    QCOMPARE(lockKeyFromEvdev(42, false).kind, LockKeyKind::Ignore);
    QCOMPARE(lockKeyFromEvdev(29, false).kind, LockKeyKind::Ignore);
    QCOMPARE(lockKeyFromEvdev(59, false).kind, LockKeyKind::Ignore);
    QCOMPARE(lockKeyFromEvdev(255, false).kind, LockKeyKind::Ignore);
}

QTEST_GUILESS_MAIN(TestLockInput)
#include "tst_lockinput.moc"