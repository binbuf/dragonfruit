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

        SignalSpy { id: actionSpy; signalName: "actionInvoked" }
        SignalSpy { id: activatedSpy; signalName: "activated" }

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

        function test_the_banner_renders_an_action_row_and_invokes_it() {
            var banner = make(bannerComponent, {
                appName: "Mail",
                summary: "New message",
                body: "From Ada",
                actions: [
                    { key: "reply", label: "Reply" },
                    { key: "archive", label: "Archive" }
                ]
            });
            // The card grows for the action row.
            compare(banner.cardHeight, 132);
            verify(findChild(banner, "actionsRow").visible);

            var reply = findChild(banner, "action_reply");
            verify(reply);
            actionSpy.target = banner;
            actionSpy.clear();
            mouseClick(reply, reply.width / 2, reply.height / 2);
            compare(actionSpy.count, 1);
            compare(actionSpy.signalArguments[0][0], "reply");
        }

        function test_the_banner_body_click_activates() {
            var banner = make(bannerComponent, { summary: "New message", body: "From Ada" });
            compare(banner.cardHeight, 96);
            activatedSpy.target = banner;
            activatedSpy.clear();
            mouseClick(banner, 200, 40);
            compare(activatedSpy.count, 1);
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