---
title: Notes
description: Ideas, phrases and references in one place — written on their own or about a work, with tags, checklists, a bank of phrases that knows what was used, and a way to grow an idea into a work.
---

A note is anything worth keeping that is not a work: an idea you have not
started, a reference, a list of things to check. kilna keeps them as one
kind of thing with a **kind** rather than a separate place for each. The
kinds are words of the craft and live in the
[profile document](/kilna/reference/profile-document/), so a craft that works
differently names its own.

A kind the profile gives sections is a kind of **card**: the characters,
places and lore of the Studio profile are cards of the
[canon](/kilna/guides/the-canon/), kept on the Canon screen as facts, with
the note as each card's free text. The Notes screen keeps the rest.

## The Notes screen

**Notes** in the menu (or `G` `N`) opens every note of the profile. The list is
on the left, the open note on the right.

- **The chips** along the top narrow the list to one kind, with how many there
  are of each. Click the chip that is on to turn it off.
- **The search box** finds notes by any word of the title or the text.
- **The tag picker** narrows the list to one tag. Clicking a tag on an open
  note does the same.
- **+ Note** makes a new note in the kind you are looking at and opens it
  ready to type.

The open note is part of the address, so `Alt+←` walks back through the
notes you opened, and a search hit from `Ctrl+K` lands on the note itself.

## Writing a note

The note opens as it reads — markdown drawn, links resolved. **Edit** (or a
double-click on the text) turns it into its text; **Done** or `Escape` turns it
back. The text saves itself a moment after you stop typing; nothing asks you
to press save.

- **The title** is optional. A note without one is listed by its first line.
- **The kind** is picked beside the title.
- **Tags** sit along the bottom. **+ tag** offers the tags already used on
  other notes, so the second note about winter is tagged `winter` rather than
  `Winter`; `Enter` takes whatever you typed as a new one. A note's tags are
  its own vocabulary — the tags on works are a separate list.
- **Links** to a work or a version — `[[work:…]]`, `[[version:…]]` — open
  that work when clicked.

### Pictures in a note

Paste a picture while writing - a screenshot, a picture copied from a page -
or drop a picture file onto the window, and it goes into the text where the
caret stood, on a line of its own:

```markdown
![board](media/5f1c….png)
```

The picture is copied into the workspace like any [attached file](/kilna/guides/files-and-covers/#where-the-file-goes)
and belongs to the note: a backup takes it along, and it goes to the trash with
the note. The text names it by the file it was stored as, relative to the
workspace, so it reads the same in an [export](/kilna/reference/data/), which
carries the pictures in a `media` folder beside the pages. A card of the canon
takes pictures in its note the same way; they stay out of the card's gallery,
which shows what the card *is*.

See [ADR 0058](https://github.com/lacodda/kilna/blob/main/docs/adr/0058-a-picture-in-a-note-is-named-by-the-file-it-was-stored-as.md).

### Checklists

A line written as `- [ ] check the thickness of the layer` is drawn as a
checkbox. Click it and the line becomes `- [x] …` — the text is what is
kept, and the box only edits it. The list shows how far along each note's
checklist is, `2/5`, so a to-do list can be read without opening it.

Checkboxes tick the same way in a work's **Notes** tab.

## A note and a work

A note can be **about a work**. The line under the title says which; click it
to open the work, or **×** to let go. **Attach to a work** picks one from the
catalogue — the way an idea written on its own joins the song it turned out
to be for. A work's own **Notes** tab lists the notes about it, and each has a
button that opens it here.

### Making an idea a work

**Make it a work** turns the open note into a new work. You choose the kind
and the name — the note's title, or its first line, is offered — and the
note's text becomes the work's **first version**, in the kind's first role
that is the work itself (*Lyrics* for a song).

The note does not stay. Its words live in the version now, and keeping both
would leave the same text in two places, one of them edited and the other
not. The note goes to the [trash](/kilna/guides/the-trash/), and **Undo**
(or `Ctrl+Z`) takes the whole thing back: the work goes, the note returns.
Its tags stay with the note; the new work starts with none.

An idea or a phrase is the exception — below.

## Ideas and phrases

An **idea** and a **phrase** are notes of their own kinds, in every shipped
profile. What makes them different is that they are *material*: works are
made from them, and a bank of material is only worth keeping if it knows what
has been used.

- **Where it stands.** An idea or a phrase is **fresh**, **used**, **parked**
  (set aside for later) or **dropped** (given up on, and kept so it is not
  written again). The state is picked beside the title, and the screen opens
  such a kind on its fresh ones — the state picker above the list moves between
  states.
- **To a work.** Where a plain note says *Attach to a work*, an idea or a
  phrase says **To a work**: picking the work ties the note to it and marks it
  used, in one step, and **Undo** takes both back. The note shows on the work's
  **Notes** tab. Letting the work go makes it fresh again.
- **Make it a work.** The text becomes the new work's first version, as for
  any note — but the idea stays in the bank, used and tied to the new work,
  instead of going to the trash. The words in the work will change; the idea
  is the record of what was taken.

### The bank of phrases

A phrase is one line, and a thousand of them are read as lines, not as pages.
The **Phrase** chip opens the bank: a row per phrase — its words, its tags,
the work it went into — and *All* leaves the phrases out, so they never bury
the notes.

- Type a new phrase into the box at the top and press `Enter`.
- The arrow at the end of a row sends it **to a work**; the menu beside it
  edits the line (a double-click does too), makes a work of it, sets it
  aside, drops it, brings it back to fresh, or deletes it.
- The search box, the tag and the state narrow the bank the way they narrow
  the list.

The bank shows the first two hundred rows; **Show more** adds the next.
