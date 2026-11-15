// SPDX-License-Identifier: MIT
#include "displaycapabilities.h"

#include <QVariantMap>

QList<DisplayAdvancedRow> displayAdvancedRows()
{
    // Supported rows first: these are the two controls the compositor
    // actually owns through `display.brightness` / `display.rotation`. Every
    // later row names the exact compositor/colord/sensor half that is OPEN.
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
          QStringLiteral("The compositor lays out every output in one row and "
                         "has no primary/mirror switch; OPEN."),
          QStringLiteral("pane") },
        { QStringLiteral("autoBrightness"), QStringLiteral("Automatically adjust brightness"),
          QStringLiteral("toggle"), QStringLiteral("display.autoBrightness"),
          false,
          QStringLiteral("The compositor has no ambient-light sensor input; OPEN."),
          QStringLiteral("pane") },
        { QStringLiteral("adaptiveColor"), QStringLiteral("Adaptive color"),
          QStringLiteral("toggle"), QStringLiteral("display.adaptiveColor"),
          false,
          QStringLiteral("No white-point sensor is exposed by the compositor; OPEN."),
          QStringLiteral("pane") },
        { QStringLiteral("refreshRate"), QStringLiteral("Refresh rate"),
          QStringLiteral("popup"), QStringLiteral("display.refreshRate"),
          false,
          QStringLiteral("The compositor reports the current mode; runtime mode "
                         "switching is OPEN."),
          QStringLiteral("pane") },
        { QStringLiteral("arrangement"), QStringLiteral("Arrangement"),
          QStringLiteral("button"), QStringLiteral("display.arrangement"),
          false,
          QStringLiteral("The compositor lays out one output today; multi-output "
                         "arrangement is OPEN."),
          QStringLiteral("pane") },
        { QStringLiteral("nightLight"), QStringLiteral("Night light"),
          QStringLiteral("toggle"), QStringLiteral("display.nightLight"),
          false,
          QStringLiteral("The compositor accepts set_night_light but does not apply "
                         "a gamma LUT; OPEN."),
          QStringLiteral("pane") },
        { QStringLiteral("nightLightSchedule"), QStringLiteral("Schedule"),
          QStringLiteral("popup"), QStringLiteral("display.nightLight.schedule"),
          false,
          QStringLiteral("The night-light gamma path is OPEN, so a schedule would "
                         "have no effect."),
          QStringLiteral("pane") },
        { QStringLiteral("nightLightTemperature"), QStringLiteral("Temperature"),
          QStringLiteral("slider"), QStringLiteral("display.nightLight.temperature"),
          false,
          QStringLiteral("The night-light gamma path is OPEN; OPEN."),
          QStringLiteral("pane") },
        { QStringLiteral("colorProfile"), QStringLiteral("Color profile"),
          QStringLiteral("popup"), QStringLiteral("display.colorProfile"),
          false,
          QStringLiteral("colord is not integrated; OPEN."),
          QStringLiteral("pane") },
        { QStringLiteral("tvMode"), QStringLiteral("When connected to TV"),
          QStringLiteral("popup"), QStringLiteral("display.tvMode"),
          false,
          QStringLiteral("The compositor has no content-type/TV policy; OPEN."),
          QStringLiteral("pane") },
        { QStringLiteral("resolutionList"), QStringLiteral("Resolution list"),
          QStringLiteral("popup"), QStringLiteral("display.resolution"),
          false,
          QStringLiteral("A true mode list is OPEN; the scaled-resolution tiles "
                         "are the live path."),
          QStringLiteral("advanced") },
        { QStringLiteral("hdr"), QStringLiteral("High dynamic range"),
          QStringLiteral("toggle"), QStringLiteral("display.hdr"),
          false,
          QStringLiteral("No HDR output path is implemented; OPEN."),
          QStringLiteral("advanced") },
        { QStringLiteral("vrr"), QStringLiteral("Variable refresh rate"),
          QStringLiteral("toggle"), QStringLiteral("display.vrr"),
          false,
          QStringLiteral("VRR is reported by the compositor but has no runtime "
                         "switch; OPEN."),
          QStringLiteral("advanced") },
        { QStringLiteral("overscan"), QStringLiteral("Overscan"),
          QStringLiteral("slider"), QStringLiteral("display.overscan"),
          false,
          QStringLiteral("The compositor has no overscan control; OPEN."),
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
