#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""A continuously scrolling X11 client for the T238 material workload bench.

The compositor's scene cache must invalidate on every client commit. This
window scrolls a large text buffer at ~60 Hz, producing a steady stream of
buffer commits through Xwayland while the desktop is otherwise static.

Usage: t238-scroll-client.py <duration-ms>
"""
import sys
import tkinter as tk


def main():
    duration_ms = int(sys.argv[1]) if len(sys.argv) > 1 else 5000
    root = tk.Tk()
    root.title("T238 Scroll")
    root.geometry("520x360+320+220")
    text = tk.Text(root, width=80, height=24, wrap="none")
    text.pack(fill="both", expand=True)
    for index in range(4000):
        text.insert("end", f"line {index:05d}  T238 continuous scroll payload\n")
    text.see("1.0")

    state = {"direction": 1}

    def tick():
        text.yview_scroll(state["direction"], "units")
        first, last = text.yview()
        if last >= 0.999:
            state["direction"] = -1
        elif first <= 0.001:
            state["direction"] = 1
        root.after(16, tick)

    root.after(16, tick)
    root.after(duration_ms, root.destroy)
    root.mainloop()


if __name__ == "__main__":
    main()
