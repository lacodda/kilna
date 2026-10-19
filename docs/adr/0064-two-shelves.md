# 0064 — Two shelves

Date: 2026-10-18
Status: Accepted

## Context

By v0.97 the material for works to come lived in four places on the Notes
screen: **notes**, **ideas**, **phrases** and a **bank of words**. In code an
idea differed from a note by one flag (`material`, ADR 0045), and a word of
the bank from a phrase by being a row of another table - a facet of a term
(`term.bank`, ADR 0052), sorted into blocks (`term_block`). The owner could
not tell them apart: "I no longer understand how Notes differ from Ideas, and
Phrases from Words." Their workspace said the same thing in numbers: 507
ideas, almost all seeds of songs; 1,127 phrases, every one a line; 356 notes;
five words in the bank, all gathered for one song that was not in kilna yet.

The next two versions (v0.99 "Conception", v0.100 "Research") prepare a work
inside kilna - a brief, scenarios, sources - and attach notes and phrases to
it, so the shape of that material had to settle first.

What turns up in a head is one of two things: a line - a word, a phrase, a
quote - to use somewhere later, or a thought, from a seed to a written page.

## Decision

**Two kinds of note: a phrase and a note.** Every shipped profile names
`phrase` (material, one line) and `note`. The `idea` kind is retired:
migration 0040 turns every idea into a note tagged `idea` - a tag rather than
a kind, because finding the seeds again is a filter, not a different thing -
and the profile upgrade takes the kind out of an older stored copy
(`RETIRED_NOTE_KINDS`). A note is not spent by works, so a converted idea
stands fresh; one that went into a work stays tied to it. The tag is the
key `idea`, not a word of the owner's language: a migration does not know
the language of the window. Renaming a tag on every note at once is one
gesture (`note.retag`), and that is how it becomes the owner's word.

**A word of the bank is a phrase; a block is a shelf.** A word is only the
shortest phrase. Migration 0040 writes every banked word as a phrase in its
state - the phrase already written in the same words when there is one, so
nothing is kept twice - and every word a block still held after it left the
bank as a dropped phrase. The blocks come across under their own ids as
`shelf` rows; their words as `shelf_note` rows. A shelf holds notes, not
terms, and a phrase may stand on several, as a word stood in several blocks.

**A term keeps what is the record's own.** The `bank` column goes (the table
is rebuilt). A term that was only kept - no strictness, no singing, no forms,
no topic, no note, no work named for it - has said everything as a phrase and
is deleted; any other stays a term without its bank. How a phrase's word is
sung is read from the term of the same words: one word, one record still
(ADR 0052), now between a phrase and a term.

**What a phrase row says is read, not stored** (`lines::rows`): its shelves,
how many works say it now - every phrase matched against the works' current
texts in one walk, by stems, as the register counts (ADR 0044) - and the
term's way of singing it. A phrase may say whose words it is: a new column,
`note.attribution`, searchable and clocked like the others.

**A phrase or a note attaches to a work without being spent.** `work_id` says
what it is kept for; `state` says where it stands - two facts, two columns,
written by two gestures.

**Making a work of a note copies nothing.** The work is made, the note stays
tied to it as what the work grows from, and a phrase is marked used. Before,
the note's text became the work's first version and a plain note went to the
trash: the seed of a song ended up as its lyrics, and the record of where it
came from was gone. Undo still takes the whole gesture back, and reads a
promotion logged before v0.98 the old way.

**MCP speaks the same two shelves.** `workspace` names the note kinds (a
dangling reference before: `notes` said "see `workspace`: `note_kinds`" and
`workspace` had none); `notes` returns a phrase with its shelves, `sung_in`
and `sung`, and narrows by `shelf`; `propose_note` takes a `kind`,
`attribution` and `shelf`; `propose_words` keeps a phrase (`phrase`,
`shelf`) where it kept a word of the bank. The `words` tool is gone. In a
template, `{lines}` is the phrases kept for the work and `{lines:<shelf>}`
one shelf's, in place of `{words}`.

## Consequences

- The Notes screen has one chip per kind, one always on, the kind in the
  address (`/notes?kind=phrase`). The phrases are one screen with the
  shelves on the left; the bank's screen and the register's "In the bank"
  facet are gone, and "Find in my texts" - proposals of how words are sung -
  moved to the register, where the singing lives.
- What the trash held is rewritten by the migration: a term without its
  bank, a block as a shelf without its words (they are phrases now, on the
  shelf by themselves), an idea as the note it would be.
- Operations logged before v0.98 under `block.*` are no longer replayed or
  undone: the rows they named are gone. `note.promote` keeps reading both
  shapes.
- An imported bank of ideas from the predecessor arrives the same way: its
  phrases as phrases, everything else as notes, the seeds tagged `idea`.
