#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T-13.7 Flatpak portal walkthrough driver.

Three roles, selected by the first argument (the same file runs on the host and
inside the Flatpak sandbox):

* ``presenter`` — the host-side stand-in for the shell's pickers/overlay. It
  watches the Dragonfruit backend's diagnostic ``org.dragonfruit.Portal1``
  signals and completes each waiting FileChooser / Screenshot / ScreenCast
  request. In a live session the shell plays this role; this is the headless
  half.
* ``client`` — runs inside ``flatpak run org.mozilla.firefox`` and drives the
  standard ``org.freedesktop.portal.Desktop`` frontend through a FileChooser
  open, a Screenshot, and the ScreenCast CreateSession/SelectSources/Start
  flow. It prints one ``RESULT: PASS``/``FAIL`` line and exits accordingly.
* ``trigger`` — a single FileChooser open from inside the sandbox that stays
  pending, so the live capture can photograph the shell's picker answering a
  real Flatpak request.

A real Flatpak app is required: no portal call is simulated, every one crosses
the sandbox's D-Bus proxy, the real ``xdg-desktop-portal`` frontend, and the
Dragonfruit backend.
"""
import os
import sys
import tempfile

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib  # noqa: E402

PORTAL = "org.freedesktop.portal.Desktop"
PORTAL_OBJ = "/org/freedesktop/portal/desktop"
BACKEND = "org.freedesktop.impl.portal.desktop.dragonfruit"
BACKEND_IFACE = "org.dragonfruit.Portal1"
# The presenter's recorded answers: a local file, a local image, a monitor.
# `capture-portals.sh` points these at a scratch dir; the defaults keep the
# driver runnable on its own.
PICKED_FILE = os.environ.get(
    "DF_T99_PICKED", os.path.join(tempfile.gettempdir(), "dragonfruit-t99-picked.txt")
)
PICKED_IMAGE = os.environ.get(
    "DF_T99_SHOT", os.path.join(tempfile.gettempdir(), "dragonfruit-t99-shot.png")
)


def connect():
    return Gio.DBusConnection.new_for_address_sync(
        os.environ["DBUS_SESSION_BUS_ADDRESS"],
        Gio.DBusConnectionFlags.AUTHENTICATION_CLIENT
        | Gio.DBusConnectionFlags.MESSAGE_BUS_CONNECTION,
        None,
        None,
    )


def call(bus, dest, path, iface, method, params):
    return bus.call_sync(
        dest, path, iface, method, params, None,
        Gio.DBusCallFlags.NONE, 15000, None,
    )


def presenter(bus):
    loop = GLib.MainLoop()
    state = {"count": 0}

    def on_signal(_conn, _sender, _path, _iface, signal, params):
        args = params.unpack()
        handle = args[0]
        print(f"PRESENTER: {signal} handle={handle}", flush=True)
        if signal == "FileChooserOpened":
            result = call(bus, BACKEND, PORTAL_OBJ, BACKEND_IFACE, "CompleteFileChooser",
                          GLib.Variant("(sas)", (handle, [f"file://{PICKED_FILE}"])))
        elif signal == "ScreenshotOpened":
            result = call(bus, BACKEND, PORTAL_OBJ, BACKEND_IFACE, "CompleteScreenshot",
                          GLib.Variant("(ss)", (handle, f"file://{PICKED_IMAGE}")))
        elif signal == "ScreenCastOpened":
            result = call(bus, BACKEND, PORTAL_OBJ, BACKEND_IFACE, "CompleteScreenCast",
                          GLib.Variant("(sa(su))", (handle, [("t99-monitor", 1)])))
        else:
            return
        state["count"] += 1
        print(f"PRESENTER: completed {signal} -> {result.unpack()}", flush=True)

    bus.signal_subscribe(BACKEND, BACKEND_IFACE, None, PORTAL_OBJ, None,
                         Gio.DBusSignalFlags.NONE, on_signal)
    GLib.timeout_add_seconds(int(os.environ.get("PRESENTER_SECONDS", "180")), loop.quit)
    loop.run()
    return 0 if state["count"] > 0 else 1


def on_response_store(responses):
    def handler(_conn, _sender, path, _iface, _signal, params):
        response, results = params.unpack()
        responses[path] = (response, results)
        print(f"CLIENT: Response path={path} response={response}", flush=True)
    return handler


def run_request(bus, responses, iface, method, params, label):
    print(f"CLIENT: {label}", flush=True)
    request = call(bus, PORTAL, PORTAL_OBJ, iface, method, params).unpack()[0]
    ctx = GLib.MainContext.default()
    while request not in responses:
        ctx.iteration(True)
    return responses[request]


def client(bus):
    responses = {}
    failures = []
    bus.signal_subscribe(PORTAL, "org.freedesktop.portal.Request", "Response",
                         None, None, Gio.DBusSignalFlags.NONE, on_response_store(responses))

    def token(value):
        return {"handle_token": GLib.Variant("s", value)}

    response, results = run_request(
        bus, responses, "org.freedesktop.portal.FileChooser", "OpenFile",
        GLib.Variant("(ssa{sv})", ("", "T99 Flatpak FileChooser", token("t99fc"))),
        "FileChooser.OpenFile")
    print(f"  uris = {results.get('uris')}", flush=True)
    if response != 0 or not results.get("uris"):
        failures.append("FileChooser")

    response, results = run_request(
        bus, responses, "org.freedesktop.portal.Screenshot", "Screenshot",
        GLib.Variant("(sa{sv})", ("", {"handle_token": GLib.Variant("s", "t99shot"),
                                       "interactive": GLib.Variant("b", True)})),
        "Screenshot.Screenshot")
    print(f"  uri = {results.get('uri')}", flush=True)
    if response != 0 or not results.get("uri"):
        failures.append("Screenshot")

    response, created = run_request(
        bus, responses, "org.freedesktop.portal.ScreenCast", "CreateSession",
        GLib.Variant("(a{sv})", ({"handle_token": GLib.Variant("s", "t99c"),
                                  "session_handle_token": GLib.Variant("s", "t99s")},)),
        "ScreenCast.CreateSession")
    session = created.get("session_handle")
    print(f"  session = {session}", flush=True)
    if response != 0 or not session:
        failures.append("ScreenCast.CreateSession")
        session = "/org/freedesktop/portal/desktop/session/t99/none"

    response, _ = run_request(
        bus, responses, "org.freedesktop.portal.ScreenCast", "SelectSources",
        GLib.Variant("(oa{sv})", (session, {"handle_token": GLib.Variant("s", "t99sel"),
                                            "types": GLib.Variant("u", 1),
                                            "multiple": GLib.Variant("b", False),
                                            "cursor_mode": GLib.Variant("u", 1)})),
        "ScreenCast.SelectSources")
    if response != 0:
        failures.append("ScreenCast.SelectSources")

    response, started = run_request(
        bus, responses, "org.freedesktop.portal.ScreenCast", "Start",
        GLib.Variant("(osa{sv})", (session, "", token("t99caststart"))),
        "ScreenCast.Start")
    streams = started.get("streams")
    print(f"  streams = {streams}", flush=True)
    if response != 0 or not streams:
        failures.append("ScreenCast.Start")

    print(f"CLIENT: RESULT: {'PASS' if not failures else 'FAIL ' + ','.join(failures)}",
          flush=True)
    return 0 if not failures else 1


def trigger(bus):
    """One FileChooser open that stays pending for the live capture."""
    loop = GLib.MainLoop()

    def on_response(_conn, _sender, _path, _iface, _signal, params):
        response, results = params.unpack()
        print(f"TRIGGER: Response response={response} results={results}", flush=True)
        loop.quit()

    bus.signal_subscribe(PORTAL, "org.freedesktop.portal.Request", "Response",
                         None, None, Gio.DBusSignalFlags.NONE, on_response)
    print("TRIGGER: FileChooser.OpenFile", flush=True)
    call(bus, PORTAL, PORTAL_OBJ, "org.freedesktop.portal.FileChooser", "OpenFile",
         GLib.Variant("(ssa{sv})", ("", "Open a file from the Flatpak browser",
                                    {"handle_token": GLib.Variant("s", "t99live")})))
    GLib.timeout_add_seconds(int(os.environ.get("TRIGGER_SECONDS", "90")), loop.quit)
    loop.run()
    return 0


def main():
    role = sys.argv[1] if len(sys.argv) > 1 else ""
    bus = connect()
    if role == "presenter":
        return presenter(bus)
    if role == "client":
        return client(bus)
    if role == "trigger":
        return trigger(bus)
    print("usage: flatpak-portal-driver.py presenter|client|trigger", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())