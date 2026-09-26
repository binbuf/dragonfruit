// SPDX-License-Identifier: MIT
import QtQuick
import QtTest
import Dragonfruit
import Dragonfruit.Screenshot

// polkit authentication dialog view tests (T-13.6): the message, identity,
// prompt, masked response, details expander, accept enablement, and Escape
// cancellation. Runs headless on the offscreen platform; the agent's D-Bus
// half is covered by tst_polkitagent.cpp.
Item {
    id: stage
    width: 480
    height: 420

    TestCase {
        id: testCase
        name: "PolkitDialog"
        when: windowShown

        Component { id: dialogComponent; PolkitDialog { } }
        SignalSpy { id: submittedSpy; signalName: "submitted" }
        SignalSpy { id: cancelledSpy; signalName: "cancelled" }

        function make(props) {
            var dialog = createTemporaryObject(dialogComponent, stage, props || {});
            dialog.width = stage.width;
            dialog.height = stage.height;
            submittedSpy.target = dialog;
            cancelledSpy.target = dialog;
            submittedSpy.clear();
            cancelledSpy.clear();
            waitForRendering(stage);
            return dialog;
        }

        function clickItem(item) {
            mouseClick(item, item.width / 2, item.height / 2);
        }

        function test_the_request_renders() {
            var dialog = make({
                message: "Authentication is required to manage system services.",
                actionId: "org.freedesktop.systemd1.manage-units",
                identityLabel: "alice",
                prompt: "Password:"
            });
            compare(findChild(dialog, "polkitTitle").text, "Authentication Required");
            compare(findChild(dialog, "polkitIdentity").text, "alice");
            compare(findChild(dialog, "polkitMessage").text,
                    "Authentication is required to manage system services.");
            compare(findChild(dialog, "polkitPrompt").text, "Password:");
        }

        function test_the_response_is_masked_unless_echoed() {
            var dialog = make({ identityLabel: "alice", responseLength: 3,
                                responseText: "abc", promptEcho: false });
            var value = findChild(dialog, "polkitFieldValue");
            verify(value.visible, "the response is shown");
            compare(value.text, "•••");

            dialog.promptEcho = true;
            waitForRendering(stage);
            compare(value.text, "abc");
        }

        function test_an_empty_response_cannot_submit() {
            var dialog = make({ identityLabel: "alice" });
            var accept = findChild(dialog, "polkitAccept");
            verify(accept !== null);
            verify(!accept.enabled, "no response means no submit");

            dialog.responseLength = 3;
            dialog.responseText = "abc";
            waitForRendering(stage);
            verify(accept.enabled, "a typed response enables submit");
            clickItem(accept);
            tryCompare(submittedSpy, "count", 1);
        }

        function test_details_expand() {
            var dialog = make({
                actionId: "org.freedesktop.systemd1.manage-units",
                details: [ { key: "polkit.subject-pid", value: "42" } ]
            });
            var details = findChild(dialog, "polkitDetails");
            verify(details !== null);
            verify(!details.visible, "details start collapsed");
            var toggle = findChild(dialog, "polkitDetailsToggle");
            clickItem(toggle);
            verify(details.visible, "the toggle expands the details");
        }

        function test_an_error_is_shown() {
            var dialog = make({ errorText: "Wrong password" });
            var status = findChild(dialog, "polkitStatus");
            verify(status.visible);
            compare(status.text, "Wrong password");
        }

        function test_escape_cancels() {
            var dialog = make({ identityLabel: "alice" });
            dialog.forceActiveFocus();
            waitForRendering(stage);
            keyClick(Qt.Key_Escape);
            tryCompare(cancelledSpy, "count", 1);
        }
    }
}