# 0079 — A type is a profile kept for making spaces

Date: 2026-11-16
Status: Accepted

## Context

A space is made from a profile kilna ships (ADR 0077): Studio, Blog, Novel,
Podcast. Its vocabulary is then the owner's - axes reworded, statuses added,
actions rewritten over months. A second channel of the same craft started
from the shipped words again, and the only way to carry the first one's
vocabulary over was to copy JSON between two databases by hand. A craft kilna
does not ship had no starting point at all: the nearest shipped profile, with
most of it deleted.

The profile editor could not help either. Until v0.112 it renamed words and
changed weights; adding a kind, an axis or a field meant editing the document
outside the window, and there was no way to take a document out of a space or
put one in.

## Decision

**A profile is a document that travels.** `ProfileDocument` is the shape of a
shipped profile's file - `key`, `name`, `description`, `config` - and the one
shape for everything: the files kilna ships, a profile written out of a space,
a document written by hand. All of them are read by the parser a stored
profile is read by, which brings an older format to the current one, and all
of them are held to `ProfileConfig::problems` - refused whole, every problem
named, when anything is wrong.

**A type is a document kept under a name**, in the application's database
(`space_type`), beside the list of spaces and for the same reason: it is what
this machine makes spaces of, not a fact of any one space. A type is made by
saving the open space's profile - as it is stored, not as the editor holds it
unsaved - or by reading a document from a file.

**A space made of a type is a space of the kind the type grew from.** The
type's `key` names a shipped profile; the space's manifest says that kind, and
its profile row carries that key. So the space is carried forward exactly as a
space of that kind is: the type's document is stored and then upgraded against
the shipped profile at once, and again at every start. An action shipped a
month after a type was saved reaches the spaces made of it, and the type
itself is never rewritten. A document whose key kilna does not ship grows from
the empty type.

**The empty type ships** as a fifth profile, `blank`: one kind of work with a
text, the four statuses the automation derives and one to shelve, one way out.
It is the starting point for a craft kilna does not ship, with the rest named
in Settings - which, since this version, edits the whole document.

**Reading a document into an open space goes through the draft.** The window
takes the document as the profile editor's draft; it is looked over and saved
by the gesture every edit is (`profile.update`, logged whole, undoable), or
discarded. There is no second way to write a profile.

**A type and the spaces made of it share nothing afterwards.** Each space
holds its own copy from the moment it is made. Editing or deleting a type
changes no space; editing a space changes no type.

## Consequences

- Two channels of one craft start from one vocabulary; a vocabulary can be
  sent to another machine as one file.
- A type is not versioned and not synchronised: it is a row on this machine.
  A backup of a space does not hold the types, and does not need to - the
  space holds its own profile.
- Because a type keeps its shipped key, a type saved from Studio and edited
  until nothing of Studio is left is still carried forward as Studio, and
  what `carry_forward` adds by key comes back into spaces made of it. That is
  the price `carry_forward` already states for a profile edited in place, and
  the same answer applies: what arrives is one click to remove.
- Works are not part of a type. A space made of one is empty.
