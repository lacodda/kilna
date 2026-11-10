# 0077 — A space is a folder with a database of its own

Date: 2026-11-10
Status: Accepted

## Context

Until v0.109 one database held every profile kilna ships - Studio, Novel,
Podcast, Blog - and one was active: switching profiles switched which works
the window showed. That was enough for one craft per person. It was not
enough for two channels of the same craft. The register of repeats, the canon
and the calendar are each one per database, so two channels of songs kept in
one Studio profile would have read each other's repeats, characters and slots;
a backup or a move was always all of it; and the coming sync (major 2) would
have had to sync every craft at once.

The owner's own database showed the shape of the problem from the other side:
472 works under Studio, and nothing ever written under the three other
profiles that every database had been seeded with.

## Decision

**A space is a folder with a database of its own:** `space.kilna` (what the
space is - its id, name, kind, colour and mark), `kilna.db`, `media/` and
`assistant/`, and `kilna.lock` while a window holds it. A space's database
holds one profile, the kind it was made from; `profile::seed` installs only
that one into an empty database and carries forward whatever a database
already holds.

**The application keeps its own database, `app.db`,** in its data directory
and never inside a space: the registry of spaces (where each folder is, when
it was opened, whether it is in the trash, whether its name is still to be
asked) and the settings that are the person's whatever space is open - the
assistant's, which move out of each workspace (space schema 0050 drops the
table; the application's schema starts at its own 0001). The week's budget is
the person's one wallet: the spend it is checked against is counted across
every space this machine reaches.

**What a space is lives in its folder; where it is lives in the registry.**
The name, the colour and the mark travel with the folder, so a space copied
to another machine is called what it was called here. The registry keeps no
copy of them: a space out of reach is listed by its folder. A folder whose
file carries the id of a listed space is that space moved, when the old
folder is gone, and a copy otherwise - which gets an id of its own.

**The window opens one space and swaps it without a restart.** `AppState`
holds the application's connection and the open space's, and switching
replaces the space's connection whole. It is refused while anything is going -
a run, a task waiting for a slot, a plugin - since each would finish into the
space it began in; a run also writes through connections to the space it
started in, whatever the window opens after. The window then starts over on
the dashboard with every cached answer dropped.

**A file's path stays whole in its row, and is rebased when the folder
moves.** Every comparison of two paths - is a file still named by a live row
or by an entry of the trash, before its bytes go - compares one form with the
same form, so paths stay absolute. Opening a space points every row, and every
row the trash keeps, that names a `media/` folder elsewhere at this space's own
`media/` when the file is there under the same name.

**A space open on another machine is open here for reading only.** The window
leaves `kilna.lock` - the machine and when it last said so - and renews it
every minute; another machine that finds it fresh opens the database with
SQLite's read-only flag, and a write anywhere answers with one refusal
(`space.readOnly`) through the conversion of SQLite's own error, so no command
has to ask first. A hold older than five minutes is taken over. The same
machine is never kept out by its own hold: one window per machine is what the
single instance promises. `kilna --mcp` takes no hold and does not sweep
runs: it is the window's companion, and a run may be going there.

**The move into spaces runs once, at the first start of v0.109,** and can be
run again after a start stopped half-way: a copy of the old database is kept
beside it; the assistant's settings are copied into `app.db` before the step
that drops them runs; the profile with works (the open one when it has any)
becomes the first space and keeps the database itself, moved, not copied, the
database last; every other profile with content becomes a space of its own
from a copy with the others taken out; the shipped profiles nobody used go;
each new space asks its name once.

**A space opens by double-click and by link.** The installer ties `.kilna` to
kilna, and `kilna://<space>/work/<id>` - the space by name - is registered
through Tauri's deep-link plugin; both reach a running window through the
single instance. A copy of kilna with `kilna.portable` beside its executable
keeps its data, and makes its spaces, in a `data` folder beside it, and
remembers a space under it by its relative place.

## Consequences

- Profiles are no longer switched: `activate_profile`, `list_profiles` and the
  profile picker are gone, and the picker's place lists spaces.
- A workspace from before v0.109 can still be opened as a space's database:
  `AppState::open` opens one database by itself (tests, a rehearsal on a copy).
- Rows that name a profile without a foreign key - the trash, the tombstones,
  the operations log - are pruned by profile when a database is split.
- A space opened read-only does not rebase its files, so on a second machine
  its pictures show only when the folder sits at the same path there.
- Sync (major 2) is per space, as the trash, the history and the canon are.
