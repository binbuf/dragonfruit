// SPDX-License-Identifier: MIT
#pragma once

#include <QString>
#include <QtGlobal>

namespace dragonfruit {

// What a lock-screen key press means to the password field (T-12.3c).
enum class LockKeyKind {
    Ignore,
    Character,
    Backspace,
    Return,
    Escape,
};

struct LockKey {
    LockKeyKind kind = LockKeyKind::Ignore;
    // The text this key inserts. Normally one code point, but an xkb layout can
    // produce a short multi-code-point string (e.g. a composed symbol).
    QString text;
};

// Map a raw evdev keycode (`wl_keyboard.key`) to a lock-screen action using the
// live xkb text for the key. This is the production path: the shell keeps the
// compositor's `wl_keyboard.keymap` fd and translates keycodes with the user's
// actual layout, including Shift, Caps Lock, AltGr, and the numeric keypad.
//
// `text` is the UTF-8 text `wl_keyboard` produced for the key. Escape,
// Backspace, and Return are recognised by keycode because their xkb text is a
// control character (or empty). Control chords are ignored rather than
// inserted into the password.
LockKey lockKeyFromText(quint32 key, const QString &text);

// Map a raw evdev keycode with a fixed US/ASCII layout, using only Shift.
// Fallback for the brief window before the compositor's keymap arrives (and
// for the chrome scenes, which do not use the lock/polkit password path).
LockKey lockKeyFromEvdev(quint32 key, bool shift);

} // namespace dragonfruit
