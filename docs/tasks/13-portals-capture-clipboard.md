# T-13 — Portals, Screenshot/Recording, Clipboard, Auth

> **Track, not a single slice.** This file is the design reference. It is executed as 12 one-session units: [T-13.1a](units/091-t-13.1a-portal-backend-and-session-service.md) · [T-13.1b](units/092-t-13.1b-settings-and-globalshortcuts-portals.md) · [T-13.2a](units/093-t-13.2a-filechooser-portal.md) · [T-13.2b](units/094-t-13.2b-filechooser-picker-ui.md) · [T-13.3a](units/095-t-13.3a-screenshot-portal-and-selection.md) · [T-13.3b](units/096-t-13.3b-screenshot-save-copy-and-gate.md) · [T-13.4a](units/097-t-13.4a-screencast-portal-and-picker.md) · [T-13.4b](units/098-t-13.4b-screencast-stream-and-fallback.md) · [T-13.5a](units/099-t-13.5a-clipboard-round-trips.md) · [T-13.5b](units/100-t-13.5b-clipboard-history.md) · [T-13.6](units/101-t-13.6-polkit-authentication-agent.md) · [T-13.7](units/102-t-13.7-flatpak-validation-and-capture.md). Strict order and prerequisites live in [ROADMAP.md](../ROADMAP.md).

| | |
|---|---|
| **Slice** | 13 of 17 — the sandbox and capture boundary |
| **Area** | `portal/` · `shell/screenshot/` · clipboard/auth services |
| **Depends on** | T-10, T-12 |
| **Blocks** | T-14 (Flatpak zoo), T-15, T-17 |
| **Legacy detail** | [legacy/27-portal-backend.md](legacy/27-portal-backend.md) · [legacy/28-screenshot-recording-ui.md](legacy/28-screenshot-recording-ui.md) · [legacy/29-clipboard-auth-agent.md](legacy/29-clipboard-auth-agent.md) · [07-system-integration.md](../design/07-system-integration.md) |

## Demo

```
a Flatpak browser runs in the Dragonfruit session
→ Open File → the Dragonfruit file chooser (files-core), not a portal error
→ screenshot (Cmd+Shift+3/4) → the capture UI; save/share
→ screen share in the browser → the Dragonfruit screencast picker; the stream
  is visible
→ copy text, an image, and a file list from a native app into the browser and
  back
→ a privileged operation raises the Dragonfruit polkit auth agent
```

Capture: `docs/captures/t13-portals.*` (the browser walkthrough).

## Why now

Portals are the compatibility boundary: without them, Flatpak and most modern
browsers are broken, which makes the desktop unusable for real daily work.
They also depend on Files (FileChooser) and the session (screen share), so
this is the first slice after both exist. The clipboard and auth agent share
the same "host stack, our presentation" principle and land together.

## Inherited and reused

- The private capture gate: **no screencopy-style grabs, ever** (CI gate from
  the legacy T-02 work).
- The T-07 private protocols and T-12 session for the backend.
- files-core (T-10) for FileChooser.
- Design-system components for the pickers/overlays.
- `docs/private-protocols.md` and the IPC versioning policy.

## Scope

### In

1. **xdg-desktop-portal-dragonfruit backend**: FileChooser, Screenshot,
   ScreenCast, GlobalShortcuts, Settings, and the minimum of the rest the
   desktop needs; a real session-bus service; graceful behavior when the
   portal frontend is absent.
2. **Screenshot/recording UI**: the shortcut path, region/window/fullscreen
   selection, save/copy, and screen-share source pickers; the compositor-side
   capture path stays portal-only.
3. **Clipboard**: text, images, and file lists (`text/uri-list`) through the
   data-device; a clipboard history manager if the design calls for it, with
   the same source-of-truth discipline.
4. **Auth agent**: polkit authentication agent rendering prompts in our
   shell; no custom privilege escalation.
5. **Flatpak/browser validation**: a real Flatpak app performs all of the
   above in the T-12 session.

### Out / explicitly deferred

- Recording editing/trimming, streaming integrations.
- Per-surface dmabuf feedback tranches (can land here if needed for zero-copy;
  otherwise post-gate).
- Additional portals beyond the desktop's needs.

## Acceptance

- [ ] The demo runs in a real session and the capture is committed.
- [ ] FileChooser, Screenshot, ScreenCast, and clipboard round-trips pass with
      a Flatpak app.
- [ ] The capture-grab gate (`make check-no-capture-grab`) stays green: no
      non-portal grabs.
- [ ] The polkit agent handles a real privileged request.
- [ ] `make e2e` and `make soak` stay green; previous demos still pass.

## Test plan

- Unit: portal request/response models, mime handling.
- Integration: D-Bus portal calls with a test client; clipboard mime
  round-trips in both directions.
- Nested/session: Flatpak walkthrough capture.
- Security: confirm no capture path bypasses the portal.

## Risks

- **Portal breadth** can sprawl; ship the desktop's needs and keep the
  interface additive.
- **PipeWire screencast** is the hardest piece; timebox and fall back to
  stills-only if needed, recording the gap.
- **Clipboard ownership** must stay with the existing data-device bridge; do
  not add a second owner.

## Hand-off

- T-14's compatibility zoo uses the portals.
- T-15's privacy/security panes reference the portal settings.
- T-17's gate includes a Flatpak browser.
