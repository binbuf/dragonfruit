// SPDX-License-Identifier: MIT
// The Files command-line location argument (T-10.6b). The Dock launches Files
// with `trash://` as a positional argument, which main.cpp maps onto the
// `DF_FILES_START_URI` seam; this checks the parser independently of QML.
#include <QDir>
#include <QFile>
#include <QTemporaryDir>
#include <QtTest>

#include "FilesArguments.h"

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