// SPDX-License-Identifier: MIT
// The Displays advanced capability table (T227).
//
// The Displays pane must never advertise a control the compositor does not
// implement. This table is the one list of advanced rows: the supported rows
// (brightness, rotation) write the real `display.*` keys the shell forwards to
// `df_output`; every other row is disabled with the reason the underlying
// compositor/colord half is OPEN. The Settings pane renders this table and the
// shell policy tests assert it, so a row cannot quietly become a fake control.
#pragma once

#include <QString>
#include <QVariantList>

// One advanced Displays row. `control` is the design-system control kind the
// pane uses for a supported row (`slider`, `popup`) or `info` for a disabled
// row. `reason` is empty only when `supported` is true.
struct DisplayAdvancedRow {
    QString id;
    QString label;
    QString control;
    // The map's settings key/API for this row (used by search/deep links).
    QString key;
    bool supported = false;
    QString reason;
};

// The rows in pane order. Pure; no bus, QML, or Wayland.
QList<DisplayAdvancedRow> displayAdvancedRows();

// The same table in the shape the QML `Settings` bridge exposes:
// `{ id, label, control, supported, reason }`.
QVariantList displayAdvancedRowsVariant();
