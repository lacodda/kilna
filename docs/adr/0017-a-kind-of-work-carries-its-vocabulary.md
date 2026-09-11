# 0017 — A kind of work carries its vocabulary

Date: 2026-09-11
Status: Accepted

## Context

ADR 0001 put everything that differs between crafts into the profile
document rather than the schema: the axes a work is judged on, the tiers a
total falls into, the roles a version can have, the kinds of release, the
statuses. It laid them flat on the profile — one list of axes for every work
the profile holds — because every work in a profile was the same kind of
thing. A music profile held songs.

The owner's decision of 2026-09-10 broke that premise: a video is a work of
its own, made in the same studio as the song it is often cut to, judged on
dynamics and editing rather than hook and lyrics, carrying a plot rather
than lyrics, going out on YouTube rather than as an audio release. A second
profile for videos was rejected (device A of that decision): the profile is
what *separates* works, and the three things the owner asked for — the song
shows its clip, a video is made from a song, both are created from either
side — are ties.

So one profile, several kinds, each with its own vocabulary. That is a
change to the shape of the document, and by the rule of 2026-08-30 — no
users, no compatibility — it is made once and whole rather than as an
optional layer over the flat lists.

## Decision

**Format 2: the vocabulary belongs to the kind.** `work_kinds[]` entries
carry `axes`, `tiers`, `version_roles`, `release_kinds` and `statuses`. The
profile keeps what is genuinely about the profile: collection kinds, meta
fields, marks, prompts, rhythm, catalogue columns. The flat lists are gone
from the document.

**Format 1 is read, and read as "every kind gets these".** The parser takes
either shape (`RawProfileConfig`): a document with flat lists hands them to
every kind that declares nothing of its own — all of it or none of it, so a
kind that names even one list is taken to have named its vocabulary — and
comes out as format 2. Every path that reads a document goes through the one
parser: the stored rows, the shipped files, an imported JSON, the tests. At
startup every stored row still in format 1 is rewritten, once.

**The shipped files may stay flat where the kinds share a vocabulary.** The
novel, blog and podcast profiles ship flat lists, which every kind takes;
Studio (the profile formerly called Music) ships flat lists for `song` and
`instrumental` and explicit vocabularies for `video` and `short`. A person
writing a profile by hand can do the same.

**An existing workspace gains a new kind without its judgement.** When the
shipped profile gains a kind the stored copy lacks, carry-forward appends
it with its statuses, roles and kinds of release, and without its axes and
tiers. The workspace's axes are the owner's own words; a stranger's axes
appearing silently beside them is the one thing the profile must never do.
Works of the new kind are scored empty until the owner writes its axes. A
fresh workspace, seeded rather than carried forward, gets the whole kind.

**A kind the profile does not know is an empty vocabulary.** Nothing to
score, no roles, no doors; the work still opens. Screens with no single work
in hand — the catalogue's filters, the calendar's kind bar, a batch moving
many works — read the union of every kind's list, once per key.

**Everything that judges, names or ships a work reads the work's kind.** A
score's total and tier, a status derived from the facts, a release's
readiness, a proposal read out of an answer, the MCP server's tools: each
looks up `config.vocabulary(work.kind)`. There is no path that reads a flat
list, because there is none.

## Consequences

- The profile editor edits a vocabulary per kind. A profile with four kinds
  shows four sections; the shared ones are copies, edited separately. A
  future "same as" reference is a convenience, not a format change.
- Two works of different kinds can share a status key with different
  labels; the catalogue's filter shows the first label. Keys are the
  contract, labels are the words.
- `release_kinds` are per work kind too: a video's *YouTube* and a song's
  *clip* are different doors. The calendar draws each release with the glyph
  its work's kind names for it.
- Scores taken before this change are unaffected: a score is a snapshot of
  marks by axis key, and the song's keys did not move. The migration was run
  on a copy of the owner's workspace (262 scores) before the tag, and every
  total and tier read the same.
- The MCP `workspace` tool now lists kinds with their vocabularies; an agent
  scoring a work reads that work's kind's axes.
