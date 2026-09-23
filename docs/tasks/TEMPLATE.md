---
# Optional per-task overrides read by the harness (delete if unused):
# provider: claude | cursor | opencode | codex | gemini | antigravity
# model: <model id>
# variant: high | low | ...   (reasoning effort; only sent when the provider/model supports it)
# timeoutMin: 240
# verify: <shell command the harness runs after this task reports done; non-zero fails the task>
---
# TNN — <Title>

## Goal
One or two sentences: what exists at the end of the session that did not exist before, and why it matters.

## Context (read first)
- `path/to/file.ts:123` — why this file matters
- `<design>/<doc>.md` — the design this task implements

## Scope
- [ ] Concrete, verifiable item

## Out of scope
- Item and the task that owns it (→ TNN)

## Design notes
Decisions the implementer must follow (names, signatures, constraints).

## Done when
- [ ] Tests named here pass, with the commands to run them
- [ ] Docs touched: …
- [ ] Hand-off below filled in

## Hand-off
_(filled in by the implementing session: what landed, what deviated and why, what the next task must know)_
