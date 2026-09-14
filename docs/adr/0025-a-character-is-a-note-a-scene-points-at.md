# 0025 — A character is a note, and a scene points at it

Date: 2026-09-14
Status: Accepted

## Context

"Every scene with her in it" is a question a storyboard has to answer, and
free text cannot answer it: a hero named three ways across fifty
descriptions is three heroes to anything that reads them. The predecessor
had a `characters` table with twelve rows and no edits in its whole life —
the 2026-08-10 review read that as a dead subsystem, and the mill of
2026-09-10 read it better: the rows were never edited because there was no
screen to edit them on. So the need is real and the old shape was not.

Two questions followed. What a character *is* — a table of its own, or
something the application already has. And how a scene holds one.

## Decision

**A character is a note of the kind `character`.** Notes are already "ideas,
lore, reference — one type distinguished by `kind` and tags"; a character is
that with a screen of its own, which arrives in v0.73 and edits this same
note (decision of 2026-09-11: one entity, two stages). A second table for
people would be the duplicate ADR 0001 forbids, and would leave the
character's body — who she is, how she looks — in a note beside it.

**The kinds a note may take are the profile's words**, in a new optional
`note_kinds`: Studio and Novel name a character, a location, lore and a
plain note; Podcast names a guest and a segment; Blog names a source. A
craft decides what kinds of thing it writes down, the way it decides its
kinds of shot. A profile naming none has not decided, and any note goes —
the leniency a kind with no `shot_types` gets.

**A scene points at notes through `scene_note`**, a row per (scene, note).
No role column on it: what a note *is* is its kind, and a second word for it
here would be a place for the two to disagree. The kind is checked on write
rather than in the schema — the vocabulary changes while the workspace
lives, and a database that refused a note whose kind was renamed yesterday
would lose the link rather than the word.

**Naming the same note twice is the same link**, not an error and not a
second row: a person naming her again means she is in the scene, which she
already was.

The links travel with the trash in both directions: deleting a scene takes
what it was about, deleting a character takes her out of the scenes, and
restoring either puts the references back — captured the way a work's links
are (ADR 0019). A work's capture reaches them through its board, which is
why a capture's key may now be a whole condition rather than a column name.

Rejected: a `characters` table (above); a text field on the scene holding
names (the three-heroes problem, which is the whole reason for this);
a role on the link (the kind already says it); checking the kind in the
schema (the renamed-vocabulary problem above).
