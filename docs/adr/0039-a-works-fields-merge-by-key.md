# 39. A work's fields merge by key, and an undo takes back one

Date: 2026-09-28

## Status

Accepted.

## Context

A work's fields - BPM, key, the hook, whatever the craft's profile names -
live in one JSON object, `work.meta`. Until v0.82 a patch replaced the whole
object: the window read the fields, changed one, and sent them all back. Two
things followed from that. A field written somewhere else between the read and
the write - by a plugin, by the assistant, by a second screen - was put back
the way the window last saw it. And the operation log held the whole object
on both sides, so undoing one edit put back every field as it stood before
it: an undo of the BPM also took back the key typed after it.

v0.82 made the overview a board whose fields are edited in place, one at a
time, which is exactly the pattern that loses neighbours.

## Decision

**A work's `meta` patch merges by key.** A key with a value sets that field, a
key with `null` removes it, and a key not sent is not touched. The window
sends only the field it changed (`{ meta: { bpm: 104 } }`), and so do the
assistant's field writes and a plugin's writes on a work.

**The log holds only the keys that were sent.** The `before` of an edit is the
old value of those keys (`{"meta":{"bpm":98}}`), so an undo takes back that
field and nothing beside it, and replaying the log rebuilds the same object.

**Which fields merge is declared by the patch type.** A trait in
`reversal.rs` makes each recorded patch say which of its fields merge by key;
only the work's `meta` does - a release's `meta` is still replaced whole, the
form that edits it sends every field. The compiler refuses to record a patch
type that has not said.

**Old entries are read as the merge they meant.** Entries written before this
hold the whole object on both sides; undo and replay read them as a merge of
every key, which is what they did. The log is not rewritten.

## Consequences

Two screens editing different fields of one work no longer overwrite each
other, and an undo is as small as the edit it takes back. The tests hold both:
`tests/undo_takes_back.rs` undoes one of two field edits and keeps the other,
keeps a neighbour changed since, and takes an old whole-object entry back
whole; `tests/replay_rebuilds.rs` rebuilds a log that mixes the two forms.
