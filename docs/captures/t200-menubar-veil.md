# T200 — menu-bar veil live capture

Nested compositor, replacement rail, deterministic checker wallpaper. Two X11
windows sit directly under the 95 pt menu-bar veil: a black window at x≈100
and a white window at x≈900. The black window exposes the measured depth LUT;
the white window shows that the Light white lift is invisible over white.

| depth pt | capture | measured | delta |
|---|---|---|---|
| 5 | 0.4510 | 0.4510 | +0.0000 |
| 15 | 0.4392 | 0.4392 | +0.0000 |
| 25 | 0.4039 | 0.4078 | -0.0039 |
| 35 | 0.3529 | 0.3569 | -0.0039 |
| 50 | 0.2392 | 0.2431 | -0.0039 |
| 75 | 0.0588 | 0.0588 | -0.0000 |
| 90 | 0.0078 | 0.0078 | -0.0000 |
| 95 | 0.0000 | 0.0000 | +0.0000 |

White window invisible: True

Artifacts: `t200-menubar-veil-{light,dark,fallback}.png` plus the
matching `query scale` / `query material` text dumps.
