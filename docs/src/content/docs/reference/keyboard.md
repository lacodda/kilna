---
title: Keyboard
description: Every shortcut the application answers to, what happens while you are typing, and where the arrows belong to something else.
---

Press `?` anywhere to see this list inside the application. It is generated
from the same table the shortcuts are read from, so it cannot fall behind them.

## Going places

Press `G`, then a letter. The letters are the initials of the English screen
names, and they stay the same in every language — a shortcut that moved with
the translation would have to be relearned per language.

| Keys | Screen |
| --- | --- |
| `G` `D` | Dashboard |
| `G` `C` | Catalogue |
| `G` `K` | Calendar |
| `G` `J` | History |
| `G` `N` | Notes |
| `G` `T` | Trash |
| `G` `S` | Settings |

The chord waits about a second and a half for its second key. If none arrives,
or the key goes nowhere, the chord is dropped and nothing happens — a forgotten
`G` never turns a later keystroke into a jump.

## Moving around

| Keys | What it does |
| --- | --- |
| `Ctrl+K` / `⌘K` | Find anything: works, drafts, notes, comments, assistant replies. A note opens on the Notes screen, a comment on the Comments screen |
| `Alt+←` | Back |
| `Alt+→` | Forward |

Back and forward are the browser's own keys, and they work here for the same
reason the address bar would: every screen, every open work and every tab is an
address ([ADR 0006](https://github.com/lacodda/kilna/blob/main/docs/adr/0006-browser-routing-inside-tauri.md)).
Opening a work, switching to its Score tab and pressing `Alt+←` twice puts you
back where you started.

## Everywhere

| Keys | What it does |
| --- | --- |
| `Ctrl+Z` / `⌘Z` | Take back the last thing you changed |
| `?` | This list |
| `Esc` | Close what is open — a dialog, the search box; a text on the whole screen steps back one level |

Undo takes back one thing: the last change to the workspace. Not every change
can be taken back — a plan applied to the whole calendar, a status recomputed
across the catalogue, and other things that touch many rows at once are left
alone rather than half-reversed. When there is nothing to take back, kilna says
so instead of doing something else. Deleting is safe regardless of any of this:
it goes to the [trash](/kilna/guides/the-trash/), which is a drawer, not a
shredder.

You will usually not need the key. Saving an edit puts a line at the corner of
the screen naming what changed, with **Undo** beside it — the same bargain
deleting has made since the beginning: nothing asks you to confirm beforehand,
and one click takes it back afterwards. The keystroke is for when the line has
gone and you have moved on.

## While you are typing

A shortcut must never cost you a sentence:

- **Letter shortcuts stand aside.** `G` and `?` do nothing while the focus is
  in a text field, a select, or anything made editable. They are letters, and
  in a field a letter is a letter.
- **Shortcuts with a modifier keep working.** `Ctrl+K` opens search from inside
  a field on purpose — it is the way *out* of where you are. `Alt+←` walks back
  from inside the version editor for the same reason: `Alt` types nothing, so
  it interrupts nothing.
- **`Ctrl+S` writes the open text now.** The text in a version saves itself
  a moment after you pause; the key is for the hand that presses it anyway,
  and it writes immediately instead of a moment later.
- **`Ctrl+Z` belongs to the field you are in.** It is the one shortcut with a
  modifier that stands aside, because inside a text field it already means
  something — take back the word you just typed. Taking back a saved edit while
  someone meant to take back a typo would be the worst kind of helpful. Step out
  of the field and `Ctrl+Z` is kilna's again.

## The arrows belong to what is under them

Bare arrow keys are never taken by the shell. They belong to whatever has
focus, and several things want them:

- the **version list** walks through revisions with `↑` and `↓`, keeping one
  tab stop for the whole history;
- the **search box** and every menu move their highlight with the arrows, with
  `Home` and `End` jumping to the ends and type-ahead finding an item by its
  first letters;
- the **score scale** takes `←` and `→` to move a mark, with `Tab` between axes;
- a **calendar chip** being carried by keyboard is placed with the arrows, and
  `Esc` puts it back where it was;
- a **frame opened full screen** walks scene to scene with `←` and `→`,
  skipping scenes that have no picture yet, and `Esc` closes it;
- an open **scene row** takes `Ctrl+V` to paste a picture from the clipboard
  straight onto that scene.

## A menu on every row

Wherever a row stands for something — a work in the catalogue, a release on the
card, a chat in the assistant — there are two ways to its actions, and both
reach the same list:

- the **three dots** at the end of the row;
- a **right click** anywhere on the row, or a long press on a touch screen.

The row lights up while its menu is open, so it is clear which of twenty rows
the actions belong to.

Either way the menu is keyboard-operable in full: arrows that wrap, `Home` and
`End`, type-ahead that finds an item by its first letters, and `Esc` to leave
without choosing.

## Picking out a run of rows

In the [catalogue](/kilna/guides/the-catalogue/), `Shift` and a click on a
second checkbox take every row between it and the last one you ticked, in the
order the table is showing them. The run follows the first row's state: shift-
clicking out of a ticked row fills the span, out of an unticked one clears it.
