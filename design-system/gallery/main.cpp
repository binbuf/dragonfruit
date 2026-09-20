// SPDX-License-Identifier: MIT
// Dragonfruit component gallery entry point (T-08).
//
// Normal run: an interactive review app. Headless snapshot run: set
// DRAGONFRUIT_GALLERY_SNAPSHOT to a directory and the window walks every
// page x scheme x motion variant, writes PNGs, and quits. That is what the
// visual-regression script and the art-direction review consume.

#include <QDebug>
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QVariant>
#include <cstdio>

int main(int argc, char *argv[])
{
    // Keep headless snapshot failures visible: Qt's default handler is quiet
    // in some container configurations.
    qInstallMessageHandler([](QtMsgType, const QMessageLogContext &, const QString &message) {
        fprintf(stderr, "gallery: %s\n", message.toLocal8Bit().constData());
    });

    QGuiApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("dragonfruit-gallery"));
    app.setOrganizationName(QStringLiteral("dragonfruit"));

    const QString snapshotDir = QString::fromLocal8Bit(qgetenv("DRAGONFRUIT_GALLERY_SNAPSHOT"));

    QQmlApplicationEngine engine;
#ifdef DRAGONFRUIT_QML_IMPORT_DIR
    engine.addImportPath(QStringLiteral(DRAGONFRUIT_QML_IMPORT_DIR));
#endif
    engine.rootContext()->setContextProperty(QStringLiteral("dragonfruitSnapshotDir"), snapshotDir);
    QObject::connect(
        &engine,
        &QQmlApplicationEngine::objectCreationFailed,
        &app,
        []() { QCoreApplication::exit(1); },
        Qt::QueuedConnection);
    engine.loadFromModule("Dragonfruit.GalleryApp", "GalleryWindow");

    if (engine.rootObjects().isEmpty())
        return 1;

    return app.exec();
}
