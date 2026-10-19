---
title: Notes
description: Two shelves of things worth keeping - phrases kept as rows on shelves you name, and notes written as pages - with tags, checklists, and a way to grow a note into a work.
---

A note is anything worth keeping that is not a work: a line you may sing one
day, a reference, a list of things to check. kilna keeps them as one kind of
thing with a **kind** rather than a separate place for each. The kinds are
words of the craft and live in the
[profile document](/kilna/reference/profile-document/), so a craft that works
differently names its own.

A kind the profile gives sections is a kind of **card**: the characters,
places and lore of the Studio profile are cards of the
[canon](/kilna/guides/the-canon/), kept on the Canon screen as facts, with
the note as each card's free text. The Notes screen keeps the rest.

Since 0.98 the rest is **two shelves**: phrases and notes. See
[ADR 0064](https://github.com/lacodda/kilna/blob/main/docs/adr/0064-two-shelves.md).

## The Notes screen

**Notes** in the menu (or `G` `N`) opens the screen. Along the top is one chip
for each kind of note the profile names, and one chip is always on. In every
shipped profile the chips are **Phrase** and **Note**; the blog profile also
has **Source**. There is no *All* chip: you are always on one shelf.

The kind on show is part of the address: `/notes?kind=phrase` and
`/notes?kind=note`. `/notes` opens on the first kind, Phrase. A note opened by
address, `/notes/<id>`, opens on its own kind.

The **Note** chip shows a list on the left and the open note on the right.

- **The search box** finds notes by any word of the title, the text or whose
  words it is.
- **The tag picker** narrows the list to one tag. Clicking a tag on an open
  note does the same.
- **+ Note** makes a new note and opens it ready to type.

The open note is part of the address, so `Alt+←` walks back through the
notes you opened, and a search hit from `Ctrl+K` lands on the note itself.

### Tags

With a tag picked in the head, a menu beside the picker offers **Rename the
tag**, on every note that carries it, and **Take it off every note**. Each is
one gesture and one undo (`Ctrl+Z`).

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

Any note or phrase can be **about a work**, or kept *for* it. The line under
the title says which; click it to open the work, or **×** to let go.
**Attach to a work** picks one from the catalogue. Attaching does not spend
the note: a note is never spent by works, and a phrase stays fresh. A work's
own **Notes** tab lists the notes and phrases kept for it, and each has a
button that opens it here. A phrase there shows its kind chip, its state when
it is not fresh, and whose words it is, as *— Old Joy*. **Open in Notes** on
a phrase opens the phrases screen.

### Making a work of a note

**Make it a work** is on a note, and in the menu of a phrase. You choose the
kind and the name - the note's title, or its first line, is offered - and a
work is made.

**The note stays, attached to the work.** It is what the work grows from.
**Nothing is copied**: the work starts with no version, and you write the
first one yourself. A phrase made a work is also marked used. **Undo** (or
`Ctrl+Z`) takes it all back.

### Ideas

There is no kind of note called *idea* any more. Every idea became a plain
note tagged `idea` when 0.98 first opened your workspace. The tag is the
English key `idea`; rename it from the tag menu if you want another word.

## Phrases

The **Phrase** chip opens the phrases screen: shelves on the left, phrases on
the right, one row each. A phrase is one line - a word, a quote, a title - and
a thousand of them are read as lines, not as pages.

### Shelves

The left side lists **All phrases** and **On no shelf**, each with a count,
then your shelves in your order.

- Name a new shelf in the field at the foot and press `Enter`.
- Double-click a shelf to rename it.
- Drag its grip, or use **Move up** and **Move down** in its menu, to change
  the order.
- **Delete the shelf** sends it to the [trash](/kilna/guides/the-trash/). Its
  phrases stay.

A phrase may stand on several shelves.

### Keeping phrases

The field at the top keeps phrases. Type a word, a line or a quote and press
`Enter`. Paste a list and each line becomes a phrase; `Shift+Enter` starts a
new line by hand. Nothing is split by commas: *coffee, a pulsar and the sea*
is one phrase.

A line that is already kept - the same words, whatever the case - is not kept
twice. It stays as it stood and joins the open shelf. With a shelf open, new
phrases go onto it.

### A row

Each row shows:

- **The words.** Double-click to edit them.
- **Whose words** they are: a quote's author, a film, a song. Set it from the
  row menu, **Whose words...**; it shows as *- Old Joy*.
- **How its word is sung**, where the record keeps a way of singing it, for
  example *МарсЭль*. See [Singing a text](/kilna/guides/singing-a-text/).
- The other shelves it stands on, and its tags.
- Its state, when it is not fresh: **used**, **set aside** or **dropped**.
- **Sung in N works.** This is read off the works' current texts every time
  you look, the way the [register](/kilna/guides/the-register/) counts. It is
  never stored.
- The work it is kept for, as a link.

### Gestures

- Drag the grip onto a shelf on the left, or use the folder button, **Put on a
  shelf**. The **×** on a row takes it off the open shelf only.
- The link button, **Attach to a work**, keeps the phrase *for* that work. The
  work does not spend it: the phrase stays fresh.
- The menu edits the line, sets whose words it is, attaches it or lets the
  work go, makes a work of it, marks it **used**, **set aside**, **dropped** or
  back to **fresh**, or deletes it.

### The head

The state picker (fresh by default), the search box and the tag picker narrow
the phrases. Search finds by the words and by whose words. The screen shows
the first two hundred rows; **Show more** adds the next.

### Phrases and the guard of repeats

A word is **rare** for the [guard of repeats](/kilna/guides/the-guard-of-repeats/)
when you keep it as a phrase of its own: a phrase that is one word and not
dropped. A phrase of several words makes none of its words rare.

### Found in your texts

Proposals of stresses and respellings your sung texts already write are on the
[Register](/kilna/guides/the-register/#found-in-your-texts) screen.

## For the assistant

An action reads the phrases kept for a work through `{lines}`, or one shelf
through `{lines:<shelf>}`. An agent reads notes with the MCP tool `notes` and
proposes them with `propose_note` and `propose_words`. See
[the profile document](/kilna/reference/profile-document/#template-placeholders)
and [MCP](/kilna/reference/mcp/).
