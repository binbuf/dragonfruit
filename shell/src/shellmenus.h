// SPDX-License-Identifier: MIT
// The fixed system menu (T-09) and the real-session return item (T-12.6b).
//
// The drag-on-fruit (system) menu is fixed for the session. This is the pure
// builder so `tst_dockcore` can assert its rows without QML or the shell
// process. The one conditional row is **"Quit to <previous desktop>"**, shown
// only when the session was started by the real-session dev harness
// (`DRAGONFRUIT_DEV_RETURN` set).
#pragma once

#include <QString>
#include <QVariantList>

namespace ShellMenus {

// The desktop the dev harness will restore, read from `DRAGONFRUIT_DEV_RETURN`
// (empty for a normal session). The variable is the previous session id the
// harness saved before it pointed the display manager at Dragonfruit.
QString devReturnDesktop();

// The dev tool that implements the return, read from `DRAGONFRUIT_DEV_BIN`
// (empty when the harness did not name one, in which case the shell falls back
// to `dragonfruit` on PATH).
QString devBin();

// The fixed system menu (the brand mark), leftmost on the bar. Rows are
// `{label, shortcut, action, enabled}` maps, or `{type: "separator"}`. When
// `devReturn` is non-empty a `quit-to-return` row labeled
// "Quit to <devReturn>" is inserted before Lock Screen.
QVariantList systemMenu(const QString &userName, const QString &devReturn);

} // namespace ShellMenus