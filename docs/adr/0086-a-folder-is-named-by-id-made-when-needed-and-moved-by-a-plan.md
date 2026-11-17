# 0086 — A folder is named by id, made when needed, and moved by a plan

Date: 2026-11-17
Status: Accepted

## Context

A work's folder on disk is found by a template and looked at, never attached
or owned (ADR 0057). Three things about that arrangement wore badly.

**The name.** The shipped templates named a folder by the title of what the
work is made from: `songs/{origin.title}`, and inside it `video`, `audio`,
`shorts/{n}`. A song renamed in kilna was a song whose folder was no longer
found - the files stayed under the old name, and the Files tab showed an
empty place where the renders had been.

**The making.** A folder that was not there offered **Make the folder**. It
was a step between a person and the place they wanted to drop a file in, and
the alternative that suggests itself - make the folder with the work - leaves
a folder for every work ever planned: a year laid out in advance is a few
hundred empty folders nobody will open.

**The promise.** "kilna never writes, moves or deletes a byte in it" was true
and left no way to change a layout: a template is one line in the profile,
and the files it pointed at yesterday do not follow it.

## Decision

**A folder is named by the work's id.** `{id}` and `{kind}` are placeholders,
and every shipped profile names `{kind}s/{id}` - one folder per work, flat,
under the media folder. The id is the one name a work has that nothing ever
changes. It is not a name to read: the way to a folder is **Open the folder**,
on the Files tab and in a catalogue row's menu. `{title}`, `{n}`, a field and
`{origin.…}` stay, for a layout of the owner's own.

Inside a work's folder the top is for what is finished, and what it was made
out of lies below - `raw/`, `scenes/`, `images/`, `drafts/`, `candidates/`.
The names are a habit; what is held is that a kind's rule for its finished
file reads the top of the folder and never a folder below.

**A folder is made when it is first needed, and never before.** Making a work
makes no folder. Opening it does - `open_work_folder` makes it and opens it
in the file manager - and so does a plugin that writes into it, which makes
its own. There is no "make the folder".

**A folder with nothing in it goes with its work; one with a file never
goes.** When a work goes to the trash, deleted or its making undone, a folder
that holds no file anywhere under it is removed, by the call that refuses a
folder holding anything. A work restored gets its folder again when it is
next opened. Nothing else in kilna removes a folder, and nothing removes a
file: not a delete, not emptying the trash, not an undo.

**A layout is changed by a plan, and the templates last.** A stored profile's
templates are never rewritten on upgrade - that alone would lose sight of
every file. `kilna --rehome-media` plans the move from the stored templates
to the shipped ones by asking kilna itself where each work's folder is and
where it would be: a file keeps its place inside its work's folder and
belongs to the deepest work that holds it; what lies where the old templates
wrote and belongs to no work goes to `_unsorted_migration/`; a file that
would land on another goes to `_unsorted_migration/_conflicts/`; a folder
outside the media folder is taken in only when it was named to look into and
is called exactly what one work is called. The plan is a report and moves
nothing. `--apply` copies the database, moves the files, removes the folders
the moves emptied, and switches the templates only when every move landed -
through the logged edit the profile editor makes, so a replay arrives at the
same templates.

## Consequences

- A rename never moves a folder, and no folder exists for a work that has no
  files.
- "Never writes, moves or deletes a byte" becomes "never touches a file by
  itself": kilna makes a folder, removes an empty one, and moves files only
  when asked to with a plan a person has read.
- A stored profile with title-named templates keeps working exactly as
  before until its owner runs the move. The two layouts do not mix: the
  templates switch for every kind at once, after the files.
- An orphaned subfolder inside a work's folder - a `clip/` with no clip in
  the workspace - moves with the work that holds it, under the same name. It
  is not guessed to be a work.
- Removing an empty folder happens when the work goes to the trash, not when
  the trash is emptied: by then the work is a snapshot, and where its folder
  was is no longer something kilna can ask.
- The move is the owner's to run. It is not part of an upgrade, and nothing
  runs it by itself.
