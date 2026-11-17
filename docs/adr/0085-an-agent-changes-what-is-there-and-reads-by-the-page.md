# 0085 — An agent changes what is there, and reads by the page

Date: 2026-11-17
Status: Accepted

## Context

An agent outside the window proposes and a person keeps (ADR 0018). What it
could propose was additions. `propose_note` made a note; a second version of
a thought made a second note. `propose_words`, given a word the record
already kept, added a strictness only where there was none and forms to the
ones there were, and dropped a new strictness, a kind, a topic and a note
without a word - while the answer said "proposed". A word entered wrongly,
or two words entered as one ("spacesuit, isotope"), could not be mended from
outside at all.

Reading had the opposite trouble. `notes`, `register`, `trials` and
`catalogue` answered with everything: a record of two thousand words, a bank
of fifteen hundred notes with their bodies, into a context that has to hold
the whole conversation.

## Decision

**A proposal may change what is there, and says what it changes.** A note is
named by id or exact title and the proposal carries what the note held when
it was made (`before`), so the person reads was against becomes; keeping it
writes only what the proposal says. A word the record keeps is read against
the record when the proposal is made, and the item carries its `changes` -
field, was, becomes. Nothing is written until it is kept.

**Notes come as one proposal of several**, kept whole or note by note
(`note:N`), as words and a package for the canon are. The single plain call
- a body, a title, a kind - stays the proposal it was.

**A word can be taken out and split.** `delete` sends it to the trash;
`split` makes its parts, each with the one's kind, strictness, topic and
note and the works named for it, and sends the one to the trash. Both are
one unit of work, so one undo (ADR 0082).

**What is kept in part is said.** Each field of a known word is written by
itself; one the register refuses is returned in `Outcome.unapplied` with the
word, the field and why, and the window says it beside "kept".

**One word is one thing a text can say.** A new word written with a comma or
a slash is kept as proposed, with a warning - in the proposal, in the tool's
answer, and under the word's box in the window.

**The long readings come by the page.** `limit`, `offset`, and `total`,
`has_more` in the answer; `summary: true` leaves the long texts out and says
how long they are; one entry is then read whole by its id or its name. A
page holds 500 at most, whatever is asked.

## Consequences

- The four reading tools answer with an object where they answered with a
  list. There are no clients to keep the old shape for.
- A change is shown against the note as it was when proposed. A note edited
  in between is overwritten by a kept change in the fields the change names;
  the trash and undo are the way back.
- `origin_work` on a note waits for the column (the stage "Documents and
  links"); `propose_style` and the motion bricks are FR-08 8.1, with the
  style packs.
