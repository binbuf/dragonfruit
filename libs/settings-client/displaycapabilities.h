// SPDX-License-Identifier: GPL-3.0-or-later
// The Displays capability table (T227, extended T273).
//
// The Displays pane must never advertise a control the compositor does not
// implement. This table is the one list of display rows: the supported rows
// (brightness, rotation) write the real `display.*` keys the shell forwards to
// `df_output`; every other row is disabled with the reason the underlying
// compositor/colord/sensor half is OPEN. The pane renders this table in both
// its main groups and the advanced sheet, so a row cannot quietly become a
// fake control in one place and a real one in another.
#pragma once

#include <QString>
#include <QVariantList>

// One display row. `control` is the design-system control kind the row would
// use (`slider`, `popup`, `toggle`, `button`); a disabled row still names the
// kind so the reason is attached to the right affordance. `area` is `pane`
// when the row belongs in a main pane group and
// `advanced` when it belongs only in the advanced sheet. `reason` is empty
// only when `supported` is true.
struct DisplayAdvancedRow {
    QString id;
    QString label;
    QString control;
    // The map's settings key/API for this row (used by search/deep links).
    QString key;
    bool supported = false;
    QString reason;
    // `pane` or `advanced`.
    QString area = QStringLiteral("pane");
};

// The rows in pane order. Pure; no bus, QML, or Wayland.
QList<DisplayAdvancedRow> displayAdvancedRows();

// The same table in the shape the QML `Settings` bridge exposes:
// `{ id, label, control, supported, reason, area }`.
QVariantList displayAdvancedRowsVariant();
