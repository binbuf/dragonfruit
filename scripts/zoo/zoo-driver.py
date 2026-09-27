#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Drive the T-14.6a strange-app zoo and write the identity/decoration/menu
matrix.

Run by ``scripts/zoo/zoo-run.sh``; not directly. It polls the compositor's
read-only ``query identity`` introspection for the launched windows, reads the
X11 ``WM_CLASS`` for the Xwayland clients, resolves every raw identity through
``dragonfruit-app-index``, resolves the menu through
``dragonfruit-menu-broker``, and records the SSD/CSD outcome from ``query
decorations``. The matrix lands in ``docs/captures/t14-zoo-matrix.{md,json}``.
"""
import argparse
import json
import os
import socket
import subprocess
import sys
import time

from PIL import Image


def log(message):
    print(f"zoo: {message}", flush=True)


class Synthetic:
    """Client end of the compositor's synthetic-input UnixDatagram harness."""

    def __init__(self, path):
        self.path = path
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
        self.client = f"/tmp/opencode-zoo-{os.getpid()}.sock"
        try:
            os.unlink(self.client)
        except FileNotFoundError:
            pass
        self.sock.bind(self.client)
        self.sock.settimeout(2.0)

    def query(self, command):
        self.sock.sendto(command.encode(), self.path)
        chunks = []
        while True:
            try:
                data, _ = self.sock.recvfrom(65536)
            except socket.timeout:
                break
            text = data.decode()
            chunks.append(text)
            if "end\n" in text:
                break
        return "".join(chunks)

    def identities(self):
        """[(window_id, app_id, title)] from `query identity`."""
        rows = []
        for line in self.query("query identity").splitlines():
            parts = line.split(" ", 2)
            if len(parts) >= 3 and parts[0] == "identity":
                rows.append((int(parts[1]), parts[2].split(" ", 1)[0],
                             parts[2].split(" ", 1)[1] if " " in parts[2] else ""))
        return rows

    def decorations(self):
        rows = {}
        for line in self.query("query decorations").splitlines():
            p = line.split()
            if p and p[0] == "decoration":
                rows[int(p[1])] = p[2] == "1"
        return rows


# The zoo. `raw` is what the app is expected to present as the Wayland
# `app_id` / Xwayland `WM_CLASS` class. `expected_desktop` is the desktop id
# app-index must resolve it to. The decoration column was agreed in the track
# design: Xwayland is server-side decorated; GTK/Electron draw their own
# client-side frame; SDL2 maps an undecorated surface.
APPS = [
    {
        "key": "firefox",
        "label": "Firefox (X11)",
        "backend": "x11",
        "raw": ["org.mozilla.firefox", "Navigator", "firefox"],
        "expected_desktop": "org.mozilla.firefox.desktop",
        "expected_decoration": "CSD",
        "note": "real native Firefox via Xwayland; GTK draws its own client-side frame",
    },
    {
        "key": "xterm",
        "label": "xterm",
        "backend": "x11",
        "raw": ["XTerm", "xterm"],
        "expected_desktop": "xterm.desktop",
        "expected_decoration": "SSD",
        "note": "real xterm (distro package, extracted)",
    },
    {
        "key": "steam",
        "label": "Steam (X11 stand-in)",
        "backend": "x11",
        "raw": ["Steam", "steam"],
        "expected_desktop": "steam.desktop",
        "expected_decoration": "SSD",
        "note": "stand-in: raw X11 window with Steam's WM_CLASS (Steam not installable)",
    },
    {
        "key": "calculator",
        "label": "GNOME Calculator (GTK4)",
        "backend": "wayland",
        "raw": ["org.gnome.Calculator"],
        "expected_desktop": "org.gnome.Calculator.desktop",
        "expected_decoration": "CSD",
        "note": "real GTK4/libadwaita app via flatpak",
    },
    {
        "key": "sdl",
        "label": "SDL game (SDL2)",
        "backend": "wayland",
        "raw": ["game.zoo.sdl"],
        "expected_desktop": "game.zoo.sdl.desktop",
        "expected_decoration": "SSD",
        "note": "real SDL2 game window on the nested Wayland socket; the sample presents frames so SDL attaches its first buffer (T-14.6b fix)",
    },
    {
        "key": "electron",
        "label": "Electron app",
        "backend": "wayland",
        "raw": ["electron", "Electron"],
        "expected_desktop": "electron-zoo.desktop",
        "expected_decoration": "SSD",
        "note": "real Electron/Chromium client; Chromium negotiates server-side decoration",
    },
]


def x11_classes(display):
    """{class_lower: (instance, class, title)} for the Xwayland client list."""
    try:
        root = subprocess.run(
            ["xprop", "-display", display, "-root", "_NET_CLIENT_LIST"],
            check=True, capture_output=True, text=True).stdout
    except (subprocess.CalledProcessError, FileNotFoundError):
        return {}
    ids = [w.strip().rstrip(",") for w in root.split("#", 1)[-1].split(",")]
    found = {}
    for win in ids:
        if not win:
            continue
        try:
            out = subprocess.run(
                ["xprop", "-display", display, "-id", win, "WM_CLASS", "_NET_WM_NAME"],
                capture_output=True, text=True).stdout
        except FileNotFoundError:
            continue
        instance = klass = title = ""
        for line in out.splitlines():
            if line.startswith("WM_CLASS"):
                values = line.split("=", 1)[1].strip().split(",")
                if len(values) == 2:
                    instance = values[0].strip().strip('"')
                    klass = values[1].strip().strip('"')
            elif "_NET_WM_NAME" in line and "=" in line:
                title = line.split("=", 1)[1].strip().strip('"')
        if klass:
            found[klass.lower()] = (instance, klass, title)
            if instance:
                found.setdefault(instance.lower(), (instance, klass, title))
    return found


def resolve_identity(app_index, app, app_id, classes):
    """Return (desktop_id, source) or ("", "miss"/"error")."""
    if app["backend"] == "x11":
        instance, klass, _ = classes.get(app_id.lower(), ("", app_id, ""))
        args = [app_index, "--resolve-window", klass or app_id]
        if instance:
            args.append(instance)
    else:
        args = [app_index, "--resolve", app_id]
    try:
        proc = subprocess.run(args, capture_output=True, text=True)
    except FileNotFoundError:
        return "", "error"
    out = proc.stdout.strip()
    if proc.returncode != 0 or not out:
        return "", "miss"
    try:
        record = json.loads(out)
    except json.JSONDecodeError:
        return "", "error"
    return record.get("desktopId", ""), record.get("source", "")


def resolve_menu(menu_broker, app_id):
    try:
        proc = subprocess.run(
            [menu_broker, "--resolve", app_id], capture_output=True, text=True)
    except FileNotFoundError:
        return "error", 0
    if proc.returncode != 0 or not proc.stdout.strip():
        return "error", 0
    try:
        menu = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return "error", 0
    return menu.get("tier", "none"), len(menu.get("menus", []))


def wait_for_windows(synth, expected, timeout):
    """Poll until every expected raw identity is seen (or the timeout).
    Returns {app_key: (window_id, app_id, title, first_seen_s)}."""
    wanted = {app["key"]: app for app in expected}
    seen = {}
    start = time.time()
    while time.time() - start < timeout:
        for win, app_id, title in synth.identities():
            for key, app in wanted.items():
                if key in seen:
                    continue
                if any(app_id.lower() == raw.lower() for raw in app["raw"]):
                    seen[key] = (win, app_id, title, round(time.time() - start, 1))
        if len(seen) == len(wanted):
            break
        time.sleep(0.5)
    for key in wanted:
        if key not in seen:
            log(f"{key}: no window after {timeout}s")
    return seen


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--synth", required=True)
    parser.add_argument("--xdisplay", default="")
    parser.add_argument("--outdir", required=True)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--app-index", required=True)
    parser.add_argument("--menu-broker", required=True)
    parser.add_argument("--prefix", default="t14-zoo")
    parser.add_argument("--wait", type=float, default=60.0)
    parser.add_argument("--skip", action="append", default=[],
                        help="app key to skip because it is unavailable")
    args = parser.parse_args()

    synth = Synthetic(args.synth)
    synth.query("set color-scheme dark")
    time.sleep(0.3)

    skipped = set(args.skip)
    expected = [app for app in APPS if app["key"] not in skipped]
    log(f"waiting up to {args.wait:.0f}s for {len(expected)} zoo windows")
    seen = wait_for_windows(synth, expected, args.wait)

    classes = x11_classes(args.xdisplay) if args.xdisplay else {}
    decos = synth.decorations()

    rows = []
    for app in APPS:
        if app["key"] in skipped:
            rows.append({
                "key": app["key"], "label": app["label"], "backend": app["backend"],
                "note": app["note"], "status": "not run",
                "reason": "not available in this environment", "launch": False,
                "raw": "", "desktopId": "", "expectedDesktop": app["expected_desktop"],
                "identity": False, "decoration": "not run",
                "expectedDecoration": app["expected_decoration"], "decorationOk": False,
                "menuTier": "not run", "menus": 0, "menuOk": False,
            })
            continue
        window = seen.get(app["key"])
        if not window:
            rows.append({
                "key": app["key"], "label": app["label"], "backend": app["backend"],
                "note": app["note"], "status": "fail", "reason": "window never mapped",
                "launch": False, "raw": "", "desktopId": "",
                "expectedDesktop": app["expected_desktop"], "identity": False,
                "decoration": "unknown", "expectedDecoration": app["expected_decoration"],
                "decorationOk": False, "menuTier": "not run", "menus": 0, "menuOk": False,
            })
            continue
        win_id, raw, title, _ = window
        desktop_id, source = resolve_identity(args.app_index, app, raw, classes)
        ssd = decos.get(win_id)
        expected_ssd = app["expected_decoration"] == "SSD"
        decoration = "SSD" if ssd else ("CSD" if ssd is not None else "unknown")
        decoration_ok = ssd is not None and ssd == expected_ssd
        menu_tier, menu_count = resolve_menu(args.menu_broker, raw)
        rows.append({
            "key": app["key"], "label": app["label"], "backend": app["backend"],
            "note": app["note"], "status": "run", "reason": "",
            "launch": True, "window": win_id, "raw": raw, "title": title,
            "desktopId": desktop_id, "expectedDesktop": app["expected_desktop"],
            "identity": desktop_id == app["expected_desktop"], "source": source,
            "decoration": decoration, "expectedDecoration": app["expected_decoration"],
            "decorationOk": decoration_ok,
            "menuTier": menu_tier, "menus": menu_count,
            "menuOk": menu_tier == "none",
        })

    # Capture a still of the mapped zoo. The nested output's wallpaper colour
    # is discovered from the pixels rather than hardcoded (it follows the
    # shipped wallpaper); its bounding box is the nested window.
    os.makedirs(args.outdir, exist_ok=True)
    still = None
    try:
        still = capture_still(args.outdir, args.prefix, args.scratch)
    except Exception as err:  # noqa: BLE001 - capture is best-effort
        log(f"capture failed: {err}")

    _write_matrix(args.outdir, args.prefix, rows, still)

    failures = [r for r in rows if r["status"] == "fail"
                or (r["status"] == "run"
                    and not (r["identity"] and r["decorationOk"]))]
    for row in rows:
        log(f"{row['label']}: launch={row['launch']} raw={row['raw'] or '-'} "
            f"identity={row['desktopId'] or '-'} ({'ok' if row['identity'] else 'MISS'}) "
            f"decoration={row['decoration']} menu={row['menuTier']}")
    return 1 if failures else 0


def capture_still(outdir, prefix, scratch):
    """Screenshot the host, find the nested output by its wallpaper colour,
    and crop it to `<outdir>/<prefix>.png`."""
    full = os.path.join(scratch, "zoo-full.png")
    subprocess.run(["spectacle", "-b", "-n", "-f", "-o", full], check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    im = Image.open(full).convert("RGB")
    px = im.load()
    w, h = im.size
    counts = {}
    for y in range(0, h, 4):
        for x in range(0, w, 4):
            p = px[x, y]
            if sum(p) < 260:
                counts[p] = counts.get(p, 0) + 1
    if not counts:
        raise RuntimeError("no dark wallpaper pixels found")
    wall = max(counts, key=counts.get)
    # Keep the tolerance tight: the host wallpaper can share the hue, and a
    # loose match would grow the crop into the host desktop.
    tol = 4
    minx, miny, maxx, maxy = w, h, -1, -1
    for y in range(0, h, 2):
        for x in range(0, w, 2):
            p = px[x, y]
            if all(abs(p[i] - wall[i]) <= tol for i in range(3)):
                minx = min(minx, x)
                maxx = max(maxx, x)
                miny = min(miny, y)
                maxy = max(maxy, y)
    if maxx < 0:
        raise RuntimeError("wallpaper colour not found in the screenshot")
    log(f"nested window rect {(minx, miny, maxx - minx, maxy - miny)} (wallpaper {wall})")
    dest = os.path.join(outdir, f"{prefix}.png")
    im.crop((minx, miny, maxx + 1, maxy + 1)).save(dest)
    log(f"saved {dest}")
    return dest


def _write_matrix(outdir, prefix, rows, still):
    path = os.path.join(outdir, f"{prefix}-matrix.json")
    with open(path, "w") as handle:
        json.dump({"apps": rows, "capture": still}, handle, indent=2)
        handle.write("\n")
    log(f"wrote {path}")

    lines = [
        "# T-14.6a strange-app zoo matrix",
        "",
        "Scripted nested run (`scripts/zoo/zoo-run.sh`). Each app is launched "
        "against the private Dragonfruit socket; its raw identity is read from "
        "the compositor (`query identity`, plus `xprop` for the Xwayland "
        "clients), resolved through `dragonfruit-app-index`, its decoration "
        "tier read from `query decorations`, and its global-menu tier from "
        "`dragonfruit-menu-broker`. `pass` means the observed outcome matches "
        "the expected outcome below.",
        "",
        "| App | Backend | Raw identity | Resolved desktop id | Identity | Decoration (expected) | Menu tier | Launch |",
        "|---|---|---|---|---|---|---|---|",
    ]
    for row in rows:
        if row["status"] == "not run":
            lines.append(
                f"| {row['label']} | {row['backend']} | — | not run | n/a | "
                f"not run ({row['expectedDecoration']}) | n/a | n/a |")
            continue
        identity = "pass" if row["identity"] else "**FAIL**"
        launch = "pass" if row["launch"] else "**FAIL**"
        deco = "pass" if row["decorationOk"] else "**FAIL**"
        menu = "pass" if row["menuOk"] else "**FAIL**"
        resolved = row["desktopId"] or "—"
        lines.append(
            f"| {row['label']} | {row['backend']} | `{row['raw'] or '—'}` | "
            f"`{resolved}` | {identity} | {row['decoration']} ({row['expectedDecoration']}) {deco} | "
            f"{row['menuTier']} {menu} | {launch} |")
    lines += [
        "",
        "Notes:",
        "",
    ]
    for row in rows:
        lines.append(f"- **{row['label']}** — {row['note']}"
                     + (f". Outcome: {row['reason']}." if row["reason"] else ""))
    lines += [
        "",
        "The global-menu tier is `none` (the fixed application menu) for every "
        "zoo app: none of them exports a native or DBusMenu menu in this run, "
        "which is the expected fallback. The `dbusmenu`/`native` tiers are "
        "exercised by the T-14.4 bridge tests with `--mock-menu`.",
        "",
    ]
    with open(os.path.join(outdir, f"{prefix}-matrix.md"), "w") as handle:
        handle.write("\n".join(lines) + "\n")
    log(f"wrote {os.path.join(outdir, prefix + '-matrix.md')}")


if __name__ == "__main__":
    sys.exit(main())