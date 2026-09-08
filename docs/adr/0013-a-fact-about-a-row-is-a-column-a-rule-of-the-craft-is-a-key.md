# 0013 — A fact about a row is a column; a rule of the craft is a key

Date: 2026-09-08
Status: Accepted

## Context

The model package gathers every change to what the schema records into a
few versions before 1.0 freezes the format (see the plan's "format 1 frozen"
rule). Its second step, v0.50, had to place a dozen new things somewhere:
who gave a score, a tier held by hand and why, a bookmark, the time of day a
release goes out, the draft a draft was written from, what a collection is
aiming for, a due date on a board note, the kind of answer an axis takes,
weights that differ by kind of release, and which columns the catalogue
shows.

ADR 0001 says the schema is one and what differs between crafts lives in
the profile. It does not say where a *new* thing goes when both places could
hold it. A tier pin could be a profile rule ("clips are held at their
score") or a fact about one work; axis kinds could be columns on
`work_score` or keys in the profile; the catalogue columns had been living
in the browser for two versions because neither place had been chosen.

Each of these choices is small. Taken one at a time, they would have been
made a different way each time, and the format that freezes at 1.0 would
carry the record of that.

## Decision

**A fact about one row is a column on that row. A rule of the craft is a
key in the profile document. Nothing lives in the browser.**

Applied to v0.50:

| Thing | Where | Why |
| --- | --- | --- |
| `work_score.rater` | column | who judged is a fact about one snapshot |
| `work.tier_pinned`, `tier_pinned_at`, `tier_pin_reason` | columns | one work, held by one person, for one reason — the shape of the status pin (0005), plus the reason a number nobody can argue with later needs |
| `work.bookmarked_at` | column | one work, marked to come back to; not a status (derives nothing), not a mark (not a word from the profile), not a pin (that word is taken twice) |
| `release.scheduled_time`, `time_zone` | columns | one release, one instant; the slot stays a date because the calendar is read a month at a time |
| `work_version.parent_version_id` | column, `ON DELETE SET NULL` | one draft's lineage; a pruned branch keeps its leaves |
| `collection.target_size`, `due_on` | columns | one collection's goal |
| `focus_note.due_on` | column | one line's deadline |
| `axes[].kind`, `axes[].options` | profile keys | what kind of answer an axis takes is the craft's rule, and a snapshot never has to know — a number reads on any kind, a boolean on a flag, an option key on a choice |
| `release_kinds[].axis_weights` | profile key | a clip and an audio release weigh the same axes differently, and that is a rule about the craft, not about a work |
| `catalogue_columns` | profile key | a novel and a record are read down different columns; the craft chooses, not the machine |

Two consequences of the rule were applied at the same time:

- **The profile validates itself.** With more keys come more ways to write
  a document that loads and then misbehaves — a choice with nothing to
  choose from, a weight on an axis that does not exist, a tier past 100.
  `ProfileConfig::validate` names every problem in words a person can act on
  ("axis 3 (`length`) is a choice with nothing to choose from"), and
  `update_config` refuses the whole document rather than storing a half.
  Every shipped profile passes it, by test.
- **The export names its format.** Every page begins `format: 2`. A reader
  written against the pages can say which shape it understands; the number
  moves when a field changes meaning, not when one is added.

The field clocks of 0011 name their columns, so the four triggers on the
tables that gained columns are dropped and recreated whole in 0012. That is
the cost of the structural guarantee, paid at the one place it is due; a
migration that adds a column to a clocked table and forgets its trigger is
caught by a test that pins the new column and reads its clock.

## Consequences

**Positive.** One rule to apply the next time, and the format that freezes
at 1.0 reads as a design rather than as a history. The browser holds
nothing a person would miss on a second machine. A profile that would
misbehave says so before it is saved.

**Negative.** The profile document grows, and so does the page describing
it. A choice axis stores an option *key* in the snapshot: rename the option
and old scores still read, delete it and they are skipped like a missing
axis — the same trade 0001 made for axis keys, now one level down.

**Neutral.** The interface for most of these is not in this version: the
model package lays the columns and keys, the versions that follow build on
them. Until then a pinned tier is visible in the catalogue's verdict and in
the export, a rater and a parent in the export, and the rest only through
the API — which is what "one migration, UI later" was chosen to mean.
