// SPDX-License-Identifier: MIT
//
// Minimal Electron client for the T-14.6a strange-app zoo. Electron apps are
// the common "portable" identity case: the Wayland `app_id` (or X11
// `WM_CLASS`) is the binary basename, and the `.desktop` entry has no
// `StartupWMClass`, so app-index resolves it through the executable-basename
// heuristic. Run with an Electron binary supplied by the zoo script:
//
//   electron --no-sandbox --ozone-platform=wayland --class=electron \
//     scripts/zoo/electron/main.js
const { app, BrowserWindow } = require("electron");

function createWindow() {
  const win = new BrowserWindow({
    width: 420,
    height: 280,
    title: "Electron Zoo",
  });
  win.loadURL(
    "data:text/html," +
      encodeURIComponent(
        '<body style="font-family:sans-serif;margin:24px">' +
          "<h1>Electron zoo</h1><p>A Chromium/Electron client.</p></body>"
      )
  );
}

app.whenReady().then(createWindow);
app.on("window-all-closed", () => app.quit());