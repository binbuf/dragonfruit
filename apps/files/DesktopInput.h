// SPDX-License-Identifier: MIT
// Pure input mapping for the Files-owned desktop process (T-19.3).
//
// The desktop is a Wayland client of the compositor's private protocol: the
// compositor hands it raw `wl_pointer`/`wl_keyboard` events and `FilesDesktop`
// re-injects them into the offscreen QML scene as Qt events. That translation
// is pure and lives here, header-only, so the unit test can exercise it
// without a compositor or libwayland (mirroring `FilesArguments.h`).
//
// The evdev codes and xkb `depressed` mask bits are the Wayland wire values:
// `WL_KEYBOARD_MODIFIER_MASK_SHIFT` is bit 0 (0x1), Control 0x4, Mod1/Alt 0x8,
// Mod4/Super 0x40; mouse buttons are BTN_LEFT 0x110, BTN_RIGHT 0x111,
// BTN_MIDDLE 0x112.
#pragma once

#include <QPointF>
#include <Qt>

// The xkb modifier mask from `wl_keyboard.modifiers`, as Qt modifiers.
inline Qt::KeyboardModifiers desktopModifiersFromXkb(quint32 mask)
{
    Qt::KeyboardModifiers modifiers;
    if (mask & 0x1u)
        modifiers |= Qt::ShiftModifier;
    if (mask & 0x4u)
        modifiers |= Qt::ControlModifier;
    if (mask & 0x8u)
        modifiers |= Qt::AltModifier;
    if (mask & 0x40u)
        modifiers |= Qt::MetaModifier;
    return modifiers;
}

// A raw evdev key code (from `wl_keyboard.key`) as a Qt key, or
// `Qt::Key_unknown` for keys the desktop scene does not consume. This is the
// same minimal map the shell chrome uses (navigation, reveal, activation):
// enough for arrow/Home/End navigation, Return to open, Escape to clear, and
// Delete to trash. Text entry (the inline rename editor) is a later slice.
inline Qt::Key desktopKeyFromEvdev(quint32 key)
{
    switch (key) {
    case 1:
        return Qt::Key_Escape;
    case 14:
        return Qt::Key_Backspace;
    case 28:
        return Qt::Key_Return;
    case 57:
        return Qt::Key_Space;
    case 102:
        return Qt::Key_Home;
    case 103:
        return Qt::Key_Up;
    case 105:
        return Qt::Key_Left;
    case 106:
        return Qt::Key_Right;
    case 107:
        return Qt::Key_End;
    case 108:
        return Qt::Key_Down;
    case 111:
        return Qt::Key_Delete;
    case 29:
        return Qt::Key_Control;
    case 42:
        return Qt::Key_Shift;
    case 56:
        return Qt::Key_Alt;
    case 125:
        return Qt::Key_Meta;
    default:
        return Qt::Key_unknown;
    }
}

// A raw evdev button code as a Qt mouse button, or `Qt::NoButton`.
inline Qt::MouseButton desktopMouseButton(quint32 button)
{
    switch (button) {
    case 0x110:
        return Qt::LeftButton;
    case 0x111:
        return Qt::RightButton;
    case 0x112:
        return Qt::MiddleButton;
    default:
        return Qt::NoButton;
    }
}

// Whether a fresh left press completes a double-click with the previous one
// (T-19.3). The private protocol only delivers press/release pairs, so the
// desktop synthesizes the `MouseButtonDblClick` the QML `onDoubleClicked`
// handler expects; this is the pure decision (the compositor's own double-click
// uses the same interval/distance idea). `elapsedMs` is the time since the last
// press, or a negative value when there is none.
inline bool desktopIsDoubleClick(bool haveLast, qint64 elapsedMs, int intervalMs,
                                 const QPointF &last, const QPointF &now, int distance)
{
    if (!haveLast || elapsedMs < 0)
        return false;
    if (elapsedMs > intervalMs)
        return false;
    return (now - last).manhattanLength() <= distance;
}