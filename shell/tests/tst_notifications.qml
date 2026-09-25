// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Notifications

// Notification chrome tests (T-11.1a): the banner card renders the service
// view and the notification-center list renders the history. Runs headless on
// the offscreen platform + software scene graph; the service half (the D-Bus
// round-trip) is covered by the Rust integration suite.
Item {
    id: stage
    width: 800
    height: 480

    TestCase {
        id: testCase
        name: "Notifications"
        when: windowShown

        Component { id: bannerComponent; NotificationBanner { } }
        Component { id: centerComponent; NotificationCenter { } }

        function make(component, props) {
            var obj = createTemporaryObject(component, stage, props || {});
            waitForRendering(stage);
            return obj;
        }

        function test_the_banner_shows_the_notification() {
            var banner = make(bannerComponent, {
                appName: "Mail",
                summary: "New message",
                body: "From Ada",
                urgency: "normal"
            });
            compare(findChild(banner, "appName").text, "Mail");
            compare(findChild(banner, "summary").text, "New message");
            compare(findChild(banner, "body").text, "From Ada");
            compare(banner.Accessible.name, "New message");
        }

        function test_the_banner_hides_empty_fields() {
            var banner = make(bannerComponent, { summary: "Only a summary" });
            verify(!findChild(banner, "appName").visible);
            verify(findChild(banner, "summary").visible);
            verify(!findChild(banner, "body").visible);
        }

        function test_the_center_renders_every_history_entry() {
            var center = make(centerComponent, {
                history: [
                    { appName: "Mail", summary: "New message", body: "From Ada" },
                    { appName: "Chat", summary: "Ping", body: "" }
                ]
            });
            var list = findChild(center, "historyList");
            compare(list.count, 2);
            // Delegates are created during rendering; wait for the first row.
            var first = null;
            for (var i = 0; i < 25 && !first; ++i) {
                wait(20);
                first = list.itemAtIndex(0);
            }
            verify(first);
            compare(findChild(first, "historySummary").text, "New message");
            compare(findChild(list.itemAtIndex(1), "historySummary").text, "Ping");
            verify(!findChild(center, "emptyHistory").visible);
        }

        function test_the_center_shows_an_empty_state() {
            var center = make(centerComponent, { history: [] });
            verify(findChild(center, "emptyHistory").visible);
            compare(findChild(center, "historyList").count, 0);
        }
    }
}