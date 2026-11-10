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
    void textUsesTheLiveLayout();
    void controlChordsAreIgnored();
    void textControlKeysMap();
};

void TestLockInput::controlKeysMap()
{
    QCOMPARE(lockKeyFromEvdev(1, false).kind, LockKeyKind::Escape);
    QCOMPARE(lockKeyFromEvdev(14, false).kind, LockKeyKind::Backspace);
    QCOMPARE(lockKeyFromEvdev(28, false).kind, LockKeyKind::Return);
    QCOMPARE(lockKeyFromEvdev(96, false).kind, LockKeyKind::Return);
    QCOMPARE(lockKeyFromEvdev(57, false).kind, LockKeyKind::Character);
    QCOMPARE(lockKeyFromEvdev(57, false).text, QStringLiteral(" "));
}

void TestLockInput::lettersRespectShift()
{
    QCOMPARE(lockKeyFromEvdev(30, false).text, QStringLiteral("a"));
    QCOMPARE(lockKeyFromEvdev(30, true).text, QStringLiteral("A"));
    QCOMPARE(lockKeyFromEvdev(50, false).text, QStringLiteral("m"));
    QCOMPARE(lockKeyFromEvdev(50, true).text, QStringLiteral("M"));
}

void TestLockInput::digitsAndSymbolsRespectShift()
{
    QCOMPARE(lockKeyFromEvdev(2, false).text, QStringLiteral("1"));
    QCOMPARE(lockKeyFromEvdev(2, true).text, QStringLiteral("!"));
    QCOMPARE(lockKeyFromEvdev(11, true).text, QStringLiteral(")"));
    QCOMPARE(lockKeyFromEvdev(53, true).text, QStringLiteral("?"));
    QCOMPARE(lockKeyFromEvdev(12, false).text, QStringLiteral("-"));
    QCOMPARE(lockKeyFromEvdev(12, true).text, QStringLiteral("_"));
}

void TestLockInput::unknownKeysAreIgnored()
{
    // Shift, Ctrl, Alt, and the function keys are not text.
    QCOMPARE(lockKeyFromEvdev(42, false).kind, LockKeyKind::Ignore);
    QCOMPARE(lockKeyFromEvdev(29, false).kind, LockKeyKind::Ignore);
    QCOMPARE(lockKeyFromEvdev(59, false).kind, LockKeyKind::Ignore);
    QCOMPARE(lockKeyFromEvdev(255, false).kind, LockKeyKind::Ignore);
}

void TestLockInput::textUsesTheLiveLayout()
{
    // The compositor's xkb text already reflects Caps Lock, Shift, AltGr, the
    // active keyboard group, and the keypad. The lock field must insert that
    // text verbatim instead of re-deriving it from Shift alone.
    QCOMPARE(lockKeyFromText(30, QStringLiteral("A")).kind, LockKeyKind::Character);
    QCOMPARE(lockKeyFromText(30, QStringLiteral("A")).text, QStringLiteral("A"));
    QCOMPARE(lockKeyFromText(30, QStringLiteral("ä")).text, QStringLiteral("ä"));
    QCOMPARE(lockKeyFromText(30, QStringLiteral("€")).text, QStringLiteral("€"));
    // Keypad digits arrive as ordinary text once Num Lock is on.
    QCOMPARE(lockKeyFromText(79, QStringLiteral("1")).text, QStringLiteral("1"));
    // A multi-code-point layout output is kept whole.
    QCOMPARE(lockKeyFromText(30, QStringLiteral("e\u0301")).text,
             QStringLiteral("e\u0301"));
}

void TestLockInput::controlChordsAreIgnored()
{
    // Ctrl+A produces a control character; it must never enter a password.
    QCOMPARE(lockKeyFromText(30, QStringLiteral("\x01")).kind, LockKeyKind::Ignore);
    QCOMPARE(lockKeyFromText(30, QStringLiteral("\t")).kind, LockKeyKind::Ignore);
    // A dead key or a navigation key produces no text.
    QCOMPARE(lockKeyFromText(30, QString()).kind, LockKeyKind::Ignore);
}

void TestLockInput::textControlKeysMap()
{
    // The xkb text for these keys is a control character (or empty), so they
    // are recognised by keycode before the text is considered.
    QCOMPARE(lockKeyFromText(1, QStringLiteral("\x1b")).kind, LockKeyKind::Escape);
    QCOMPARE(lockKeyFromText(14, QStringLiteral("\b")).kind, LockKeyKind::Backspace);
    QCOMPARE(lockKeyFromText(28, QStringLiteral("\r")).kind, LockKeyKind::Return);
    QCOMPARE(lockKeyFromText(96, QStringLiteral("\r")).kind, LockKeyKind::Return);
}

QTEST_GUILESS_MAIN(TestLockInput)
#include "tst_lockinput.moc"
