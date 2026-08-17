# 0007 — The journal stores keys, not sentences

Date: 2026-08-17
Status: Accepted

## Context

The journal table declared in migration 0001 had a `summary` column holding a
finished line of text. Nothing ever wrote to it, so the shape had never been
tested against a real requirement.

Two of them arrived since. ADR-adjacent work in v0.13 gave kilna a second
language, with English as a source rather than a fallback and a build gate
(`tools/check-locales.mjs`) holding every locale to one shape. And the trash in
v0.14 established that a record of something outlives the thing itself.

A stored sentence fails both. History written while the interface was English
stays English forever, including for someone who switches to Russian on their
second day — and the locale gate cannot see those strings to check them, because
they are rows in a database rather than keys in a file. The same wording would
also have to exist twice: once in the entry and once in the toast that announced
the same event seconds earlier.

## Decision

An entry stores an **action key and its values** — `work.created` plus
`{"title": "Winter road"}` — and the sentence is built when the entry is read.
The key is an i18n key in the `journal.` namespace, shared with the toast that
announces the same event, so one string in the locale file serves both surfaces
and the gate covers both at once.

Values are **copied into the entry, not referenced**. A line about a work reads
correctly after that work is renamed or deleted, which is the case the journal
exists for.

Three consequences follow:

- **Writing never fails the caller.** `journal::record` returns nothing and
  swallows its own errors. Recording that something happened is not part of that
  something happening, and a full disk must not turn a completed deletion into a
  failed one.
- **Entries are written in the command layer, not in the domain functions.**
  Domain functions are reused by import, which would otherwise write one entry
  per imported row and bury a real history under a bulk operation. A command is
  the boundary where a person did something.
- **A key with no translation renders as itself.** A line of history nobody
  worded is still a line of history; it must not vanish.

`tests/journal_keys.rs` holds the two halves together: every key the backend
writes must have a sentence in `en.json`, and every sentence must belong to a key
something still writes. The locale gate cannot catch this, because a key missing
from *both* locales is consistent.

## Consequences

The `journal` table from 0001 is replaced rather than migrated (0004). It had no
writer and therefore no rows anywhere, so there was nothing to carry over.

Retention is the one place in kilna where something is removed automatically:
read entries older than seven days are swept at startup, unread ones never. This
is a deliberate contrast with the trash, which discards nothing on its own — a
notice that has been read has done its job, while a thing you deleted may still
be wanted.

Storing keys means a reworded sentence changes history retroactively: entries
written last month read with this month's wording. That is accepted, and is the
same trade the interface already makes everywhere else.
