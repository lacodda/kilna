# 0045 — Material is spent, not moved

Date: 2026-09-30
Status: Accepted

## Context

The predecessor had a bank of ideas: well over a thousand of them, most of
them phrases - single lines worth keeping, "a lighthouse keeps the hours
nobody asked for". It was frozen on its third day and never edited again.
The diagnosis was not that nobody wanted a bank; it was the form:

- **A phrase was a card.** A page of cards, each holding one line, is a wall;
  a thousand lines have to be read as lines, one under another.
- **Nothing said what had been used.** Every idea stayed "new" for ever.
  "Became a song" was a column planned and never written, so the bank could
  not answer the only question a bank is for: what is still there to use?
- **Ideas were a subsystem of their own**, beside notes, lore and characters,
  each with its own screen. kilna already decided the other way (ADR 0025):
  one note, with a kind.

kilna's notes had one way from a note to a work: "make it a work", which
moves the body into the work's first version and puts the note in the trash,
so the same text is not kept in two places (ADR 0001). For an idea that grew
into a song that is right. For a phrase it is wrong: the phrase is a line
that went into a song, and the bank has to remember that it went, or it will
be used again.

## Decision

**Ideas and phrases are notes**, of kinds the profile names - `idea` and
`phrase` in every shipped profile. Two flags on a note kind say what is
different about them:

- **`material`** - notes of the kind are what works are made from, and are
  spent by them. A material note carries a **state**: `fresh`, `used`,
  `parked` (set aside for later) or `dropped` (given up on). The words are
  the code's, because the code sets one of them: sending a note to a work
  makes it `used`.
- **`line`** - a note of the kind is one line. The Notes screen shows such a
  kind as rows of a table, one line each, with its state, its tags and the
  work it went into - not as pages in the list beside the other notes, and
  not under "All kinds".

**Material is spent, not moved.** "To a work" ties the note to the work -
its `work_id` - and marks it used, in one gesture; the note stays in the
bank and shows on the work's Notes tab. "Make it a work" on a material note
makes the work from the note's text and leaves the note in the bank, used
and tied to the new work. A plain note still moves, as ADR 0001 has it.

`state` is a column on every note. Only the material kinds show it; a plain
note is fresh for ever, which says nothing and harms nothing.

## Consequences

A phrase in the bank and the line in the song it went into are two texts
that happened to start equal: the song's line is edited, the phrase is the
record of what was taken. That is the one duplicate this allows, and it is
the point of the bank.

Undoing "make it a work" on a material note throws the work away and puts
the note back as it was - fresh, tied to nothing - rather than out of the
trash, where it never went.

A work thrown away takes the notes tied to it along, material or not, and
brings them back when it is restored: a phrase used in a song that no longer
exists is in the trash with the song.

The predecessor's bank is brought in by the import from atlas: phrases as
phrases, every other idea as an idea, its words as tags, its state as the
state.

Rejected: a table of ideas beside the notes (the subsystem the mill
retired); keeping "used" as a tag (a tag cannot be set by a gesture and read
back as a fact); moving a phrase into the work like any note (the bank would
forget it was spent - the failure it exists to prevent); a state per kind
named by the profile (the code sets `used`, so the word cannot be the
profile's).
