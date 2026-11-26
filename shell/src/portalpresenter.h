// SPDX-License-Identifier: GPL-3.0-only
// The shell's portal-presenter identity and the `Pending*` decoder (T311).
//
// The shell owns `org.dragonfruit.PortalPresenter1` for the lifetime of its
// process. The portal backend watches that name and cancels every waiting
// FileChooser, Screenshot, and ScreenCast request when it disappears, so a
// crashed or restarted shell can never leave an application waiting.
//
// On startup each bridge also asks the backend for its `Pending*` list. The
// watch normally wins the race, but a backend that missed the disappearance
// (or a presenter name that was never acquired) must not leave a stale
// request behind: the bridge cancels what it finds. Cancelling is the honest
// choice because a `Pending*` row carries only the handle and a kind/mode;
// re-presenting a chooser or picker without its options would answer the
// caller with the wrong result (notably SaveFiles).
//
// This header is deliberately small and header-only: the three bridges share
// one spelling of the presenter name and one parser, and the tests can use the
// same parser against their fake replies.
#pragma once

#include <QString>
#include <QStringList>

#if defined(QT_DBUS_LIB)
#include <QDBusArgument>
#include <QMetaType>
#include <QVariant>
#include <QVariantList>
#endif

namespace Dragonfruit {
namespace PortalPresenter {

// The well-known name the shell owns for its lifetime.
inline QString serviceName()
{
    return QStringLiteral("org.dragonfruit.PortalPresenter1");
}

#if defined(QT_DBUS_LIB)
// Decode one `Pending*` reply into the request handles the bridge must
// cancel. The real backend answers `a(ss)` (handle, detail); the test fakes
// answer the same pairs as an `av` list. Both shapes collapse to the first
// field of each row.
inline QStringList pendingHandles(const QVariant &value)
{
    QStringList handles;

    // One row can arrive as the spec's `(ss)` struct (the real backend), as
    // an `as` list (a `QVariantList` of strings marshalled as a variant), or
    // already demarshalled as a `QVariantList` (some Qt/test paths).
    const auto appendRow = [&handles](const QDBusArgument &row) {
        const QString signature = row.currentSignature();
        if (signature == QLatin1String("(ss)")) {
            QString handle;
            QString detail;
            row.beginStructure();
            row >> handle >> detail;
            row.endStructure();
            if (!handle.isEmpty())
                handles.append(handle);
            return;
        }
        if (signature == QLatin1String("as")) {
            row.beginArray();
            if (!row.atEnd()) {
                QString handle;
                row >> handle;
                if (!handle.isEmpty())
                    handles.append(handle);
            }
            row.endArray();
            return;
        }
        if (signature == QLatin1String("s")) {
            QString handle;
            row >> handle;
            if (!handle.isEmpty())
                handles.append(handle);
        }
    };

    const auto appendPair = [&appendRow, &handles](const QVariant &row) {
        if (row.metaType() == QMetaType::fromType<QDBusArgument>()) {
            appendRow(row.value<QDBusArgument>());
            return;
        }
        const QVariantList fields = row.toList();
        if (!fields.isEmpty()) {
            const QString handle = fields.first().toString();
            if (!handle.isEmpty())
                handles.append(handle);
        }
    };

    if (value.metaType() == QMetaType::fromType<QDBusArgument>()) {
        const QDBusArgument argument = value.value<QDBusArgument>();
        argument.beginArray();
        while (!argument.atEnd()) {
            if (argument.currentSignature() == QLatin1String("(ss)")) {
                QString handle;
                QString detail;
                argument.beginStructure();
                argument >> handle >> detail;
                argument.endStructure();
                if (!handle.isEmpty())
                    handles.append(handle);
            } else {
                QVariant element;
                argument >> element;
                appendPair(element);
            }
        }
        argument.endArray();
        return handles;
    }

    const QVariantList rows = value.toList();
    for (const QVariant &row : rows)
        appendPair(row);
    return handles;
}
#endif

} // namespace PortalPresenter
} // namespace Dragonfruit
