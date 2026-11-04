// SPDX-License-Identifier: MIT
// T-21.1d: pure GPU-chrome capability and format-selection rules.
#include <QtTest>

#include "gpuchromecapabilities.h"

using namespace GpuChromeCapabilities;

class TestGpuCapabilities : public QObject
{
    Q_OBJECT

private slots:
    void formatTableParsesNativeRecords()
    {
        QByteArray table;
        auto append = [&](uint32_t format, uint64_t modifier) {
            const char padding[4] = {0, 0, 0, 0};
            table.append(reinterpret_cast<const char *>(&format), sizeof(format));
            table.append(padding, sizeof(padding));
            table.append(reinterpret_cast<const char *>(&modifier), sizeof(modifier));
        };
        append(Abgr8888, ModifierLinear);
        append(Argb8888, 0x0102030405060708ULL);

        const QVector<FormatModifier> formats = parseFormatTable(table);
        QCOMPARE(formats.size(), 2);
        QCOMPARE(formats.at(0).format, Abgr8888);
        QCOMPARE(formats.at(0).modifier, ModifierLinear);
        QCOMPARE(formats.at(1).format, Argb8888);
        QCOMPARE(formats.at(1).modifier, quint64(0x0102030405060708ULL));
    }

    void malformedFormatTableIsRejected()
    {
        QVERIFY(parseFormatTable(QByteArray()).isEmpty());
        QVERIFY(parseFormatTable(QByteArray(15, '\0')).isEmpty());
        QVERIFY(parseFormatTable(QByteArray(17, '\0')).isEmpty());
        // A zero format record is not a usable DRM format.
        QVERIFY(parseFormatTable(QByteArray(16, '\0')).isEmpty());
    }

    void eglExtensionsAreParsedFromEitherList()
    {
        const EglCapabilities caps = parseEglExtensions(
            "EGL_MESA_image_dma_buf_export EGL_KHR_image_base",
            "EGL_EXT_image_dma_buf_import "
            "EGL_EXT_image_dma_buf_import_modifiers EGL_EXT_platform_wayland");
        QVERIFY(caps.imageDmaBufExport);
        QVERIFY(caps.imageBase);
        QVERIFY(caps.imageDmaBufImport);
        QVERIFY(caps.imageDmaBufImportModifiers);
        QVERIFY(caps.platformWayland);
        QVERIFY(caps.exportReady());

        const EglCapabilities missing = parseEglExtensions(
            "EGL_KHR_image_base", "EGL_EXT_image_dma_buf_import");
        QVERIFY(!missing.imageDmaBufExport);
        QVERIFY(!missing.exportReady());
    }

    void selectionPrefersAbgrThenLinear()
    {
        const QVector<FormatModifier> formats = {
            {Argb8888, ModifierLinear},
            {Abgr8888, 0x0100000000000001ULL},
            {Abgr8888, ModifierLinear},
        };
        const FormatSelection selected = selectFormat(formats);
        QVERIFY(selected.valid);
        QCOMPARE(selected.pair.format, Abgr8888);
        QCOMPARE(selected.pair.modifier, ModifierLinear);
    }

    void selectionFallsBackToArgbAndExplicitModifier()
    {
        const QVector<FormatModifier> formats = {
            {Argb8888, 0x0200000000000002ULL},
            {Argb8888, ModifierInvalid},
        };
        const FormatSelection selected = selectFormat(formats);
        QVERIFY(selected.valid);
        QCOMPARE(selected.pair.format, Argb8888);
        QCOMPARE(selected.pair.modifier, quint64(0x0200000000000002ULL));
    }

    void selectionUsesImplicitModifierAsLastResort()
    {
        const QVector<FormatModifier> formats = {
            {Xrgb8888, ModifierLinear},
            {Abgr8888, ModifierInvalid},
        };
        const FormatSelection selected = selectFormat(formats);
        QVERIFY(selected.valid);
        QCOMPARE(selected.pair.format, Abgr8888);
        QCOMPARE(selected.pair.modifier, ModifierInvalid);
    }

    void selectionRejectsUnsupportedFormats()
    {
        const QVector<FormatModifier> formats = {
            {Xrgb8888, ModifierLinear},
            {Xbgr8888, ModifierLinear},
        };
        const FormatSelection selected = selectFormat(formats);
        QVERIFY(!selected.valid);
        QCOMPARE(selected.reason, QStringLiteral("no-format"));

        const FormatSelection empty = selectFormat({});
        QVERIFY(!empty.valid);
        QCOMPARE(empty.reason, QStringLiteral("no-format"));
    }

    void namesAreStableForLogs()
    {
        QCOMPARE(formatName(Abgr8888), QStringLiteral("ABGR8888"));
        QCOMPARE(formatName(Argb8888), QStringLiteral("ARGB8888"));
        QCOMPARE(formatName(Xrgb8888), QStringLiteral("XRGB8888"));
        QCOMPARE(modifierName(ModifierLinear), QStringLiteral("linear"));
        QCOMPARE(modifierName(ModifierInvalid), QStringLiteral("invalid"));
        QCOMPARE(describe({Abgr8888, ModifierLinear}),
                 QStringLiteral("ABGR8888:linear"));
        QCOMPARE(formatList({{Abgr8888, ModifierLinear}, {Argb8888, ModifierLinear}}, 1),
                 QStringLiteral("ABGR8888:linear,..."));
    }
};

QTEST_MAIN(TestGpuCapabilities)
#include "tst_gpucapabilities.moc"
