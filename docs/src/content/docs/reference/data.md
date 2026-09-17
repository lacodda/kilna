---
title: Data
description: Export, backup and import, where the workspace file lives, why restoring needs the app closed, and what the workspace records about itself.
---

kilna is local-first: one SQLite file holds your workspace, and media stays
as plain files on disk with only a path recorded in the database. See
[ADR 0002](https://github.com/lacodda/kilna/blob/main/docs/adr/0002-local-first-storage.md)
for the reasoning behind that split.

The same screen holds one thing that is not about files: **Statuses**, which
compares every work's status against what has actually happened to it and shows
what it would change before changing anything. That one is described in
[Statuses](/kilna/guides/statuses/).

## Where the workspace file lives

The workspace is created under the platform's application data directory the
first time kilna runs. From inside the app, the **workspace path** command
reports the exact file location, so you can find it in a file manager or point
a backup tool at it directly.

## Backup

Backing up copies the whole workspace to a destination you choose. This uses
SQLite's own backup interface rather than a plain file copy — with
write-ahead logging enabled, the file on disk at any instant is not the whole
story, and a naive copy taken mid-write can produce a database that opens but
is missing the most recent changes. kilna's backup is taken through a live
connection instead, so it's complete even if writes are happening at the same
moment.

A suggested file name is generated for you from the current timestamp, in the
form `kilna-YYYY-MM-DDTHH-MM-SS.db` — colons are replaced because they aren't
valid in a Windows file name.

## Restore

Restoring replaces the live workspace with a backup file. Two things happen
for safety:

- The file being restored is verified as an actual kilna workspace first — it
  must contain a `profile` table — before anything about the current
  workspace is touched. A stray file handed to restore by mistake is refused,
  and the live workspace is left exactly as it was.
- The **current** workspace is moved aside, not deleted, to a sibling file
  named with a `.db.replaced` extension. A restore is exactly the moment when
  the thing being replaced might turn out to have been wanted after all.

### Restore needs the app closed

Restoring a workspace while kilna has an open connection to it is unsafe — a
running app is actively reading and writing through its own connection, and a
file swapped out from underneath it can leave things in an inconsistent
state. Close kilna, replace the workspace file (or use a restore flow that
does so before kilna reopens it), then relaunch.

Stray write-ahead-log and shared-memory files (`.db-wal`, `.db-shm`) belonging
to the workspace being replaced are cleaned up automatically as part of a
restore, so they don't linger and confuse a subsequent open.

## Export to markdown

Export writes the active profile's entire contents out as one markdown file
per work, plus the profile's own configuration as `profile.json`, into a
directory you choose. This is the concrete form of "you are not locked in" —
a plain directory of files, readable without kilna, with each work's
structural facts in a YAML front matter block and its full bodies underneath.

Each work's page includes:

- Front matter: the page's `format` number, title, kind, status,
  created/updated timestamps, a pinned tier and its reason when there is one,
  the bookmark when set, the stage as a percentage when the work has been
  judged, and every craft-specific `meta` field.
- Every version, grouped by role, newest revision first, with the current one
  marked, the revision it was written from named, and the **body in full** —
  not a summary.
- A table of score history: date, total, tier, who gave it, and the axis
  values behind each snapshot.
- The storyboard, for a work that has one: the scenes as a table — number,
  section, seconds, kind of shot, description — then each scene's prompt
  blocks in full.
- Every release tied to the work: kind, date (scheduled or released), time of
  day and zone when set, and the link if one was recorded.
- Notes attached to the work, with their tags.

The `format` line is for a reader written against these pages: it says which
shape the page takes, and moves when a field changes meaning or a section
changes shape — not when a field is added. Pages written by v0.50 and later
say `format: 2`; earlier exports carried no line, which reads as format 1.

Notes not attached to any work are written to a separate `notes.md` rather
than lost. File names are derived from each work's title with unsafe
characters replaced and a short id suffix appended, so two works sharing a
title never overwrite one another — and non-ASCII titles are kept as-is,
since a title in Cyrillic or any other script should stay readable in the
exported file name.

## Import from a predecessor

kilna can bring in a slice of a predecessor workspace — the command exists for
migrating out of an earlier personal tool, not as a general-purpose importer
for arbitrary data. Existing titles are skipped rather than duplicated.

A title you deleted here is skipped too, even after you emptied the trash:
the workspace remembers what it deleted (see below), and an import that
resurrected it would be undoing a decision you made. The report says how
many were left that way. To have one back, restore it from the
[trash](/kilna/guides/the-trash/) while it is still there.

## What the workspace records about itself

Three kinds of record are kept alongside your data. The first two are
written by the database itself rather than by any screen, so nothing can
skip them:

- **A trace of every deletion.** Which row went, from which table, when, and
  whether it was restored since — without its contents. It survives emptying
  the trash and is what lets the import above tell *deleted* from *never
  here*. A [link between works](/kilna/guides/made-from/) is traced the same
  way, and so is a cut — one stretch of a short's splice, taken from a donor
  video.
- **A clock per field.** For everything you edit in place — a work, a
  release, a note, a scene, a collection, a profile, a chat, a board note —
  the moment each field last changed, not just the row. Saving a form with the same
  values leaves no mark.

The third is written by the application, because only it knows what you
meant:

- **A log of what you asked for.** Every change you make — creating a work,
  editing it, scoring it, scheduling a release, moving a batch of works to
  another status — is recorded as the thing you asked for, together with
  everything carrying it out produced: the identifiers, the moment. One
  gesture is one entry, however many rows it moved, so a change of status
  across a dozen works reads as the one action it was. Nothing is folded
  together and nothing is trimmed by age: the log is complete or it is not
  the log.

  It is not the [history](/kilna/guides/the-history/) you can read on screen.
  That one is written for you, in your language, and forgets old news on
  purpose. This one is written for the machine, and its use is that the
  workspace can be rebuilt from it: play the entries back into an empty
  database, and you have the same one.

All three name the **device** that made the change: an identity the workspace
mints for itself the first time it is opened by this version, and keeps for
life. A restored backup keeps it too — it is the same workspace, continued.

None of this is shown in the interface today. It exists so that the day two
copies of a workspace meet — a second computer, a phone — the facts needed to
merge them honestly are already there for everything written since this
version, rather than only from the day the merge was built. The reasoning is
in [ADR 0012](https://github.com/lacodda/kilna/blob/main/docs/adr/0012-deletions-leave-a-tombstone-and-fields-carry-clocks.md)
and, for the log, [ADR 0014](https://github.com/lacodda/kilna/blob/main/docs/adr/0014-an-operation-records-the-intent-and-what-it-generated.md).
