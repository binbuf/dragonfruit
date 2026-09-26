// SPDX-License-Identifier: MIT
#pragma once

#include <QChar>
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
    QChar character;
};

// Map a raw evdev keycode (`wl_keyboard.key`) to a lock-screen action.
// `shift` selects the shifted symbol.
//
// The shell closes the compositor's `wl_keyboard.keymap` fd (it is not an xkb
// client), so this is a fixed US/ASCII layout: enough for a password field on
// the reference desktop. A non-US layout is a known limitation recorded in the
// T-12.3c hand-off; the compositor still owns which surface the key reaches.
LockKey lockKeyFromEvdev(quint32 key, bool shift);

} // namespace dragonfruit