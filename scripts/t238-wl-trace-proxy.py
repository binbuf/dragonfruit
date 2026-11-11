#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Minimal Wayland protocol proxy used only for the T238 wake-only-swap audit.

It forwards bytes (and SCM_RIGHTS file descriptors) between one listening
socket and the real host socket, and logs the subset of client requests that
matter for a surface commit:

  wl_compositor.create_surface
  wl_surface.attach / damage / damage_buffer / frame / commit

The object map is built from wl_registry.global/bind and wl_compositor's
create_surface request, so it does not need the protocol XML at runtime.

Usage:
    wl-trace-proxy.py <upstream-socket> <listen-socket> <log-path>

The proxy runs until stdin is closed (send EOF / Ctrl-D) or SIGTERM.
"""
import array
import os
import selectors
import signal
import socket
import struct
import sys
import threading
import time

SCM_RIGHTS = getattr(socket, "SCM_RIGHTS", 1)


def recv_with_fds(sock, size=65536, fds=64):
    fd_buf = array.array("i", [0] * fds)
    msg, ancdata, flags, addr = sock.recvmsg(size, socket.CMSG_SPACE(fd_buf.itemsize * fds))
    received = []
    for level, kind, data in ancdata:
        if level == socket.SOL_SOCKET and kind == SCM_RIGHTS:
            arr = array.array("i")
            arr.frombytes(data[: len(data) - (len(data) % arr.itemsize)])
            received.extend(arr.tolist())
    return msg, received, flags


def send_with_fds(sock, data, fds):
    if fds:
        fd_buf = array.array("i", fds)
        sock.sendmsg([data], [(socket.SOL_SOCKET, SCM_RIGHTS, fd_buf.tobytes())])
    else:
        sock.sendall(data)
    for fd in fds:
        os.close(fd)


class Wire:
    def __init__(self, log, label):
        self.buf = b""
        self.log = log
        self.label = label
        # wl_display is always object 1.
        self.objects = {1: "wl_display"}  # id -> interface
        self.globals = {}       # name -> interface
        self.pending = []       # (surface id, kind, size) events for the current burst
        self.surfaces = {}      # id -> counters

    def counters(self, oid):
        return self.surfaces.setdefault(
            oid, {"attach": 0, "attach_null": 0, "damage": 0, "damage_buffer": 0,
                  "commit": 0, "frame": 0, "damage_rects": 0}
        )

    def parse_client(self, data):
        self.buf += data
        while len(self.buf) >= 8:
            oid, word = struct.unpack_from("<II", self.buf, 0)
            size = word >> 16
            opcode = word & 0xFFFF
            if size < 8 or len(self.buf) < size:
                break
            payload = self.buf[8:size]
            self.buf = self.buf[size:]
            self.handle(oid, opcode, payload)

    def u32(self, payload, offset):
        return struct.unpack_from("<I", payload, offset)[0]

    def string(self, payload, offset):
        length = self.u32(payload, offset)
        offset += 4
        raw = payload[offset:offset + length]
        return raw.rstrip(b"\0").decode("utf-8", "replace"), offset + ((length + 3) & ~3)

    def handle(self, oid, opcode, payload):
        iface = self.objects.get(oid)
        if iface == "wl_display" and opcode == 1:  # get_registry
            new_id = self.u32(payload, 0)
            self.objects[new_id] = "wl_registry"
            return
        if iface == "wl_registry" and opcode == 0:  # bind
            name = self.u32(payload, 0)
            interface, offset = self.string(payload, 4)
            new_id = self.u32(payload, offset + 4)
            self.objects[new_id] = interface
            return
        if iface == "wl_compositor" and opcode == 0:  # create_surface
            new_id = self.u32(payload, 0)
            self.objects[new_id] = "wl_surface"
            self.counters(new_id)
            return
        if iface == "wl_surface":
            counters = self.counters(oid)
            if opcode == 1:  # attach(buffer, x, y)
                buffer_id = self.u32(payload, 0)
                counters["attach"] += 1
                if buffer_id == 0:
                    counters["attach_null"] += 1
                self.pending.append((oid, "attach", 0 if buffer_id == 0 else 1))
            elif opcode == 2:  # damage(x,y,w,h)
                counters["damage"] += 1
                counters["damage_rects"] += 1
                x, y, w, h = struct.unpack_from("<iiii", payload, 0)
                self.pending.append((oid, "damage", (x, y, w, h)))
            elif opcode == 3:  # frame(new_id)
                counters["frame"] += 1
                self.pending.append((oid, "frame", 0))
            elif opcode == 6:  # commit
                counters["commit"] += 1
                kinds = [kind for (sid, kind, value) in self.pending if sid == oid]
                attaches = [value for (sid, kind, value) in self.pending
                            if sid == oid and kind == "attach"]
                damages = kinds.count("damage") + kinds.count("damage_buffer")
                rects = [
                    value for (sid, kind, value) in self.pending
                    if sid == oid and kind in ("damage", "damage_buffer")
                ]
                damage_area = sum(max(0, rect[2]) * max(0, rect[3]) for rect in rects)
                commits = counters["commit"]
                self.log.write(
                    "CLIENT commit surface=%d n=%d attach=%s damage_calls=%d "
                    "damage_area=%d damage_rects=%s frame=%s full_surface_assumed=%d\n"
                    % (
                        oid,
                        commits,
                        "buffer" if attaches and attaches[-1] == 1 else (
                            "null" if attaches else "none"),
                        damages,
                        damage_area,
                        ";".join("%d,%d,%d,%d" % rect for rect in rects) or "-",
                        "yes" if "frame" in kinds else "no",
                        1 if (attaches and attaches[-1] == 1 and damages == 0) else 0,
                    )
                )
                self.pending = [(sid, kind, value) for (sid, kind, value) in self.pending
                                if sid != oid]
            elif opcode == 9:  # damage_buffer
                counters["damage_buffer"] += 1
                x, y, w, h = struct.unpack_from("<iiii", payload, 0)
                self.pending.append((oid, "damage_buffer", (x, y, w, h)))
            return
        # Any other object with a new_id first argument: remember the type when
        # we can infer it from the interface's request table is overkill here.
        # wl_region is the only other wl_compositor child we care about.
        if iface == "wl_compositor" and opcode == 1:
            new_id = self.u32(payload, 0)
            self.objects[new_id] = "wl_region"

    def parse_server(self, data):
        self.buf += data
        while len(self.buf) >= 8:
            oid, word = struct.unpack_from("<II", self.buf, 0)
            size = word >> 16
            opcode = word & 0xFFFF
            if size < 8 or len(self.buf) < size:
                break
            payload = self.buf[8:size]
            self.buf = self.buf[size:]
            iface = self.objects.get(oid)
            if iface == "wl_registry" and opcode == 0:  # global(name, interface, version)
                name = self.u32(payload, 0)
                interface, _ = self.string(payload, 4)
                self.globals[name] = interface
            elif iface == "wl_display" and opcode == 0:  # error
                pass


class Connection:
    def __init__(self, client, upstream, log, label):
        self.client = client
        self.upstream = upstream
        self.log = log
        self.label = label
        self.c2s = Wire(log, label + " c2s")
        self.s2c = Wire(log, label + " s2c")
        # Share the object map between directions: the client creates ids and
        # the server replies on them.
        self.s2c.objects = self.c2s.objects
        self.s2c.globals = self.c2s.globals
        self.s2c.surfaces = self.c2s.surfaces
        self.closed = False

    def pump(self, src, dst, wire, name):
        try:
            while True:
                data, fds, flags = recv_with_fds(src)
                if not data:
                    if fds:
                        for fd in fds:
                            os.close(fd)
                    break
                wire.parse_client(data) if name == "c2s" else wire.parse_server(data)
                send_with_fds(dst, data, fds)
        except OSError:
            pass
        finally:
            self.close()

    def close(self):
        if self.closed:
            return
        self.closed = True
        try:
            self.client.close()
        except OSError:
            pass
        try:
            self.upstream.close()
        except OSError:
            pass

    def run(self):
        t = threading.Thread(target=self.pump, args=(self.upstream, self.client, self.s2c, "s2c"),
                             daemon=True)
        t.start()
        self.pump(self.client, self.upstream, self.c2s, "c2s")
        t.join(timeout=1.0)


def main():
    upstream_path, listen_path, log_path = sys.argv[1:4]
    try:
        os.unlink(listen_path)
    except OSError:
        pass
    log = open(log_path, "w", buffering=1)
    log.write("# wayland proxy %s -> %s\n" % (listen_path, upstream_path))
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(listen_path)
    server.listen(16)
    stop = threading.Event()

    def on_signal(_signum, _frame):
        stop.set()
        try:
            server.close()
        except OSError:
            pass

    signal.signal(signal.SIGTERM, on_signal)
    signal.signal(signal.SIGINT, on_signal)
    count = 0
    server.settimeout(0.2)
    while not stop.is_set():
        try:
            client, _ = server.accept()
        except socket.timeout:
            continue
        except OSError:
            break
        upstream = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            upstream.connect(upstream_path)
        except OSError as exc:
            log.write("# upstream connect failed: %s\n" % exc)
            client.close()
            continue
        count += 1
        conn = Connection(client, upstream, log, "conn%d" % count)
        threading.Thread(target=conn.run, daemon=True).start()
    log.write("# proxy stopped after %d connections\n" % count)
    log.close()


if __name__ == "__main__":
    main()
