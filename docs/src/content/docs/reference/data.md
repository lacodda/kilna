---
title: Data
description: Export, backup and import, where the workspace file lives, and why restoring needs the app closed.
---

kilna is local-first: one SQLite file holds your workspace, and media stays
as plain files on disk with only a path recorded in the database. See
[ADR 0002](https://github.com/lacodda/kilna/blob/main/docs/adr/0002-local-first-storage.md)
for the reasoning behind that split.

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

- Front matter: title, kind, status, created/updated timestamps, and every
  craft-specific `meta` field.
- Every version, grouped by role, newest revision first, with the current one
  marked and the **body in full** — not a summary.
- A table of score history: date, total, tier, and the axis values behind
  each snapshot.
- Every release tied to the work: kind, date (scheduled or released), and the
  link if one was recorded.
- Notes attached to the work, with their tags.

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
