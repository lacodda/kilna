# 0076 — A test workspace is a copy of one migrated database

Date: 2026-11-08
Status: Accepted

## Context

Almost every Rust test starts from a workspace of its own: `fixtures::workspace()`
opened a database in memory, applied every migration, and seeded the shipped
profiles. With 49 migrations, a debug build spent about 0.32 s on that before the
test did anything, and some 680 tests start that way. On CI the lib tests took
595 s of the ubuntu job's 13 minutes, and on the owner's machine the suite ran so
long that the system stopped it for memory.

Seeding added another 35 ms: the shipped profiles - half a megabyte of JSON - were
parsed twice per seed, and each stored document was parsed whole to read one
number.

## Decision

**The migrations run once per process.** `db::open_in_memory` copies, page for
page, one database migrated on first use (`MIGRATED`), with SQLite's backup API.
The copy carries the schema and `user_version`; the pragmas and the device are its
own, since the device must differ between two workspaces. `db::write_migrated`
writes the same pages to a file, and `fixtures::workspace_file` opens it as the
window opens a workspace, for tests that need one on disk.

**Migrations are still tested where they are the subject.** The tests in
`db::migrations` apply them to an empty database and to one left at an older
version; the tests of `db::open`, of the device and of `AppState::open` still open
an empty path. A test about anything else no longer pays for them.

**The shipped profiles are parsed once per process** and handed out as copies, and
a stored document's format is read without building the rest of it.

Nothing a user sees changes: the application opens one workspace a session, from
a file, through `db::open`, which migrates as it did.

## Consequences

- On CI the lib tests went from 595 s to 42 s on ubuntu after the first step.
- A new test opened with the fixtures costs milliseconds of setup.
- A test that wants to see migrations run must open an empty path with
  `db::open`, or call `migrations::apply` itself; `open_in_memory` does not
  migrate after the first call.
- The template lives as long as the test process, the size of an empty
  workspace. The parsed shipped profiles live as long as the application, which
  reads them three times at start.
