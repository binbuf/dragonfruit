// SPDX-License-Identifier: MIT
#include "lockinput.h"

namespace dragonfruit {
namespace {

LockKey character(QChar ch)
{
    return LockKey{ LockKeyKind::Character, ch };
}

} // namespace

LockKey lockKeyFromEvdev(quint32 key, bool shift)
{
    switch (key) {
    case 1:
        return LockKey{ LockKeyKind::Escape, QChar() };
    case 14:
        return LockKey{ LockKeyKind::Backspace, QChar() };
    case 28:
    case 96: // keypad enter
        return LockKey{ LockKeyKind::Return, QChar() };
    case 57:
        return character(QChar(' '));
    // Number row: unshifted digit, shifted symbol.
    case 2:
        return character(shift ? QChar('!') : QChar('1'));
    case 3:
        return character(shift ? QChar('@') : QChar('2'));
    case 4:
        return character(shift ? QChar('#') : QChar('3'));
    case 5:
        return character(shift ? QChar('$') : QChar('4'));
    case 6:
        return character(shift ? QChar('%') : QChar('5'));
    case 7:
        return character(shift ? QChar('^') : QChar('6'));
    case 8:
        return character(shift ? QChar('&') : QChar('7'));
    case 9:
        return character(shift ? QChar('*') : QChar('8'));
    case 10:
        return character(shift ? QChar('(') : QChar('9'));
    case 11:
        return character(shift ? QChar(')') : QChar('0'));
    case 12:
        return character(shift ? QChar('_') : QChar('-'));
    case 13:
        return character(shift ? QChar('+') : QChar('='));
    case 26:
        return character(shift ? QChar('{') : QChar('['));
    case 27:
        return character(shift ? QChar('}') : QChar(']'));
    case 39:
        return character(shift ? QChar(':') : QChar(';'));
    case 40:
        return character(shift ? QChar('"') : QChar('\''));
    case 41:
        return character(shift ? QChar('~') : QChar('`'));
    case 43:
        return character(shift ? QChar('|') : QChar('\\'));
    case 51:
        return character(shift ? QChar('<') : QChar(','));
    case 52:
        return character(shift ? QChar('>') : QChar('.'));
    case 53:
        return character(shift ? QChar('?') : QChar('/'));
    // Letter rows.
    case 16: return character(shift ? QChar('Q') : QChar('q'));
    case 17: return character(shift ? QChar('W') : QChar('w'));
    case 18: return character(shift ? QChar('E') : QChar('e'));
    case 19: return character(shift ? QChar('R') : QChar('r'));
    case 20: return character(shift ? QChar('T') : QChar('t'));
    case 21: return character(shift ? QChar('Y') : QChar('y'));
    case 22: return character(shift ? QChar('U') : QChar('u'));
    case 23: return character(shift ? QChar('I') : QChar('i'));
    case 24: return character(shift ? QChar('O') : QChar('o'));
    case 25: return character(shift ? QChar('P') : QChar('p'));
    case 30: return character(shift ? QChar('A') : QChar('a'));
    case 31: return character(shift ? QChar('S') : QChar('s'));
    case 32: return character(shift ? QChar('D') : QChar('d'));
    case 33: return character(shift ? QChar('F') : QChar('f'));
    case 34: return character(shift ? QChar('G') : QChar('g'));
    case 35: return character(shift ? QChar('H') : QChar('h'));
    case 36: return character(shift ? QChar('J') : QChar('j'));
    case 37: return character(shift ? QChar('K') : QChar('k'));
    case 38: return character(shift ? QChar('L') : QChar('l'));
    case 44: return character(shift ? QChar('Z') : QChar('z'));
    case 45: return character(shift ? QChar('X') : QChar('x'));
    case 46: return character(shift ? QChar('C') : QChar('c'));
    case 47: return character(shift ? QChar('V') : QChar('v'));
    case 48: return character(shift ? QChar('B') : QChar('b'));
    case 49: return character(shift ? QChar('N') : QChar('n'));
    case 50: return character(shift ? QChar('M') : QChar('m'));
    default:
        return LockKey{ LockKeyKind::Ignore, QChar() };
    }
}

} // namespace dragonfruit