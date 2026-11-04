// SPDX-License-Identifier: MIT
// T-21.1d nested GL smoke rail.
//
// Renders one QML scene through GpuChromeFrameProducer, verifies the physical
// pixel size and DPR, and writes the capture. It also logs the runtime
// linux-dmabuf/EGL probe. This binary is not part of `make e2e`: it needs a
// host Wayland compositor with a GL driver. `--expect-fallback` runs the same
// producer on a rail without GL (the offscreen QPA) and requires a clean
// recorded reason instead of a black frame.
#include <QCommandLineParser>
#include <QDir>
#include <QFileInfo>
#include <QGuiApplication>
#include <QImage>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QQuickItem>
#include <QQuickWindow>
#include <QTextStream>
#include <QtMath>

#include <cstdio>

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
    parser.addOption(outOption);
    parser.addOption(dprOption);
    parser.addOption(framesOption);
    parser.addOption(probeOnlyOption);
    parser.addOption(fallbackOption);
    parser.process(app);

    const qreal dpr = qMax(0.25, parser.value(dprOption).toDouble());
    const int frames = qMax(1, parser.value(framesOption).toInt());
    const bool expectFallback = parser.isSet(fallbackOption);
    const bool probeOnly = parser.isSet(probeOnlyOption);

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
    for (int i = 0; i < frames; ++i)
        image = producer.renderFrame();
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
