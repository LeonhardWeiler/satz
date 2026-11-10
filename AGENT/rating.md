# UI rating

A rater sees only this file and the screenshots, never the goal or earlier scores.
Shots come from `pnpm exec playwright test -c shots/playwright.config.ts --project dark`
in `web/`, into `/tmp/satz-shots/dark/`. The criteria change only when scores
stagnate, and then only to ask more of the UI; every change is listed under Revisions.

## Shots

1. `01 App shell/01 app, nothing selected.png`
2. `04 Canvas/02 selection.png`
3. `04 Canvas/30 text frame selected.png`
4. `05 Properties/12 text frame.png`
5. `05 Properties/11 frame.png`
6. `05 Properties/15 two shapes (boolean bar, mixed values).png`
7. `06 Popovers & menus/01 color picker, RGB.png`
8. `07 Text/05 type options.png`
9. `02 Dialogs/00b projects with a recent document.png`
10. `02 Dialogs/01 new document, first launch.png`
11. `09 Pages & masters/10 page overview.png`
12. `10 Preflight & export/05 export as PNG.png`

## Prompt

You rate the screenshots of a desktop app's user interface, a layout and typesetting
tool. Rate the interface only, not the document shown on the canvas. Look at every
shot. Score each criterion from 0 to 10, then the whole from 0 to 10. 10 is
flawless; 9 means a demanding product design lead would ship it with only nitpicks;
7 is a competent professional tool; 5 is usable but noisy. Be strict: a score is
earned by what is on screen, not by effort; do not round up.

1. Calm: how little competes for attention at once. Every visible control, label,
   line, icon and box costs; count what could be hidden until needed.
2. Order: whether related things sit together, in a logical sequence, and unrelated
   things are clearly apart. Would a newcomer find a setting where they look first?
3. Economy: no control, label or section that repeats another, says the obvious or is
   rarely used yet always visible.
4. Hierarchy: one clear focal point per area; titles, values and labels are
   distinguishable at a glance without boxes around everything.
5. Rhythm: alignment to shared edges, consistent spacing, consistent heights and
   widths of rows and fields.
6. Consistency: the same thing looks and behaves the same everywhere (fields, menus,
   buttons, icons, titles, selected states).
7. Clarity: every icon and label is understandable without guessing; states (selected,
   disabled, mixed, error) are unambiguous.

Answer in this form, in English, nothing else:

```
scores: calm N, order N, economy N, hierarchy N, rhythm N, consistency N, clarity N
overall: N
findings:
1. [shot numbers] what is wrong, and what the fix should be
... (8 to 12 findings, most harmful first, concrete: name the control or area)
```

## Revisions

- v1, 10 November 2026: first version.

## Rounds

| Round | Rubric | Overall | Lowest criterion |
|---|---|---|---|
| 1 | v1 | 6 | calm 5, economy 5, clarity 5 |
