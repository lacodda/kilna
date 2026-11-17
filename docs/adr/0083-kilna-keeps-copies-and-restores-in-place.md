# 0083 — kilna keeps copies of a space and restores one in place

Date: 2026-11-17
Status: Accepted

## Context

A backup was something a person had to think to take (ADR 0002, v0.50). A
migration rewrites tables in place, and the only copy of what was there
before an upgrade was one somebody had made by hand that week - or none.

Putting a backup back was four steps with kilna closed, written out in
Settings since v0.108 because a workspace cannot be replaced under the
process that holds it open: move the database aside, delete the write-ahead
log that belongs to it, copy the backup in, start again. Skipping the second
step left a log beside the wrong database.

Three smaller things had no place to stand: nobody could see what a space
took on disk or give back what deleted rows held; nothing said that a picture
was missing from under its row until the picture was looked for; and the
history dropped a read line after seven days, to nowhere, with the seven
written in the code.

## Decision

**kilna keeps copies by itself**, in `backups/` in the space's folder: the
first time the space is opened on a day (seven kept), before a newer build
changes the database's shape (three, named with the shape), and before a
restore (three). They are taken by the window, before the database is opened
for writing - so before the migrations - through a read-only connection and
SQLite's backup. A copy that cannot be taken is logged and does not keep the
space shut.

**The copies hold the database alone.** A space's files are not changed by
an upgrade, and copying a folder of clips on every start would be the reason
to turn this off. A backup taken by hand still carries the files.

**A kept copy is never written over.** Two taken in one second get distinct
names. Found by a test: a restore from the copy an earlier restore kept, in
the same second, wrote the present state over the file it was about to read.

**A restore happens in place, through SQLite.** The backup is checked first -
a kilna workspace, a shape this build can open, whole by `quick_check` - and
refused untouched otherwise. What stands is kept as a copy. Then SQLite's own
restore copies the backup's pages into the open database, and the result is
migrated and prepared as a space is on opening. No file is moved or deleted,
so there is no log to leave beside the wrong database, no restart, and a
second process holding the space - an agent's `kilna --mcp` - reads the
restored rows instead of losing its file. The task asked for a marker and a
restart that swaps files before opening; this reaches the same end with
nothing to get half-done between two starts.

**The steps by hand stay**, in Settings and in the docs, for a space that
will not open.

**Size is shown and compacting is offered, an examination only looks.**
`VACUUM` changes no row. The examination names damage, dangling references,
lost files, ownerless files and stray files, and repairs none of them.

**How long the history is kept is the space's own, in its manifest**, and
what leaves the feed goes to a page of its month in `history/` first. Not a
table: it is one number about the folder, it travels with the folder as the
name does, and writing it is no operation of the workspace.

## Consequences

- The breaking migrations of the stages ahead run over a database that was
  copied a moment before.
- Up to thirteen copies of the database lie in a space's folder. The Size
  field says what they take.
- A restored space keeps the files that are in its folder: a file attached
  after the copy was taken is then a stray, and one removed for good since is
  lost. The examination names both.
- A line of history is never dropped without a place to go: an archive that
  cannot be written keeps the line in the feed.
- Restoring replaces the operations log with the backup's, as it replaces
  every other table: the log is of the workspace it lies in.
