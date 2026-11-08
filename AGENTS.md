# Agent instructions

## Git staging and commits

Before every `git add` or `git commit`:

1. Inspect the whole working tree, including untracked files and directories:

   ```sh
   git status --porcelain=v1 --untracked-files=all
   ```

2. For each new file or directory, decide whether it is **source** or
   **generated/local-only**.

3. If it is generated or local-only, add an ignore rule instead of staging it:

   - Repo-wide, shared rule → `.gitignore` (match the existing section and
     comment style).
   - Personal or tool-specific rule → `.git/info/exclude`.

4. Verify the rule matches and the path disappears from status:

   ```sh
   git check-ignore -v <path>
   git status --short
   ```

5. Stage explicit paths. Do **not** use `git add -A` or `git add .`. Use
   `git add -u` for tracked edits, then `git add <path>` only for new files
   that are intentionally part of the change.

6. Review exactly what will be committed:

   ```sh
   git diff --cached --stat
   git diff --cached
   ```

7. Never commit secrets, credentials, tokens, private keys, or personal data.
   If one is already tracked, stop and flag it rather than committing around
   it.

## Paths that stay out of git

Treat the following as generated/local-only unless there is a deliberate,
reviewed reason to version a specific file:

- **Screenshots, captures, recordings**: `*.png`, `*.jpg`, `*.jpeg`,
  `*.webp`, `*.gif`, `*.mp4`, `*.webm`, and evidence/capture directories such
  as `docs/captures/`. This repository's history was scrubbed of these; do not
  reintroduce them.
- **Logs and traces**: `*.log`, `*.trace`, `*.jsonl`, agent/run logs, and
  `docs/logs/`.
- **Build outputs**: `build/`, `target/`, `dist/`, `out/`, `*.o`, `*.so`,
  `*.dylib`, `*.a`.
- **Caches and local state**: `.cache/`, `.local/`, `*.tmp`, `*.swp`, `*~`.
- **Editor and OS files**: `.DS_Store`, `Thumbs.db`, `.idea/`, `.vscode/`.
- **Local harness/session artifacts**: `harness/`, `.symphony/`, `.stop`.
- **Downloaded or vendored binaries/archives** unless they are intentionally
  vendored as source.
- **Anything containing a username, hostname, IP/MAC address, serial, SSID, or
  absolute home path.**

### Exceptions — these are checked-in on purpose

Do not blanket-ignore media that is a real project asset or a test fixture:

- `assets/` (icons, cursors, fonts, wallpapers)
- `design-system/gallery/snapshots/` (golden gallery references used by
  `scripts/check-gallery-snapshots.py`)
- `tests/glass/fixtures/` (small PNG fixtures)

When in doubt, leave the file out of the commit and ask.
