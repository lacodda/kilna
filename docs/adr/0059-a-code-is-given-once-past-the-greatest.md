# 0059 — A code is given once, past the greatest one of its shape

Date: 2026-10-09
Status: Accepted

## Context

ADR 0057 finds a work's folder on disk by a name its kind's template makes:
`songs/{code}` reads a field of the song. A person who already keeps their
media by code - a folder per song named after a catalogue number - points the
template at that field, and every work with a code finds its folder. Every
work without one says the field is empty: the songs made since the codes
were last given out by hand, and every new song from then on.

The question put to the owner (2026-10-08) had three answers: kilna gives a
new work the next code; a work without a code falls back to its title; or
the person types the code in. The owner chose the first.

## Decision

**A field numbers by an example.** `numbered_from` on a `text` field of
`work_meta_fields` is the first code, standing for all of them: `CAT-001` says
the letters before the number, the width the number is padded to, and where
counting starts. A field of another type, an example that does not end in
digits and a `default` beside the numbering are refused by the profile's
validation - a default and a numbering would be two answers to what a new
work starts with.

**A code is written into the work when it is made.** `actions::work::create`
gives each numbered field of the new work's kind the next code, before the
work is logged, as it fills in the defaults: the log carries the code, so a
replay writes the code the work was given rather than counting again over a
workspace that has moved on. A work that arrives with a code - carried by an
import, proposed by an agent that named one - keeps it.

**Next means past the greatest, never into a gap.** The next code is one past
the greatest value of the field's shape, read off every work of the workspace
and every work in its trash (`deletion` snapshots keep the fields). A code is
never derived from the work's place among the others: a number worked out
from an order would change when a work before it was deleted, and every
folder named after it would be lost. A gap left by a deleted work stays a
gap; a work put back from the trash finds its code still its own. A code
given by hand above the count moves the count past it.

**The works made before are given codes by one gesture.** `work.number` gives
the ticked works that have no code the next codes, in the order they were
made (the title settles a tie - an id is no order). One operation for the
batch with the codes in it (`given`), replayed word for word and undone as
one, which takes the codes off again. A work that has a code, and a work of
a kind without the field, are passed over and said. The overview offers it
for one work under the empty field (**Next code**), the catalogue's bar for
the ticked rows (**Fill "Code"**).

## Consequences

- The owner's songs carry the code their folder is named by; a new song has a
  folder name before its first file is rendered.
- A code is the work's like any field: it can be changed by hand, and nothing
  else moves when it is.
- Two devices that each make a work offline count from the same greatest code
  and can give the same one. Nothing merges two workspaces yet; when
  something does, a code given twice is a conflict for it to name, not for the
  numbering to prevent.
- A work emptied from the trash takes its code out of the count: the next new
  work could be given it again. Its folder, if one was made, is then the new
  work's.
- A clone keeps its donor's code: it is a second attempt at the same work,
  and shares its folder.
