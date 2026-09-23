---
name: change-flow
description: >-
  Plan-then-implement workflow for this repo. Use before writing or editing
  code, when the user asks to add, change, fix, or refactor anything, or when
  a change could touch more than one file.
---

# Change flow

Do not code immediately. Plan first, get agreement, then a minimal diff.

## 1. Understand

- State assumptions. If the task has more than one reading — show them and ask.
- Do not guess APIs, seed, LOD, or type names. Read the code.
- Do not propose a new package. Solve it on the existing stack first.

## 2. Show the plan (before writing anything)

```
Goal:
Steps:
  1. … → verify: …
Files to change:
Files I will not touch:
Success criteria:
Risk (determinism / LOD / UI↔world types):
```

Wait for OK unless the user already said to implement / add / just do it.

## 3. Implement

- Edit only what the task requires.
- Match the file's style. Do not reformat neighbors.
- No abstractions for a one-off use case.
- Comments only when intent is not obvious from the code (English).
- Leave no mess: unused import/var/param/type/function/file, commented-out code, debug `console.log` / `println!`, "later" `TODO`, `#[allow(dead_code)]` instead of deletion.
- Do not create helpers or exports that are unused in the same diff.

## 4. Verify

Before finishing, walk **your own** diff:

1. Every new symbol is used. Every removed call leaves no orphans.
2. Lint touched files — fix warnings you introduced (`unused`, `dead_code`).
3. Do not touch pre-existing dead code that your change did not cause.

| Change | Check |
| --- | --- |
| `src-tauri/src/world/**` | seed `6` = Ellipse; other `seed % 6` keep the same profile; `cargo test --lib` in `src-tauri` if the touched module has tests |
| UI (`app.rs` / `render.rs` / `colors.rs`) | global → region → chunk in the egui window (`cargo run` in `src-tauri`) |
| docs/rules only | do not run the app |

After the change: briefly what was done, what was left alone, how it was verified. Do not finish with unused warnings in your own diff.
