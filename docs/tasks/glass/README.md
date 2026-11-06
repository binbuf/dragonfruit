# Dragonfruit Liquid Glass

- [Symphony roadmap](plan/ROADMAP.md)
- [Workstation evidence and Dragonfruit integration map](plan/REFERENCE.md)

This task set targets Dragonfruit's Rust/Smithay compositor and Qt Quick shell. It starts from the presentation and material work in v0.1.1, including the region cache and the recent Dock fixes. The implementation sequence is T194–T208. Keep the existing look available while developing the replacement. T208 owns default enablement after nested and native acceptance.

## Run with Symphony

The local harness reads `.symphony/symphony.config.json`, which is git-ignored. In another checkout, add this object to that file's `taskSets` array. The plan uses the checked-in design docs and keeps its own roadmap, progress notes and run logs under `plan/`.

```json
{
  "name": "glass",
  "docs": "docs/tasks/glass/plan",
  "tasks": "docs/tasks/glass",
  "design": "docs/design",
  "adr": "docs/design/adr"
}
```

```sh
./.symphony/symphony lint --set glass
./.symphony/symphony status --set glass
./.symphony/symphony run --set glass
```

Run from the repository root. Symphony follows roadmap order. Dependencies in the task files also apply if selecting a subset. The verify commands for implementation tasks are hardware-independent. Capture and hardware requirements live in the task acceptance criteria. The project-wide `.stop` sentinel also applies to this set.

## Reference checkout

Use the sibling `macos-liquid-glass` checkout at commit `063a473cf06d0dba5cb83bd56c40b6fc88df2931`. T194 records the paths and hashes it actually uses and defines an explicit reference-root override for other checkout layouts. The production build and ordinary gates must work without the sibling checkout. The capture files are for local comparison. Use Dragonfruit assets and synthetic test fixtures in the product and its tests.
