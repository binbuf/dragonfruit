#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""T-17.1c Flatpak browser single-flow client (runs *inside* the sandbox).

`scripts/t17-flatpak-browser-driver.py` starts this file with
``flatpak run org.mozilla.firefox --command=python3`` and one flow name. Each
flow issues one real portal request through ``org.freedesktop.portal.Desktop``
and blocks until the **live Dragonfruit shell** (the presenter) answers it:

* ``file`` — ``FileChooser.OpenFile``. The request's ``current_folder`` option
  points the picker at a host scratch folder; the host driver clicks a row and
  accepts.
* ``shot`` — ``Screenshot.Screenshot`` with ``interactive = true``; the host
  driver drags a region in the live selection overlay.
* ``cast`` — ``ScreenCast.CreateSession`` / ``SelectSources`` / ``Start``; the
  host driver picks a source in the live picker and presses Share.

The flow prints a ``FLOW: WAITING <flow>`` line once the request is in flight
(so the host driver can act) and closes with ``FLOW: RESULT: PASS`` /
``FLOW: RESULT: FAIL``. No portal call is simulated: every one crosses the
sandbox's D-Bus proxy, the real ``xdg-desktop-portal`` frontend, and the
Dragonfruit backend.
"""
import os
import sys

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib  # noqa: E402

PORTAL = "org.freedesktop.portal.Desktop"
PORTAL_OBJ = "/org/freedesktop/portal/desktop"


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


def on_response_store(responses):
    def handler(_conn, _sender, path, _iface, _signal, params):
        response, results = params.unpack()
        responses[path] = (response, results)
        print(f"FLOW: Response path={path} response={response} results={results}",
              flush=True)
    return handler


def run_request(bus, responses, iface, method, params, label):
    print(f"FLOW: {label}", flush=True)
    request = call(bus, PORTAL, PORTAL_OBJ, iface, method, params).unpack()[0]
    ctx = GLib.MainContext.default()
    while request not in responses:
        ctx.iteration(True)
    return responses[request]


def flow_file(bus, responses):
    options = {"handle_token": GLib.Variant("s", "t154fc")}
    folder = os.environ.get("DF_T154_FOLDER")
    if folder:
        # The standard NUL-terminated `ay` the real frontend forwards; the
        # shell presenter trims the trailing NUL before listing.
        options["current_folder"] = GLib.Variant("ay", folder.encode() + b"\x00")
    response, results = run_request(
        bus, responses, "org.freedesktop.portal.FileChooser", "OpenFile",
        GLib.Variant("(ssa{sv})", ("", "T17 Flatpak FileChooser", options)),
        "FileChooser.OpenFile")
    print(f"FLOW: uris={results.get('uris')}", flush=True)
    return response == 0 and bool(results.get("uris"))


def flow_shot(bus, responses):
    response, results = run_request(
        bus, responses, "org.freedesktop.portal.Screenshot", "Screenshot",
        GLib.Variant("(sa{sv})", ("", {"handle_token": GLib.Variant("s", "t154shot"),
                                       "interactive": GLib.Variant("b", True)})),
        "Screenshot.Screenshot")
    print(f"FLOW: uri={results.get('uri')}", flush=True)
    return response == 0 and bool(results.get("uri"))


def flow_cast(bus, responses):
    response, created = run_request(
        bus, responses, "org.freedesktop.portal.ScreenCast", "CreateSession",
        GLib.Variant("(a{sv})", ({"handle_token": GLib.Variant("s", "t154c"),
                                  "session_handle_token": GLib.Variant("s", "t154s")},)),
        "ScreenCast.CreateSession")
    session = created.get("session_handle")
    print(f"FLOW: session={session}", flush=True)
    if response != 0 or not session:
        return False
    response, _ = run_request(
        bus, responses, "org.freedesktop.portal.ScreenCast", "SelectSources",
        GLib.Variant("(oa{sv})", (session, {"handle_token": GLib.Variant("s", "t154sel"),
                                            "types": GLib.Variant("u", 3),
                                            "multiple": GLib.Variant("b", False),
                                            "cursor_mode": GLib.Variant("u", 1)})),
        "ScreenCast.SelectSources")
    if response != 0:
        return False
    response, started = run_request(
        bus, responses, "org.freedesktop.portal.ScreenCast", "Start",
        GLib.Variant("(osa{sv})", (session, "", {"handle_token": GLib.Variant("s", "t154start")})),
        "ScreenCast.Start")
    streams = started.get("streams")
    print(f"FLOW: streams={streams}", flush=True)
    return response == 0 and bool(streams)


FLOWS = {"file": flow_file, "shot": flow_shot, "cast": flow_cast}


def main():
    if len(sys.argv) < 2 or sys.argv[1] not in FLOWS:
        print("usage: t17-flatpak-flow.py file|shot|cast", file=sys.stderr)
        return 2
    flow = sys.argv[1]
    bus = connect()
    responses = {}
    bus.signal_subscribe(PORTAL, "org.freedesktop.portal.Request", "Response",
                         None, None, Gio.DBusSignalFlags.NONE, on_response_store(responses))
    print(f"FLOW: READY {flow}", flush=True)
    try:
        ok = FLOWS[flow](bus, responses)
    except Exception as err:  # noqa: BLE001 - surface the reason to the host
        print(f"FLOW: EXCEPTION {err}", flush=True)
        ok = False
    print(f"FLOW: RESULT: {'PASS' if ok else 'FAIL'} {flow}", flush=True)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())