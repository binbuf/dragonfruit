// SPDX-License-Identifier: MIT
#include "displaycapabilities.h"

#include <QVariantMap>

QList<DisplayAdvancedRow> displayAdvancedRows()
{
    // Supported rows first: these are the two advanced controls the compositor
    // actually owns through `display.brightness` / `display.rotation`.
    return {
        { QStringLiteral("brightness"), QStringLiteral("Brightness"),
          QStringLiteral("slider"), QStringLiteral("display.brightness"),
          true, QString() },
        { QStringLiteral("rotation"), QStringLiteral("Rotation"),
          QStringLiteral("popup"), QStringLiteral("display.rotation"),
          true, QString() },
        { QStringLiteral("refreshRate"), QStringLiteral("Refresh rate"),
          QStringLiteral("info"), QStringLiteral("display.refreshRate"),
          false,
          QStringLiteral("The compositor reports the current mode; runtime mode "
                         "switching is OPEN.") },
        { QStringLiteral("arrangement"), QStringLiteral("Arrangement"),
          QStringLiteral("info"), QStringLiteral("display.arrangement"),
          false,
          QStringLiteral("The compositor lays out one output today; multi-output "
                         "arrangement is OPEN.") },
        { QStringLiteral("nightLight"), QStringLiteral("Night light"),
          QStringLiteral("info"), QStringLiteral("display.nightLight"),
          false,
          QStringLiteral("The compositor accepts set_night_light but does not apply "
                         "a gamma LUT; OPEN.") },
        { QStringLiteral("colorProfile"), QStringLiteral("Color profile"),
          QStringLiteral("info"), QStringLiteral("display.colorProfile"),
          false,
          QStringLiteral("colord is not integrated; OPEN.") },
        { QStringLiteral("hdr"), QStringLiteral("High dynamic range"),
          QStringLiteral("info"), QStringLiteral("display.hdr"),
          false,
          QStringLiteral("No HDR output path is implemented; OPEN.") },
        { QStringLiteral("vrr"), QStringLiteral("Variable refresh rate"),
          QStringLiteral("info"), QStringLiteral("display.vrr"),
          false,
          QStringLiteral("VRR is reported by the compositor but has no runtime "
                         "switch; OPEN.") },
    };
}

QVariantList displayAdvancedRowsVariant()
{
    QVariantList rows;
    for (const DisplayAdvancedRow &row : displayAdvancedRows()) {
        QVariantMap entry;
        entry.insert(QStringLiteral("id"), row.id);
        entry.insert(QStringLiteral("label"), row.label);
        entry.insert(QStringLiteral("control"), row.control);
        entry.insert(QStringLiteral("key"), row.key);
        entry.insert(QStringLiteral("supported"), row.supported);
        entry.insert(QStringLiteral("reason"), row.reason);
        rows.append(entry);
    }
    return rows;
}
