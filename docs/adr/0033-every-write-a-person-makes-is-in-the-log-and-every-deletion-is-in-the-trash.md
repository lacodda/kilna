# 33. Every write a person makes is in the log, and every deletion is in the trash

Date: 2026-09-24

## Status

Accepted. Amends ADR 0014 (what the operations log holds) and ADR 0031 (how a
style brick goes away).

## Context

Two promises stand under the whole application. Every change a person makes is
an operation in the log, so undo takes back that change and a replay rebuilds
the workspace (ADR 0014). And nothing is lost by pressing a button: a deletion
goes to the trash, and only the trash's own two final actions ask a question.

An audit of the code on 2026-09-24 found both broken in places, each quietly:

- **Four commands wrote past the log.** Choosing the current version, a
  plugin's returned fields, a profile edit and an import changed rows without
  an operation. After "make current", `Ctrl+Z` took back the edit before it; a
  replay rebuilt the old current version and the profile as it was seeded.
- **The gate that holds commands to the log could not see them.** It read a
  command's body as the text up to the next command, so a command followed by
  the `recording` helper, or by the deletion helper, "recorded an operation"
  while writing past the log.
- **A style was deleted outright.** No trash entry, no undo, its reference
  pictures gone with it - in a dialog where the button sat between Cancel and
  Save. ADR 0031 had made that choice when the trash did not hold bricks.
- **The trash's final action did not ask**, although the screen, the guide and
  the decisions all said it did, and a purged entry's files stayed in `media/`
  with nothing pointing at them.

## Decision

**Choosing the current version is a `work.update` of `current_version_id`.**
The ownership check stays in front of it; the write is the same operation as
any other edit of a work, with its `before`, so undo and replay know it.

**A plugin's fields are written as `release.update` or `work.update`,** merged
into the row as it is when the plugin answers rather than as it was sent, so a
field edited while the plugin ran is kept.

**A profile edit is `profile.update`,** carrying the whole document before and
after. It replays, and undo puts the previous document back: a profile is saved
as one document and taken back as one.

**An import stays outside the log, by name.** It reads a predecessor's
database once, many rows at a time. The log starts after it the way it starts
after the seed; a replay builds on top of an import rather than repeating it
from a file that may no longer exist. It is listed with that reason in the
gate's exemptions.

**The gate reads a command's own block.** Comments and literals are blanked,
the body runs from the opening brace to the one that closes it, and the set of
commands scanned has to equal the set registered in `generate_handler!` - a
scan that finds fewer passes everything it missed without a word.

**A style brick is a trash entity (`style`), with its reference pictures.**
Deleting one is `entity.discard` like every other deletion; undoing its
creation discards it into the trash like every other creation. The `style.delete`
operation is gone - no workspace had recorded one. A restored brick marks its
tombstone restored (migration 0028), as every other table already did.

**Purging forgets the files too.** The asset rows a purged or emptied entry
held are the files it still owned; each is removed from `media/` unless a live
asset row or another entry still names it - a cloned board shares its files
with the original.

**The two final actions ask through an alert,** which a stray click beside it
cannot dismiss, and the destroying button is the quiet red one rather than the
accent.

## Consequences

- `Ctrl+Z` after making a version current, after a plugin, and after a profile
  edit takes back that action.
- The window's undo offers "an edit to the profile".
- A style's deletion shows a message with *Undo*, and the style appears in the
  trash with a word of its own.
- `tests/operation_coverage.rs` fails on a command that writes past the log
  even when a helper after it records; this was checked by putting the old
  unlogged profile edit back.
