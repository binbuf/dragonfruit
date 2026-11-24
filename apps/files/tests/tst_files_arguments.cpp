// SPDX-License-Identifier: MIT
// The Files command-line location argument (T-10.6b). The Dock launches Files
// with `trash://` as a positional argument, which main.cpp maps onto the
// `DF_FILES_START_URI` seam; this checks the parser independently of QML.
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QTemporaryDir>
#include <QtTest>

#include "FilesArguments.h"
#include "OpenHandler.h"

class FilesArgumentsTest : public QObject
{
    Q_OBJECT

private slots:
    void picks_the_first_non_option_argument()
    {
        const QStringList args{QStringLiteral("dragonfruit-files"),
                               QStringLiteral("trash://")};
        QCOMPARE(filesLocationArgument(args), QStringLiteral("trash://"));
    }

    void skips_leading_options()
    {
        const QStringList args{QStringLiteral("dragonfruit-files"),
                               QStringLiteral("-platform"), QStringLiteral("offscreen"),
                               QStringLiteral("file:///home/tester")};
        QCOMPARE(filesLocationArgument(args), QStringLiteral("file:///home/tester"));
    }

    void empty_when_there_is_no_argument()
    {
        QCOMPARE(filesLocationArgument({QStringLiteral("dragonfruit-files")}),
                 QString());
        QCOMPARE(filesLocationArgument({QStringLiteral("dragonfruit-files"),
                                        QStringLiteral("--help")}),
                 QString());
        QCOMPARE(filesLocationArgument({}), QString());
    }

    // -- T333 launch forms ------------------------------------------------

    void handoff_options_are_not_positional_locations()
    {
        QCOMPARE(filesLocationArgument({QStringLiteral("dragonfruit-files"),
                                        QStringLiteral("--new-window"),
                                        QStringLiteral("--location=trash://")}),
                 QString());
        QCOMPARE(filesLocationArgument({QStringLiteral("dragonfruit-files"),
                                        QStringLiteral("--activation-handoff=1-2-3"),
                                        QStringLiteral("--location=file:///tmp")}),
                 QString());
        // A positional URI after the options still wins.
        QCOMPARE(filesLocationArgument({QStringLiteral("dragonfruit-files"),
                                        QStringLiteral("--new-window"),
                                        QStringLiteral("trash://")}),
                 QStringLiteral("trash://"));
    }

    void only_a_handoff_token_requests_the_running_instance()
    {
        QVERIFY(filesActivationRequested(
            {QStringLiteral("dragonfruit-files"),
             QStringLiteral("--activation-handoff=1-2-3"),
             QStringLiteral("--location=trash://")}));
        QVERIFY(!filesActivationRequested({QStringLiteral("dragonfruit-files"),
                                           QStringLiteral("trash://")}));
        QVERIFY(!filesActivationRequested({QStringLiteral("dragonfruit-files"),
                                           QStringLiteral("--new-window")}));
    }

    void handoff_location_reads_the_location_option()
    {
        QCOMPARE(filesHandoffLocation(
                     {QStringLiteral("dragonfruit-files"),
                      QStringLiteral("--activation-handoff=1-2-3"),
                      QStringLiteral("--location=trash://")}),
                 QStringLiteral("trash://"));
        QCOMPARE(filesHandoffLocation(
                     {QStringLiteral("dragonfruit-files"),
                      QStringLiteral("--activation-handoff=1-2-3"),
                      QStringLiteral("trash://")}),
                 QStringLiteral("trash://"));
        QCOMPARE(filesHandoffLocation(
                     {QStringLiteral("dragonfruit-files"),
                      QStringLiteral("--activation-handoff=1-2-3")}),
                 QString());
    }

    void new_window_opens_a_process_and_names_its_location()
    {
        QVERIFY(filesNewWindowRequested({QStringLiteral("dragonfruit-files"),
                                         QStringLiteral("--new-window")}));
        QVERIFY(!filesNewWindowRequested({QStringLiteral("dragonfruit-files")}));
        QCOMPARE(filesLaunchLocation(
                     {QStringLiteral("dragonfruit-files"),
                      QStringLiteral("--new-window"),
                      QStringLiteral("--location=file:///tmp")}),
                 QStringLiteral("file:///tmp"));
        QCOMPARE(filesLaunchLocation(
                     {QStringLiteral("dragonfruit-files"),
                      QStringLiteral("--activation-handoff=1-2-3"),
                      QStringLiteral("--location=trash://")}),
                 QStringLiteral("trash://"));
    }

    // -- the T-19.3 desktop process -------------------------------------

    void desktop_flag_is_detected()
    {
        QVERIFY(filesDesktopRequested({QStringLiteral("dragonfruit-files"),
                                       QStringLiteral("--desktop")}));
        QVERIFY(!filesDesktopRequested({QStringLiteral("dragonfruit-files")}));
        QVERIFY(!filesDesktopRequested({QStringLiteral("dragonfruit-files"),
                                        QStringLiteral("trash://")}));
        // The desktop flag is not mistaken for a location argument.
        QCOMPARE(filesLocationArgument({QStringLiteral("dragonfruit-files"),
                                        QStringLiteral("--desktop")}),
                 QString());
    }

    void desktop_directory_is_a_local_file_uri()
    {
        const QString uri = filesDesktopDirectory();
        QVERIFY(uri.startsWith(QStringLiteral("file://")));
        const QUrl url(uri);
        QVERIFY(url.isLocalFile());
        QVERIFY(!url.toLocalFile().isEmpty());
        QVERIFY(url.toLocalFile().endsWith(QStringLiteral("Desktop"))
                || url.toLocalFile().contains(QStringLiteral("Desktop")));
    }

    // The desktop cannot open documents in-process: it runs on the offscreen
    // platform, whose QPlatformServices is unimplemented (the
    // "This plugin does not support QPlatformServices::openDocument()" log).
    // It launches an out-of-process opener instead.
    void open_handler_command_passes_the_uri_unchanged()
    {
        const QString uri =
                QStringLiteral("file:///home/tester/Desktop/.directory");
        const QStringList command = filesOpenHandlerCommand(uri);
        if (command.isEmpty())
            QSKIP("neither xdg-open nor gio is installed on this host");
        // The last argument is always the URI, for both `xdg-open URI` and
        // `gio open URI`.
        QCOMPARE(command.last(), uri);
        QVERIFY(QFileInfo(command.first()).isExecutable());
    }

    void open_handler_command_is_empty_for_an_empty_uri()
    {
        QVERIFY(filesOpenHandlerCommand(QString()).isEmpty());
    }

    void desktop_child_environment_never_forces_the_offscreen_platform()
    {
        const QByteArray previous = qgetenv("QT_QPA_PLATFORM");
        qputenv("QT_QPA_PLATFORM", "offscreen");
        const QProcessEnvironment environment = filesDesktopChildEnvironment();
        if (previous.isNull())
            qunsetenv("QT_QPA_PLATFORM");
        else
            qputenv("QT_QPA_PLATFORM", previous);
        QVERIFY(!environment.contains(QStringLiteral("QT_QPA_PLATFORM")));
    }

    // -- the T-10.6c reveal target ---------------------------------------

    void a_file_resolves_to_its_parent_and_the_file()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString file = dir.path() + QStringLiteral("/report.pdf");
        {
            QFile handle(file);
            QVERIFY(handle.open(QIODevice::WriteOnly));
            handle.write("x");
        }
        const FilesOpenTarget target = filesOpenTarget(file);
        QCOMPARE(target.location, QUrl::fromLocalFile(dir.path()).toString());
        QCOMPARE(target.revealUri, QUrl::fromLocalFile(file).toString());
    }

    void a_file_uri_resolves_to_its_parent_and_the_file()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString file = dir.path() + QStringLiteral("/notes.txt");
        {
            QFile handle(file);
            QVERIFY(handle.open(QIODevice::WriteOnly));
            handle.write("x");
        }
        const FilesOpenTarget target =
            filesOpenTarget(QUrl::fromLocalFile(file).toString());
        QCOMPARE(target.location, QUrl::fromLocalFile(dir.path()).toString());
        QCOMPARE(target.revealUri, QUrl::fromLocalFile(file).toString());
    }

    void a_directory_resolves_to_itself_without_a_reveal()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const FilesOpenTarget target = filesOpenTarget(dir.path());
        QCOMPARE(target.location, QUrl::fromLocalFile(dir.path()).toString());
        QVERIFY(target.revealUri.isEmpty());
    }

    void a_non_file_scheme_resolves_to_itself()
    {
        const FilesOpenTarget target = filesOpenTarget(QStringLiteral("trash://"));
        QCOMPARE(target.location, QStringLiteral("trash://"));
        QVERIFY(target.revealUri.isEmpty());
    }
};

QTEST_APPLESS_MAIN(FilesArgumentsTest)

#include "tst_files_arguments.moc"