// SPDX-License-Identifier: GPL-3.0-or-later
#include "displaycapabilities.h"

#include <QVariantMap>

QList<DisplayAdvancedRow> displayAdvancedRows()
{
    // Supported rows first: these are the two controls the compositor
    // actually owns through `display.brightness` / `display.rotation`. Every
    // later row carries a user-facing reason, never an internal marker (T281).
    return {
        { QStringLiteral("brightness"), QStringLiteral("Brightness"),
          QStringLiteral("slider"), QStringLiteral("display.brightness"),
          true, QString(), QStringLiteral("pane") },
        { QStringLiteral("rotation"), QStringLiteral("Rotation"),
          QStringLiteral("popup"), QStringLiteral("display.rotation"),
          true, QStringLiteral(), QStringLiteral("pane") },
        { QStringLiteral("useAs"), QStringLiteral("Use as"),
          QStringLiteral("popup"), QStringLiteral("display.useAs"),
          false,
          QStringLiteral("Mirroring another display is not available yet."),
          QStringLiteral("pane") },
        { QStringLiteral("autoBrightness"), QStringLiteral("Automatically adjust brightness"),
          QStringLiteral("toggle"), QStringLiteral("display.autoBrightness"),
          false,
          QStringLiteral("No ambient-light sensor is available."),
          QStringLiteral("pane") },
        { QStringLiteral("adaptiveColor"), QStringLiteral("Adaptive color"),
          QStringLiteral("toggle"), QStringLiteral("display.adaptiveColor"),
          false,
          QStringLiteral("This display cannot adapt its color to the room yet."),
          QStringLiteral("pane") },
        { QStringLiteral("refreshRate"), QStringLiteral("Refresh rate"),
          QStringLiteral("popup"), QStringLiteral("display.refreshRate"),
          false,
          QStringLiteral("Changing the refresh rate at runtime is not "
                         "available yet."),
          QStringLiteral("pane") },
        { QStringLiteral("arrangement"), QStringLiteral("Arrangement"),
          QStringLiteral("button"), QStringLiteral("display.arrangement"),
          false,
          QStringLiteral("Arranging multiple displays is not available yet."),
          QStringLiteral("pane") },
        { QStringLiteral("nightLight"), QStringLiteral("Night light"),
          QStringLiteral("toggle"), QStringLiteral("display.nightLight"),
          false,
          QStringLiteral("Night light is not available on this display yet."),
          QStringLiteral("pane") },
        { QStringLiteral("nightLightSchedule"), QStringLiteral("Schedule"),
          QStringLiteral("popup"), QStringLiteral("display.nightLight.schedule"),
          false,
          QStringLiteral("Set a schedule after night light is available."),
          QStringLiteral("pane") },
        { QStringLiteral("nightLightTemperature"), QStringLiteral("Temperature"),
          QStringLiteral("slider"), QStringLiteral("display.nightLight.temperature"),
          false,
          QStringLiteral("Night-light temperature is not available yet."),
          QStringLiteral("pane") },
        { QStringLiteral("colorProfile"), QStringLiteral("Color profile"),
          QStringLiteral("popup"), QStringLiteral("display.colorProfile"),
          false,
          QStringLiteral("Color profiles are not available yet."),
          QStringLiteral("pane") },
        { QStringLiteral("tvMode"), QStringLiteral("When connected to TV"),
          QStringLiteral("popup"), QStringLiteral("display.tvMode"),
          false,
          QStringLiteral("TV display handling is not available yet."),
          QStringLiteral("pane") },
        { QStringLiteral("resolutionList"), QStringLiteral("Resolution list"),
          QStringLiteral("popup"), QStringLiteral("display.resolution"),
          false,
          QStringLiteral("A full resolution list is not available; use the "
                         "scaled-resolution choices."),
          QStringLiteral("advanced") },
        { QStringLiteral("hdr"), QStringLiteral("High dynamic range"),
          QStringLiteral("toggle"), QStringLiteral("display.hdr"),
          false,
          QStringLiteral("High dynamic range is not available on this display."),
          QStringLiteral("advanced") },
        { QStringLiteral("vrr"), QStringLiteral("Variable refresh rate"),
          QStringLiteral("toggle"), QStringLiteral("display.vrr"),
          false,
          QStringLiteral("Variable refresh rate is not available on this "
                         "display."),
          QStringLiteral("advanced") },
        { QStringLiteral("overscan"), QStringLiteral("Overscan"),
          QStringLiteral("slider"), QStringLiteral("display.overscan"),
          false,
          QStringLiteral("Overscan adjustment is not available."),
          QStringLiteral("advanced") },
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
        entry.insert(QStringLiteral("area"), row.area);
        rows.append(entry);
    }
    return rows;
}
