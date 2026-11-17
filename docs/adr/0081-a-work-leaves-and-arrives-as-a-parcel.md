# 0081 — A work leaves and arrives as a parcel

Date: 2026-11-17
Status: Accepted

## Context

The export made "you are not locked in" checkable (ADR 0002), in one
direction: a workspace went out as pages, and nothing came back. A page was
written for a reader, so reading it back would have meant parsing its tables
and telling a version's body from the page around it by headings - and a body
is free text that may hold any heading the page uses.

Three wishes waited on the way in. A folder of drafts written elsewhere, or a
text on the clipboard, could only become works one by one, by hand. One work
could not go out by itself. And a work could not be carried from one space to
another at all - the second channel of the same author started empty - short
of copying rows between two databases.

## Decision

**A work travels as a parcel**: one value holding what is the work's own - its
fields, its versions with their lineage, its scores, its release and what
that goes out under, its notes, its board with the frames drawn for it, and
the files it names. A parcel keeps the ids and the moments its rows were made
under, so the work that lands is the same work, not a copy of it.

**There is one road in.** Everything brought in becomes a parcel before it
lands, and lands by one gesture, `work.bring`:

- a `.kilna.zip` is the parcel as `work.json`, the work's page as `work.md`
  for a person, and its files under `media/`;
- a page of the export carries the parcel at its foot, in a comment, without
  the bodies; each version's body stands on the page between two marks and is
  read from there (export format 8);
- a text from outside - a `.md`, a `.txt`, the clipboard - says a title and a
  body, and is made into a parcel of the kind the person names, with the text
  as the first version of that kind's first draft role.

**A body is told from its page by marks, not by headings.** The opening mark
names the version and counts the body's lines; the closing mark ends it. An
untouched page is read by the count, so a body may hold a line that reads as
a closing mark. A body changed outside no longer matches its count and is
read up to the first closing mark - what the person wrote is what comes back.
The tables of a page stay as they were, for a person; the parcel is what is
read. Editing a table outside changes nothing, and the reference says so.

**Bringing in only adds.** A work already here - the same id, or for a text
from outside the same title - is passed over, and so is one the person threw
away, which the tombstone remembers (ADR 0012). Nothing is merged and nothing
is rewritten; each work is its own unit, so one that cannot land is said with
its reason and the rest arrive.

**What points at the space stays behind.** A parcel does not carry the
collection the work stands in, the works it was made from, the cards and
bricks its cover is built of, the trials of an experiment, the boards of its
conception, or what the audience said. Those are the space's as much as the
work's; carrying them needs the other side to say which of its own rows they
are, and that is the merge this is the forerunner of.

**The space's craft decides what lands.** The total and the tier of a carried
score are computed by the profile of the space it lands in, as every score's
are; a status the space does not name becomes the one a new work starts in,
without its pin; a kind, a role or a door the space does not have refuses the
work, before anything is written.

## Consequences

- A page written by v0.113 or later, brought into an empty space of the same
  craft and exported again, is the same page to the byte. A test holds it.
- A page written before format 8 carries no parcel and is refused with a
  sentence that says to export again. There are no users to keep reading old
  pages for.
- `work.bring` is undone like every creation, into the trash. Like
  `work.clone` it is not replayed from the log: its rows are named by the
  parcel, which the log does not hold.
- Bringing the same parcel twice is a no-op, which is what lets a folder be
  pointed at again after more files were added to it.
- A folder is read by its own files, not the folders inside it: the export's
  folder holds pages that are no works beside `works/`.
- The merge of two spaces (v1.x) starts from here: the ids already agree
  between a work and its carried twin.
