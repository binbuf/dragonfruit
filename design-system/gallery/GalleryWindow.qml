// SPDX-License-Identifier: MIT
import QtQuick
import Dragonfruit
import Dragonfruit.Gallery

// Gallery host window. `snapshotDir` is set by main.cpp when
// DRAGONFRUIT_GALLERY_SNAPSHOT is present; the window then walks every page ×
// scheme × motion variant, writes PNGs, and quits. That is the artifact the
// visual-regression script and the human art-direction review consume.
Window {
    id: win

    width: 1080
    height: 860
    visible: true
    title: qsTr("Dragonfruit Design System")
    color: Theme.color.surfaceSunken

    property string snapshotDir: dragonfruitSnapshotDir

    Component.onCompleted: {
        if (snapshotDir.length > 0)
            win.snapshotAll(snapshotDir);
    }

    function snapshotAll(dir) {
        console.log("snapshotAll ->", dir);
        var jobs = [];
        for (var p = 0; p < gallery.pages.length; ++p) {
            jobs.push({ page: p, scheme: "light", reduced: false });
            jobs.push({ page: p, scheme: "dark", reduced: false });
            jobs.push({ page: p, scheme: "dark", reduced: true });
        }
        win.runJob(dir, jobs, 0);
    }

    function runJob(dir, jobs, index) {
        if (index >= jobs.length) {
            console.log("snapshots complete");
            Qt.quit();
            return;
        }
        var job = jobs[index];
        gallery.pageIndex = job.page;
        gallery.scheme = job.scheme;
        gallery.reducedMotion = job.reduced;
        captureTimer.dir = dir;
        captureTimer.jobs = jobs;
        captureTimer.index = index;
        captureTimer.restart();
    }

    Timer {
        id: captureTimer
        interval: 300
        property string dir: ""
        property var jobs: []
        property int index: 0

        onTriggered: {
            var job = jobs[index];
            var name = gallery.pages[job.page].toLowerCase()
                       + "_" + job.scheme + (job.reduced ? "_reduced" : "") + ".png";
            gallery.grabToImage(function(result) {
                var ok = result.saveToFile(captureTimer.dir + "/" + name);
                console.log("wrote", name, ok);
                win.runJob(captureTimer.dir, captureTimer.jobs, captureTimer.index + 1);
            });
        }
    }

    // Watchdog: never hang a headless snapshot run.
    Timer {
        interval: 60000
        running: snapshotDir.length > 0
        onTriggered: {
            console.log("snapshot watchdog fired");
            Qt.quit();
        }
    }

    Item {
        anchors.fill: parent
        anchors.margins: Theme.primitive.spacing.md

        Column {
            id: header
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            spacing: Theme.primitive.spacing.sm

            Flow {
                width: parent.width
                spacing: Theme.primitive.spacing.sm

                Repeater {
                    model: gallery.pages
                    delegate: Rectangle {
                        required property int index
                        required property string modelData
                        width: pageLabel.implicitWidth + Theme.primitive.spacing.lg
                        height: Theme.controls.titlebar.height
                        radius: Theme.primitive.radius.sm
                        color: index === gallery.pageIndex ? Theme.color.accent
                                                           : Theme.color.controlFill
                        Text {
                            id: pageLabel
                            anchors.centerIn: parent
                            text: parent.modelData
                            color: index === gallery.pageIndex ? Theme.color.accentContent
                                                               : Theme.color.textPrimary
                            font.pixelSize: Theme.primitive.font.sizeSm
                        }
                        TapHandler {
                            onTapped: gallery.pageIndex = index
                        }
                    }
                }
            }

            Row {
                spacing: Theme.primitive.spacing.sm

                Toggle {
                    text: qsTr("Dark")
                    checked: gallery.scheme === "dark"
                    onToggled: (checked) => gallery.scheme = checked ? "dark" : "light"
                }
                Toggle {
                    text: qsTr("Reduced motion")
                    checked: gallery.reducedMotion
                    onToggled: (checked) => gallery.reducedMotion = checked
                }
            }
        }

        GalleryContent {
            id: gallery
            anchors.top: header.bottom
            anchors.topMargin: Theme.primitive.spacing.sm
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
        }
    }
}
