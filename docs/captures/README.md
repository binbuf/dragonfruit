# Slice captures

Every vertical slice produces a **human-verifiable capture** here — a short
recording plus stills — and reviews it against the design docs and the
design-system gallery goldens. A slice whose capture has not been watched is
not done (see the definition of done in
[`../ROADMAP.md`](../ROADMAP.md)).

Naming convention:

```text
t01-loop-v0.mp4            # the slice's demo recording
t01-loop-v0-wayland.png    # stills for the notable cases
t01-loop-v0-x11.png
t01-loop-v0-csd.png
t02-lifecycle-motion.mp4
t02-lifecycle-motion-reduced.mp4
...
```

Guidelines:

- Capture from the **nested** session for daily review; add a **DRM** capture
  where the slice touches hardware (T-03, T-12, T-16).
- Keep recordings short (30–90 s) and start from a known state
  (`make demo`).
- For material/visual slices, include light + dark and reduced motion.
- Do not commit large files; prefer a few MB per slice and link to longer
  recordings in the PR/issue rather than this folder.
