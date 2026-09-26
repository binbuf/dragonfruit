// SPDX-License-Identifier: MIT
#include "systemfont.h"

#include <QDebug>
#include <QFont>
#include <QFontDatabase>
#include <QGuiApplication>
#include <QStringList>

namespace {

// Kept in sync with `primitive.font.family` in design-system/tokens/tokens.json
// (the design system owns the choice; this library owns the installation).
constexpr auto kFontFamily = "Inter";

// Resource paths come from the `inter` resource attached to this target in
// CMakeLists.txt.
const char *const kFontFaces[] = {
    ":/dragonfruit/fonts/Inter-VariableFont_opsz,wght.ttf",
    ":/dragonfruit/fonts/Inter-Italic-VariableFont_opsz,wght.ttf",
};

} // namespace

// Q_INIT_RESOURCE must be called from outside any namespace, so the resource
// registration is referenced here and installSystemFont() calls this. The
// reference is also what keeps the generated resource object linked into the
// static library's consumers.
static void dragonfruitInitFontResources()
{
    Q_INIT_RESOURCE(inter);
}

void Dragonfruit::installSystemFont()
{
    static bool installed = false;
    if (installed)
        return;
    installed = true;

    dragonfruitInitFontResources();

    for (const char *face : kFontFaces) {
        const int id = QFontDatabase::addApplicationFont(QString::fromLatin1(face));
        if (id < 0 || QFontDatabase::applicationFontFamilies(id).isEmpty()) {
            qWarning("dragonfruit: failed to register system font face %s", face);
        }
    }

    if (!QFontDatabase::families().contains(QLatin1String(kFontFamily))) {
        qWarning("dragonfruit: system font family \"%s\" is unavailable; "
                 "falling back to the host default", kFontFamily);
        return;
    }

    // Swap only the family: the host's default size/hinting/resolution stay,
    // and every QML surface that sets a pixel size or weight keeps it.
    QFont font = QGuiApplication::font();
    font.setFamilies({QString::fromLatin1(kFontFamily)});
    font.setStyleName(QString());
    QGuiApplication::setFont(font);
}