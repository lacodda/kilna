# 0087 — What ships is given once, and offered after

Date: 2026-11-18
Status: Accepted

## Context

A profile is copied into a space and is its owner's from then on (ADR 0001).
kilna goes on shipping newer versions of what is in it - a reworded action, a
status that learned its colour, a stop that learned its hex, a kind of note
that became a card with sections - and three mechanisms had grown to carry
them, each with its own idea of "untouched":

- the carry-forward's steps (`profile/upgrade.rs`), some forty of them, most
  matching a field against the exact text it shipped with in some earlier
  release, a table of old wordings per step;
- the starter set of the style dictionary (ADR 0048), a fingerprint of the
  entry last written into each brick (`set_digest`) and a tombstone for a
  brick deleted;
- adding entries by key wherever the stored copy lacked them, which also
  brought back every entry its owner had deleted - "absence is absence
  either way", the price the carry-forward paid for not asking.

Every new field shipped meant a new step and a new table of old values, and
an entry the owner had changed never heard of a newer version at all.

## Decision

**One rule for everything kilna ships into a space** (`origin.rs`). Each
shipped entry is known by where it came from (`profile:<key>`, `pack:<id>`
when packs of styles arrive), its key there, and the fingerprint of the
version last given to it. At every start:

- an entry whose content still has that fingerprint is untouched, and takes
  the newer version;
- an entry whose content differs is its owner's: it is never written by a
  start, and a newer version waits beside it to be compared - kept, which
  stops that version being offered, or taken, which is an edit to the
  document like any other: logged, and undone by an undo;
- an entry given and since deleted stays deleted;
- an entry shipped since arrives after the entry it follows in the shipped
  list.

**The profile's lists carried by the rule** are the actions, a kind's
statuses, the stages, the marks, the fields, the kinds of note and their
sections, and the vibe's words (`profile/shipped.rs`). An entry's fingerprint
leaves out the lists inside it that ship on their own: a kind of note renewed
keeps its sections as the space has them.

**What a space was given lives beside the document** (`shipped_entry`), not
in it. The document is the owner's to read, edit, export and pass on as a
type; fingerprints in it would be noise in all three, and a document passed
on would carry another space's history. A row whose entry is absent from the
document is the tombstone of an entry its owner deleted - the entry has no
row of its own to tombstone.

**The first meeting is the initialisation.** A copy stored before the rule
has no fingerprints. An entry first met reading exactly as it ships is taken
as untouched; one reading otherwise is recorded as given nothing - so a later
start never reads it as untouched and writes over it - and the current
version is offered beside it. A list its owner writes as a whole (a kind's
statuses, a kind of note's sections, the vibe's words) met without some of
what ships is taken to have left those out on purpose. The steps that
recognise an exact older wording stay and run after the rule, so an older
copy is brought up to what ships before its owner is asked anything; the two
that only added entries by key are gone, the rule does it and remembers
deletions.

**The style dictionary's starter set keeps its columns** (`set_key`,
`set_digest`, the tombstone) and decides by the same function; its
fingerprint is the same FNV-1a, taken over the brick's JSON as it always was,
so no brick in any space reads as changed by the move.

## Consequences

- A newly shipped field of an entry needs no step: it reaches every entry
  nobody touched, and is offered to the rest.
- An entry its owner changed no longer receives a field filled in behind
  their back; the newer version is shown whole beside theirs instead. The
  steps that filled fields stay for copies older than the rule, and no new
  step is written for a list the rule carries.
- **Settings → Profile** gains **What kilna ships now**: the entries waiting,
  what differs field by field, a text compared line by line, and **Keep
  mine**, **Take the standard**, **Keep all mine**.
- A deleted shipped entry stays deleted; bringing one back is a matter of
  writing it again.
- A device syncing the document computes its own `shipped_entry` from the
  same build and the same document; what one device kept is offered again on
  another.
