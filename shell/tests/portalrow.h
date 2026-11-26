// SPDX-License-Identifier: GPL-3.0-or-later
// The `a(ss)` row shape the portal's `Pending*` methods return (T311).
//
// The fakes in `tst_screenshot`, `tst_screencast`, and `tst_chooser` answer
// with the real signature so the bridge's decoder is exercised on the same
// wire shape the backend produces. A `QVariantList` return would marshal as
// `av` and test a shape the real portal never sends.
#pragma once

#include <QDBusArgument>
#include <QList>
#include <QMetaType>
#include <QString>

struct PortalPendingRow
{
    QString handle;
    QString detail;
};

Q_DECLARE_METATYPE(PortalPendingRow)

inline QDBusArgument &operator<<(QDBusArgument &argument, const PortalPendingRow &row)
{
    argument.beginStructure();
    argument << row.handle << row.detail;
    argument.endStructure();
    return argument;
}

inline const QDBusArgument &operator>>(const QDBusArgument &argument, PortalPendingRow &row)
{
    argument.beginStructure();
    argument >> row.handle >> row.detail;
    argument.endStructure();
    return argument;
}
