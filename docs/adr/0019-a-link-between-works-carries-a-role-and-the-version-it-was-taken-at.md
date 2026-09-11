# 0019 — A link between works carries a role and the version it was taken at

Date: 2026-09-11
Status: Accepted

## Context

A video is made from a song. Since v0.57 a video is a kind of work of its
own (ADR 0017), so the question became how the two are joined: the song
should show its clips, the clip should name its song, and either should be
able to start the other. Three shapes were on the table: a `source_id`
column on `work`; a table per relationship (`clip_of`, later `remix_of`);
one table of links with a role.

Underneath sat a second question. The clip's scenes are written against
the song's text as it was that day. When the song moves on, what happens
to the clip? The owner decided (2026-09-10): nothing is pushed into it —
kilna marks the fact and the person decides, the same rule a stale score
follows. That needs the link to remember which version it was taken at,
or "moved on since" cannot be told from "as it was".

## Decision

**One table, `work_link`, for every role a link can have.** A row says
*this work was made from that one, in this role* — `donor` is the first
role; a remix and a sequel (v1.14) are values in the same column, not
tables. A work can have several sources and be the source of several
works; the pair and the role are unique. Both sides cascade, and the
trash captures the rows from both sides, so a restored song gets its
clips back and a restored clip gets its song. A link is deleted outright,
tombstoned by the schema: it is a fact about two works, not a thing with a
body worth a drawer.

**The link remembers the source's version at the moment it was made**
(`source_version_id`), and the card derives *drifted* from it: another
version is current now, or the very version taken was edited in place —
which ADR 0015 allows until something judges it, and which the version's
field clock records. A link taken when the source had no version never
drifts. A drifted link is a mark on the card with a way to the diff of the
source's own versions; nothing else changes.

**Making a work from another copies once and links.** `derive_work` gives
the new work the source's title and the overview fields the profile has,
and a `donor` link at the source's current version. Its text is not
copied: a video's roles are its own. Two operations, as a hand would make
them — the work, then the link — so a replay rebuilds both under their
ids.

Rejected: a column on `work` (one source, no role, no version); a table
per role (the second role is a migration and a screen); copying the
source's text into the derived work (the roles differ, and a copy is the
second truth ADR 0001 forbids).

## Consequences

- Migration 0015; `link.create` and `link.delete` in the operations log;
  `work_link` in the tables a replay must rebuild.
- The card gains a *Links* tab: sources with the revision taken and the
  mark, a picker to name a source, what was made from this, and one button
  per other kind of the profile to make one. The header menu offers the
  same. The catalogue's *Add* makes the kind the catalogue is narrowed to.
- The versions tab reads `?compare=` beside `?version=`, so a link can
  open the diff it names.
- The MCP `work` tool answers `sources` (with `drifted`) and `derived`.
- The export writes a *Made from* section on the work's page.
