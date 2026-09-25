// SPDX-License-Identifier: MIT
// The menu-bar Focus/DND status item (T-11.2b).
//
// The notification service owns the policy (T-11.2a); the shell reads its
// `FocusPolicy()` view and maps it onto one menu-bar slot. This is a pure
// function over the decoded policy map so it is unit-testable without D-Bus,
// QML, or a compositor.
#pragma once

#include <QVariantMap>

// Map the Focus/DND policy view (`mode`, `allowList`, `batchedCount`) to the
// menu-bar status-item map. The item hides for `off`/unknown/absent; `focus`
// shows the crescent; `dnd` shows it selected (accent). The suppressed
// batch count is always in the accessible name and, when non-zero, the label.
QVariantMap focusStatusItem(const QVariantMap &policy);
