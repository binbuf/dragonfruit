// SPDX-License-Identifier: MIT
#include "gpuchromecapabilities.h"

#include <QStringList>

#include <cstring>

namespace GpuChromeCapabilities {

namespace {

bool hasExtension(const QByteArray &extensions, const char *name)
{
    if (extensions.isEmpty())
        return false;
    const QList<QByteArray> tokens = extensions.split(' ');
    for (const QByteArray &token : tokens) {
        if (token == name)
            return true;
    }
    return false;
}

} // namespace

QVector<FormatModifier> parseFormatTable(const QByteArray &table)
{
    QVector<FormatModifier> formats;
    if (table.isEmpty() || table.size() % 16 != 0)
        return formats;
    const int count = table.size() / 16;
    formats.reserve(count);
    for (int i = 0; i < count; ++i) {
        const char *record = table.constData() + i * 16;
        FormatModifier pair;
        std::memcpy(&pair.format, record, sizeof(pair.format));
        // Bytes 4..7 are padding.
        std::memcpy(&pair.modifier, record + 8, sizeof(pair.modifier));
        if (pair.format == 0)
            continue;
        formats.append(pair);
    }
    return formats;
}

EglCapabilities parseEglExtensions(const QByteArray &clientExtensions,
                                   const QByteArray &displayExtensions)
{
    EglCapabilities caps;
    auto has = [&](const char *name) {
        return hasExtension(clientExtensions, name)
            || hasExtension(displayExtensions, name);
    };
    caps.platformWayland = has("EGL_EXT_platform_wayland")
        || has("EGL_KHR_platform_wayland");
    caps.imageBase = has("EGL_KHR_image_base") || has("EGL_KHR_image");
    caps.imageDmaBufExport = has("EGL_MESA_image_dma_buf_export");
    caps.imageDmaBufImport = has("EGL_EXT_image_dma_buf_import");
    caps.imageDmaBufImportModifiers = has("EGL_EXT_image_dma_buf_import_modifiers");
    return caps;
}

FormatSelection selectFormat(const QVector<FormatModifier> &formats,
                             const QVector<uint32_t> &preferred)
{
    for (uint32_t wanted : preferred) {
        const FormatModifier *linear = nullptr;
        const FormatModifier *explicitModifier = nullptr;
        const FormatModifier *implicitModifier = nullptr;
        for (const FormatModifier &pair : formats) {
            if (pair.format != wanted)
                continue;
            if (pair.modifier == ModifierLinear) {
                if (!linear)
                    linear = &pair;
            } else if (pair.modifier == ModifierInvalid) {
                if (!implicitModifier)
                    implicitModifier = &pair;
            } else if (!explicitModifier) {
                explicitModifier = &pair;
            }
        }
        const FormatModifier *chosen = linear ? linear
            : (explicitModifier ? explicitModifier : implicitModifier);
        if (chosen) {
            FormatSelection selection;
            selection.valid = true;
            selection.pair = *chosen;
            return selection;
        }
    }
    FormatSelection selection;
    selection.reason = QStringLiteral("no-format");
    return selection;
}

QString formatName(uint32_t format)
{
    switch (format) {
    case Abgr8888:
        return QStringLiteral("ABGR8888");
    case Argb8888:
        return QStringLiteral("ARGB8888");
    case Xbgr8888:
        return QStringLiteral("XBGR8888");
    case Xrgb8888:
        return QStringLiteral("XRGB8888");
    default:
        break;
    }
    const char chars[4] = {
        char(format & 0xff),
        char((format >> 8) & 0xff),
        char((format >> 16) & 0xff),
        char((format >> 24) & 0xff),
    };
    bool printable = true;
    for (char c : chars) {
        if (c < 0x20 || c > 0x7e) {
            printable = false;
            break;
        }
    }
    if (printable)
        return QString::fromLatin1(chars, 4);
    return QStringLiteral("0x%1").arg(format, 8, 16, QLatin1Char('0'));
}

QString modifierName(uint64_t modifier)
{
    if (modifier == ModifierLinear)
        return QStringLiteral("linear");
    if (modifier == ModifierInvalid)
        return QStringLiteral("invalid");
    return QStringLiteral("0x%1").arg(modifier, 16, 16, QLatin1Char('0'));
}

QString describe(const FormatModifier &pair)
{
    return formatName(pair.format) + QLatin1Char(':') + modifierName(pair.modifier);
}

QString formatList(const QVector<FormatModifier> &formats, int limit)
{
    QStringList tokens;
    const int count = limit > 0 ? qMin(limit, formats.size()) : formats.size();
    tokens.reserve(count + (limit > 0 && formats.size() > count ? 1 : 0));
    for (int i = 0; i < count; ++i)
        tokens.append(describe(formats.at(i)));
    if (limit > 0 && formats.size() > count)
        tokens.append(QStringLiteral("..."));
    return tokens.join(QLatin1Char(','));
}

} // namespace GpuChromeCapabilities
