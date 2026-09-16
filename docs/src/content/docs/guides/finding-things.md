---
title: Finding things
description: One box that searches works, drafts, notes and assistant replies — what it looks in, how it ranks, and why it works in Russian too.
---

`Ctrl+K` from anywhere opens the search box. So does the field in the top bar,
if your hands are on the mouse.

## Before you type anything

The empty box is not empty: it lists the last few works you opened, newest
first. Whatever is being worked on is nearly always one of them, and
recognising a name is faster than typing it — so `Ctrl+K` `↵` reopens what you
had a minute ago.

The list is six long, it belongs to this machine rather than to the workspace,
and a work leaves it when it is deleted. Typing replaces it with real results.

## What it searches

It looks in four places at once, and groups what it finds:

| Group | What is searched |
| --- | --- |
| Works | Titles, the craft fields, and the tags |
| Drafts | The full text of every version, in every role |
| Notes | The body and the title of notes attached to a work |
| Assistant | What was said in chats about a work |

Each hit opens the work it belongs to, on the tab where the hit lives — a line
found in a draft opens Versions, a note opens Notes.

## It is for recognising, not for browsing

The box shows a few hits per group rather than everything that matched. Past
half a dozen the list stops being scannable, and the useful answer is a
narrower query rather than a longer list.

That is also why hits are shown as text rather than as counts: a draft hit is
the line it was found in, with a little of what surrounds it, so you can tell
*this* verse from *that* one without opening either.

## The keyboard is the point

- `Ctrl+K` opens it, from anywhere — including from inside a text field.
- `↑` and `↓` walk the hits, across group boundaries.
- `↵` opens the highlighted one.
- `Esc` closes without going anywhere.

## Case does not matter, in any language

Searching for `гавань` finds **Гавань огней**, and `HARBOUR` finds
*Harbour lights*.

That is worth saying because it is not free. SQLite's *plain* case-insensitive
matching covers ASCII and nothing else — to it, `Г` and `г` are unrelated
bytes. The search index folds in full Unicode, which is why a Russian workspace
is as searchable as an English one.

## A word finds the forms of a word

Typing `холодильник` finds a line that says *в холодильнике*, and `кофе` finds
*кофейня*. Every word you type is matched as a beginning rather than as a whole
word, because there is no stemmer for Russian here and the word as typed is
rarely the word as written.

The same rule is why `light` finds *lights* — and why `lights` does **not** find
*light*. Type the shorter form when you are not sure.

Two words narrow rather than widen: `холодильник кофе` finds the works that say
both, which is the question "where did I write about both of those" and the
reason the box is useful at all.

Punctuation is never syntax. A stray quote, a dash, a colon — they are searched
for as text, or ignored, and never turn into an error you have to decode.

## What it does not do yet

- **No operators.** No `kind:song`, no quoted phrases. One box, one query,
  everything at once. This box is for *going somewhere* — it answers "where is
  that line", across drafts, notes and chats, and every hit opens a work.
  Narrowing a list down to a set of works is a different question, and the
  [catalogue's own box](/kilna/guides/the-catalogue/#narrowing-from-the-box)
  answers it with `status:`, `kind:`, `tier:`, `tag:` and `stage:` — and with
  the same full-text search behind it.
- **A word is matched from its beginning, not from its middle.** `холод` finds
  *холодильник*; `дильник` finds nothing.
- **Nothing outside the active profile.** Switching profiles switches what is
  searchable, like everywhere else in kilna.

The text is kept in an index that is written as you write, rather than read
back on every keystroke, so the answer does not slow down as the workspace
grows. Nothing has to be rebuilt or maintained by hand: editing a lyric updates
what it finds, and deleting a work takes it out.
