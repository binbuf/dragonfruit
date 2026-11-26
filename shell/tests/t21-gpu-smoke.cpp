// SPDX-License-Identifier: GPL-3.0-or-later
// T-21.1d nested GL smoke rail.
//
// Renders one QML scene through GpuChromeFrameProducer, verifies the physical
// pixel size and DPR, and writes the capture. It also logs the runtime
// linux-dmabuf/EGL probe. This binary is not part of `make e2e`: it needs a
// host Wayland compositor with a GL driver. `--expect-fallback` runs the same
// producer on a rail without GL (the offscreen QPA) and requires a clean
// recorded reason instead of a black frame.
#include <QCommandLineParser>
#include <QCoreApplication>
#include <QDir>
#include <QElapsedTimer>
#include <QFileInfo>
#include <QGuiApplication>
#include <QImage>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QQuickItem>
#include <QQuickWindow>
#include <QTextStream>
#include <QThread>
#include <QVector>
#include <QtMath>

#include <cstdio>
#include <cstring>

#include "gpuchromeframeproducer.h"
#include "gpuchromeprobe.h"

namespace {

constexpr int kLogicalWidth = 160;
constexpr int kLogicalHeight = 90;

void messageHandler(QtMsgType, const QMessageLogContext &, const QString &message)
{
    std::fprintf(stderr, "t21-gpu-smoke: %s\n", message.toLocal8Bit().constData());
    std::fflush(stderr);
}

void line(const QString &text)
{
    std::fprintf(stderr, "t21 gpu smoke: %s\n", text.toLocal8Bit().constData());
    std::fflush(stderr);
}

bool colorNear(const QColor &actual, const QColor &expected, int tolerance = 12)
{
    return qAbs(actual.red() - expected.red()) <= tolerance
        && qAbs(actual.green() - expected.green()) <= tolerance
        && qAbs(actual.blue() - expected.blue()) <= tolerance
        && qAbs(actual.alpha() - expected.alpha()) <= tolerance;
}

QString colorText(const QColor &color)
{
    return color.name(QColor::HexArgb);
}

bool imagesEqual(const QImage &a, const QImage &b)
{
    if (a.isNull() || b.isNull() || a.size() != b.size()
        || a.format() != b.format())
        return false;
    for (int y = 0; y < a.height(); ++y) {
        if (std::memcmp(a.constScanLine(y), b.constScanLine(y),
                        static_cast<size_t>(a.bytesPerLine())) != 0)
            return false;
    }
    return true;
}

bool premultipliedAlphaOk(const QImage &image, qreal dpr)
{
    const QRgb corner = image.pixel(0, 0);
    if (qAlpha(corner) != 0 || qRed(corner) != 0 || qGreen(corner) != 0
        || qBlue(corner) != 0)
        return false;
    const int x = qBound(0, qRound(40 * dpr), image.width() - 1);
    const int y = qBound(0, qRound(68 * dpr), image.height() - 1);
    const QRgb alphaCard = image.pixel(x, y);
    const int alpha = qAlpha(alphaCard);
    if (qAbs(alpha - 0x80) > 6)
        return false;
    return qRed(alphaCard) <= alpha && qGreen(alphaCard) <= alpha
        && qBlue(alphaCard) <= alpha;
}

bool waitForReadyFrame(GpuChromeFrameProducer &producer, int timeoutMs)
{
    QElapsedTimer timer;
    timer.start();
    while (timer.elapsed() < timeoutMs) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 10);
        if (producer.hasReadyFrame(GpuChromeFrameProducer::PrimaryView))
            return true;
        if (!producer.hasPendingFrame(GpuChromeFrameProducer::PrimaryView))
            return false;
        QThread::msleep(1);
    }
    return false;
}

int runRingCheck(GpuChromeFrameProducer &producer, qreal dpr,
                 const QString &outPath)
{
    QQmlEngine engine;
    QQmlComponent component(&engine);
    component.setData(R"(
import QtQuick
Rectangle {
    id: root
    width: 160
    height: 90
    color: "transparent"
    property color cardColor: "#ff0000"
    Rectangle {
        x: 20; y: 12; width: 90; height: 40; radius: 8
        color: root.cardColor
    }
    Rectangle {
        x: 20; y: 58; width: 40; height: 20; radius: 4
        color: "#80336699"
    }
}
)",
                      QUrl());
    if (component.isError()) {
        line(QStringLiteral("verdict=fail reason=ring-qml-error %1")
                 .arg(component.errorString().trimmed()));
        return 1;
    }
    QObject *object = component.create();
    auto *root = qobject_cast<QQuickItem *>(object);
    if (!root) {
        line(QStringLiteral("verdict=fail reason=ring-qml-root-not-item"));
        delete object;
        return 1;
    }
    root->setParentItem(producer.window()->contentItem());
    root->setWidth(kLogicalWidth);
    root->setHeight(kLogicalHeight);
    producer.window()->setColor(Qt::transparent);
    producer.window()->resize(QSize(kLogicalWidth, kLogicalHeight));
    producer.resize(QSize(kLogicalWidth, kLogicalHeight) * dpr);
    producer.window()->update();

    const auto setCard = [&](const QColor &color) {
        root->setProperty("cardColor", color);
        producer.window()->update();
    };
    const auto exportOne = [&](const QColor &color, ChromeDmabufFrame *frame,
                               QImage *image) {
        setCard(color);
        QString reason;
        if (!producer.exportFrame(frame, &reason)) {
            line(QStringLiteral("ring export failed: %1").arg(reason));
            return false;
        }
        *image = producer.readbackCurrentFrame();
        line(QStringLiteral(
                 "ring export slot=%1 serial=%2 size=%3x%4 format=%5")
                 .arg(frame->slot)
                 .arg(frame->serial)
                 .arg(frame->size.width())
                 .arg(frame->size.height())
                 .arg(GpuChromeCapabilities::formatName(frame->format)));
        return !image->isNull();
    };

    ChromeDmabufFrame red;
    ChromeDmabufFrame green;
    ChromeDmabufFrame blue;
    QImage redImage;
    QImage greenImage;
    QImage blueImage;
    if (!exportOne(QColor(255, 0, 0), &red, &redImage)
        || !exportOne(QColor(0, 255, 0), &green, &greenImage)
        || !exportOne(QColor(0, 0, 255), &blue, &blueImage)) {
        line(QStringLiteral("verdict=fail reason=ring-export"));
        delete object;
        return 1;
    }
    const bool distinctSlots = red.slot != green.slot && green.slot != blue.slot
        && red.slot != blue.slot;
    const bool paintedDistinct = !imagesEqual(redImage, greenImage)
        && !imagesEqual(greenImage, blueImage)
        && !imagesEqual(redImage, blueImage);

    // Keep the blue frame owned so the next two submits reuse the other two
    // slots. Both reuse the unchanged blue scene with a 1x1 damage report.
    producer.releaseDmabufFrame(red.slot, red.serial);
    producer.releaseDmabufFrame(green.slot, green.serial);

    ChromeSceneDamage damage;
    damage.known = true;
    damage.rect = QRect(0, 0, 1, 1);
    QVector<ChromeDmabufFrame> reusedFrames;
    bool fullFrameOk = true;
    bool outsideDamageOk = true;
    for (int attempt = 0; attempt < 2; ++attempt) {
        QString reason;
        if (!producer.submitFrame(GpuChromeFrameProducer::PrimaryView, damage,
                                  &reason)) {
            line(QStringLiteral("ring submit failed: %1").arg(reason));
            fullFrameOk = false;
            break;
        }
        if (!waitForReadyFrame(producer, 5000)) {
            line(QStringLiteral("ring frame never became ready"));
            fullFrameOk = false;
            break;
        }
        ChromeDmabufFrame reused;
        if (!producer.takeCompletedFrame(GpuChromeFrameProducer::PrimaryView,
                                         &reused, &reason)) {
            line(QStringLiteral("ring take failed: %1").arg(reason));
            fullFrameOk = false;
            break;
        }
        const QImage image = producer.readbackCurrentFrame();
        const bool equal = imagesEqual(image, blueImage);
        fullFrameOk = fullFrameOk && equal;
        const int sx = qBound(0, qRound(65 * dpr), image.width() - 1);
        const int sy = qBound(0, qRound(32 * dpr), image.height() - 1);
        const bool outside = !image.isNull()
            && image.pixel(sx, sy) == blueImage.pixel(sx, sy);
        outsideDamageOk = outsideDamageOk && outside;
        line(QStringLiteral(
                 "ring reuse slot=%1 damage=%2 full_frame_ok=%3 "
                 "outside_damage_ok=%4")
                 .arg(reused.slot)
                 .arg(reused.damage.known() ? QStringLiteral("region")
                                            : QStringLiteral("full"))
                 .arg(equal ? 1 : 0)
                 .arg(outside ? 1 : 0));
        reusedFrames.append(reused);
    }

    const bool premultipliedOk = premultipliedAlphaOk(blueImage, dpr);
    // T357: the actual exported format must be one of the supported
    // premultiplied alpha formats. An opaque X* export would pass a naive
    // import check but turn the transparent corner black.
    const bool alphaFormatOk =
        GpuChromeCapabilities::supportsPremultipliedAlpha(red.format)
        && GpuChromeCapabilities::supportsPremultipliedAlpha(green.format)
        && GpuChromeCapabilities::supportsPremultipliedAlpha(blue.format);
    const QString absoluteOut = QFileInfo(outPath).absoluteFilePath();
    QDir().mkpath(QFileInfo(outPath).absolutePath());
    const bool saved = !blueImage.isNull() && blueImage.save(outPath);
    line(QStringLiteral(
             "ring_check dpr=%1 slots=%2,%3,%4 distinct_slots=%5 "
             "painted_distinct=%6 reused=%7 full_frame_ok=%8 "
             "outside_damage_ok=%9 premultiplied_ok=%10 alpha_format_ok=%11 "
             "saved=%12 capture=%13")
             .arg(dpr, 0, 'g', 4)
             .arg(red.slot)
             .arg(green.slot)
             .arg(blue.slot)
             .arg(distinctSlots ? 1 : 0)
             .arg(paintedDistinct ? 1 : 0)
             .arg(reusedFrames.size())
             .arg(fullFrameOk ? 1 : 0)
             .arg(outsideDamageOk ? 1 : 0)
             .arg(premultipliedOk ? 1 : 0)
             .arg(alphaFormatOk ? 1 : 0)
             .arg(saved ? 1 : 0)
             .arg(absoluteOut));

    for (const ChromeDmabufFrame &frame : reusedFrames)
        producer.releaseDmabufFrame(frame.slot, frame.serial);
    producer.releaseDmabufFrame(blue.slot, blue.serial);

    const bool pass = distinctSlots && paintedDistinct
        && reusedFrames.size() == 2 && fullFrameOk && outsideDamageOk
        && premultipliedOk && alphaFormatOk && saved;
    line(QStringLiteral("verdict=%1")
             .arg(pass ? QStringLiteral("pass") : QStringLiteral("fail")));
    delete object;
    return pass ? 0 : 1;
}

} // namespace

int main(int argc, char *argv[])
{
    qInstallMessageHandler(messageHandler);
    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("t21-gpu-smoke"));

    QCommandLineParser parser;
    parser.setApplicationDescription(QStringLiteral("T-21.1d GPU chrome smoke rail"));
    parser.addHelpOption();
    QCommandLineOption outOption(QStringLiteral("out"),
                                 QStringLiteral("PNG capture path"),
                                 QStringLiteral("path"),
                                 QStringLiteral("docs/captures/t21-gpu-smoke.png"));
    QCommandLineOption dprOption(QStringLiteral("dpr"),
                                 QStringLiteral("Device pixel ratio"),
                                 QStringLiteral("ratio"),
                                 QStringLiteral("2"));
    QCommandLineOption framesOption(QStringLiteral("frames"),
                                    QStringLiteral("Render this many frames"),
                                    QStringLiteral("count"),
                                    QStringLiteral("1"));
    QCommandLineOption probeOnlyOption(QStringLiteral("probe-only"),
                                       QStringLiteral("Probe and initialize, do not render"));
    QCommandLineOption fallbackOption(
        QStringLiteral("expect-fallback"),
        QStringLiteral("Require initialize() to fail cleanly (no-GL rail)"));
    QCommandLineOption exportOption(
        QStringLiteral("export"),
        QStringLiteral("Export the rendered FBO as a dmabuf"));
    QCommandLineOption ringCheckOption(
        QStringLiteral("ring-check"),
        QStringLiteral("Paint three slots, reuse two with partial damage, and "
                       "check premultiplied alpha"));
    parser.addOption(outOption);
    parser.addOption(dprOption);
    parser.addOption(framesOption);
    parser.addOption(probeOnlyOption);
    parser.addOption(fallbackOption);
    parser.addOption(exportOption);
    parser.addOption(ringCheckOption);
    parser.process(app);

    const qreal dpr = qMax(0.25, parser.value(dprOption).toDouble());
    const int frames = qMax(1, parser.value(framesOption).toInt());
    const bool expectFallback = parser.isSet(fallbackOption);
    const bool probeOnly = parser.isSet(probeOnlyOption);
    const bool ringCheck = parser.isSet(ringCheckOption);

    line(QStringLiteral("platform=%1 chrome_gpu=%2 dpr=%3")
             .arg(app.platformName(),
                  GpuChromeFrameProducer::environmentValue().isEmpty()
                      ? QStringLiteral("unset")
                      : GpuChromeFrameProducer::environmentValue())
             .arg(dpr, 0, 'g', 4));

    // The probe uses its own libwayland connection; Qt's Wayland QPA (when
    // active) owns a separate connection for the GL context. Both target the
    // same compositor socket, and the probe only reads capability data.
    const GpuChromeProbeResult probe = GpuChromeProbe::probe();
    GpuChromeProbe::logResult(probe, QStringLiteral("smoke"));
    if (!probe.displayConnected) {
        line(QStringLiteral("verdict=fail reason=no-wayland-display"));
        return 1;
    }

    GpuChromeFrameProducer producer;
    producer.setDevicePixelRatio(dpr);
    QString reason;
    const bool initialized = producer.initialize(&reason);
    if (expectFallback) {
        if (initialized) {
            line(QStringLiteral("verdict=fail reason=unexpected-gpu-init"));
            return 1;
        }
        line(QStringLiteral("verdict=fallback reason=%1 probe_verdict=%2")
                 .arg(reason, probe.verdict()));
        return 0;
    }
    if (!initialized) {
        line(QStringLiteral("verdict=fail reason=%1 probe_verdict=%2")
                 .arg(reason, probe.verdict()));
        return 1;
    }
    if (probeOnly) {
        line(QStringLiteral("verdict=pass mode=probe-only probe_verdict=%1 selected=%2")
                 .arg(probe.verdict(),
                      probe.selected.valid
                          ? GpuChromeCapabilities::describe(probe.selected.pair)
                          : QStringLiteral("none")));
        return 0;
    }
    if (ringCheck) {
        producer.setDmabufFormats(probe.formats);
        return runRingCheck(producer, dpr, parser.value(outOption));
    }

    QQmlEngine engine;
    QQmlComponent component(&engine);
    component.setData(R"(
import QtQuick
Rectangle {
    width: 160
    height: 90
    color: "#ff0000"
    Rectangle { x: 0;  y: 0;  width: 80; height: 45; color: "#00ff00" }
    Rectangle { x: 80; y: 45; width: 80; height: 45; color: "#0000ff" }
}
)",
                      QUrl());
    if (component.isError()) {
        line(QStringLiteral("verdict=fail reason=qml-error %1")
                 .arg(component.errorString().trimmed()));
        return 1;
    }
    QObject *object = component.create();
    auto *root = qobject_cast<QQuickItem *>(object);
    if (!root) {
        line(QStringLiteral("verdict=fail reason=qml-root-not-item"));
        delete object;
        return 1;
    }
    root->setParentItem(producer.window()->contentItem());
    root->setWidth(kLogicalWidth);
    root->setHeight(kLogicalHeight);
    producer.window()->resize(QSize(kLogicalWidth, kLogicalHeight));
    producer.resize(QSize(kLogicalWidth, kLogicalHeight) * dpr);
    line(QStringLiteral(
             "scene root visible=%1 size=%2x%3 parent=%4x%5 window_ok=%6 children=%7 "
             "opacity=%8 enabled=%9 content_visible=%10")
             .arg(root->isVisible() ? 1 : 0)
             .arg(root->width())
             .arg(root->height())
             .arg(producer.window()->contentItem()->width())
             .arg(producer.window()->contentItem()->height())
             .arg(root->window() == producer.window() ? 1 : 0)
             .arg(root->childItems().size())
             .arg(root->opacity(), 0, 'g', 3)
             .arg(root->isEnabled() ? 1 : 0)
             .arg(producer.window()->contentItem()->isVisible() ? 1 : 0));

    QImage image;
    if (parser.isSet(exportOption)) {
        producer.setDmabufFormats(probe.formats);
        ChromeDmabufFrame frame;
        QString exportReason;
        const bool exported = producer.exportFrame(&frame, &exportReason);
        line(QStringLiteral(
                 "export=%1 fd=%2 format=%3 modifier=%4 stride=%5 offset=%6 "
                 "size=%7x%8 reason=%9")
                 .arg(exported ? 1 : 0)
                 .arg(frame.fd)
                 .arg(GpuChromeCapabilities::formatName(frame.format))
                 .arg(GpuChromeCapabilities::modifierName(frame.modifier))
                 .arg(frame.stride)
                 .arg(frame.offset)
                 .arg(frame.size.width())
                 .arg(frame.size.height())
                 .arg(exportReason.isEmpty() ? QStringLiteral("none") : exportReason));
        if (!exported) {
            line(QStringLiteral("verdict=fail reason=dmabuf-export"));
            return 1;
        }
        image = producer.readbackCurrentFrame();
    } else {
        for (int i = 0; i < frames; ++i)
            image = producer.renderFrame();
    }
    const QSize expected(qCeil(kLogicalWidth * dpr), qCeil(kLogicalHeight * dpr));
    const bool sizeOk = image.size() == expected;
    const bool dprOk = qFuzzyCompare(image.devicePixelRatio(), dpr);
    const QColor green = sizeOk ? image.pixelColor(expected.width() / 4,
                                                   expected.height() / 4)
                                : QColor();
    const QColor redTop = sizeOk ? image.pixelColor(expected.width() * 3 / 4,
                                                    expected.height() / 4)
                                 : QColor();
    const QColor blue = sizeOk ? image.pixelColor(expected.width() * 3 / 4,
                                                  expected.height() * 3 / 4)
                               : QColor();
    const bool pixelsOk = sizeOk
        && colorNear(green, QColor(0, 255, 0))
        && colorNear(redTop, QColor(255, 0, 0))
        && colorNear(blue, QColor(0, 0, 255));

    const QString outPath = parser.value(outOption);
    QDir().mkpath(QFileInfo(outPath).absolutePath());
    const bool saved = !image.isNull() && image.save(outPath);
    line(QStringLiteral(
             "framebuffer=%1x%2 logical=%3x%4 dpr=%5 expected=%6x%7 "
             "pixel_tl=%8 pixel_tr=%9 pixel_br=%10 "
             "size_ok=%11 dpr_ok=%12 pixels_ok=%13 saved=%14 "
             "frames=%15 probe_verdict=%16 selected=%17")
             .arg(producer.framebufferSize().width())
             .arg(producer.framebufferSize().height())
             .arg(kLogicalWidth)
             .arg(kLogicalHeight)
             .arg(image.devicePixelRatio(), 0, 'g', 4)
             .arg(expected.width())
             .arg(expected.height())
             .arg(colorText(green), colorText(redTop), colorText(blue))
             .arg(sizeOk ? 1 : 0)
             .arg(dprOk ? 1 : 0)
             .arg(pixelsOk ? 1 : 0)
             .arg(saved ? 1 : 0)
             .arg(frames)
             .arg(probe.verdict(),
                  probe.selected.valid
                      ? GpuChromeCapabilities::describe(probe.selected.pair)
                      : QStringLiteral("none")));
    line(QStringLiteral("capture=%1").arg(QFileInfo(outPath).absoluteFilePath()));

    const bool pass = sizeOk && dprOk && pixelsOk && saved;
    line(QStringLiteral("verdict=%1").arg(pass ? QStringLiteral("pass")
                                               : QStringLiteral("fail")));
    return pass ? 0 : 1;
}
