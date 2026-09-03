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
| `G` `T` | Trash |
| `G` `S` | Settings |

The chord waits about a second and a half for its second key. If none arrives,
or the key goes nowhere, the chord is dropped and nothing happens — a forgotten
`G` never turns a later keystroke into a jump.

## Moving around

| Keys | What it does |
| --- | --- |
| `Ctrl+K` / `⌘K` | Find anything: works, drafts, notes, assistant replies |
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
| `?` | This list |
| `Esc` | Close what is open — a dialog, the search box, a full-screen editor |

## While you are typing

A shortcut must never cost you a sentence, so the rule is simple and it has no
exceptions:

- **Letter shortcuts stand aside.** `G` and `?` do nothing while the focus is
  in a text field, a select, or anything made editable. They are letters, and
  in a field a letter is a letter.
- **Shortcuts with a modifier keep working.** `Ctrl+K` opens search from inside
  a field on purpose — it is the way *out* of where you are. `Alt+←` walks back
  from inside the version editor for the same reason: `Alt` types nothing, so
  it interrupts nothing.

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
  `Esc` puts it back where it was.

## A menu on every row

Wherever a row stands for something — a work in the catalogue, a release on the
card, a run in the assistant — the three dots at its end open the same menu.
It is keyboard-operable in full: arrows that wrap, `Home` and `End`, type-ahead,
and `Esc` to leave without choosing.
