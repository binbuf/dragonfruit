# Logs

One markdown file per task (`T01.md`, `T02.md`, …), written by the symphony harness after every
session. Each file is the high-level run log for that task: status, provider/model, timing, cost, commit,
and each session's reported `SYMPHONY_RESULT` status and summary. It is regenerated in place, so it always
reflects the latest state. Do not edit these files by hand; the raw provider streams live under
`.symphony/runs/` (gitignored).
