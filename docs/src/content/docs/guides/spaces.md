---
title: Spaces
description: A space is a folder with a database of its own - a channel, a blog, a book. Making one, switching between them, moving one to another machine, opening one by double-click or by a link, and what happens when a space is open somewhere else.
---

A **space** is one body of work kept apart from every other: a channel of
songs, a blog, a novel, a podcast. Each is a folder of its own, with its own
works and versions, its calendar, its canon, its register of repeats, its
history and its trash. Two spaces never see each other's works - two channels
of songs are two registers of repeats and two calendars that never mix.

```text
<a space's folder>/
  space.kilna   what the space is: its name, its kind, its colour and mark
  kilna.db      everything in it
  media/        the files attached to its works
  assistant/    the empty folder the assistant's runs start in
```

## Kinds

A space is made of one of the profiles kilna ships - **Studio** (songs, clips,
shorts), **Blog**, **Novel** or **Podcast** - and speaks that craft's words
from then on. The kind is a starting point, not a cage: the profile inside the
space is its own copy, and editing it in **Settings → Profile** changes that
space alone. Two spaces of the same kind are as normal as two of different
kinds.

## Making one and switching

The button at the foot of the menu shows the space that is open. Press it to
see the others, newest first, and open one: the window swaps to it without a
restart and starts on its dashboard. **New space…** asks for a name, a kind, a
colour and a mark, makes the folder and opens it. **Manage spaces** opens
**Settings → Spaces**, where each can be renamed, given another look, shown in
its folder, or put in the trash.

The command palette (**Ctrl+K**) offers every other space too: type its name.

While something is going - an assistant run, a task waiting for a slot, a
plugin - the window stays where it is, and says so: each of them would finish
into the space it began in.

## The trash, and forgetting

A space in the trash is off the menu, and its folder is untouched. From
**Settings → Spaces** it can be taken back, or **forgotten**: taken off the list
for good. kilna never deletes a space's folder - a forgotten one is still on
disk, and double-clicking its `space.kilna` brings it back.

## Settings: the application's and the space's

**Settings** lists two groups. The application's are the same whatever space
is open: how the window looks and speaks, the work card, the assistant (the
model, how many runs at once, what a run may read, the week's budget), agents,
and the spaces themselves. The space's open with it: its profile - the words,
the rhythm, the calendar, the window of repeats - what its plugins are told,
and its data.

The week's budget is one for all your spaces: what the runs of every space
cost this week counts against it together.

## Backing up and moving

A space is its folder. Backing it up, or carrying it to another machine, is
copying the folder whole. On the other machine, double-click its `space.kilna`
to open it: kilna adds it to its list, and finds its files where the folder
is now. A copy opened beside its original becomes a space of its own.

**Settings → Data → Backup** still makes a backup of the open space's database
while it is in use, with its files beside it; see [Data](/kilna/reference/data/).

## Open on another machine

A space kept in a synced folder can be open on two machines at once, and two
windows writing one database through a sync would lose one's work to the
other's. So the window that opens a space leaves a `kilna.lock` in its folder
and renews it every minute. Another machine that finds it fresh opens the space
**for reading only** - a banner says who holds it, and a save is refused in
one sentence - until it is closed there. A hold left behind by a window that
crashed goes stale after five minutes and is taken over.

## Links into a space

`kilna://<space>/work/<id>` opens a work from anywhere a link can be followed -
a note in another app, a calendar entry, a message to yourself. The space is
its name, with spaces and letters beyond ASCII percent-encoded as a link needs
them: `kilna://Night%20Shift/work/3f2c…`. Following one opens the space if it
is not open, and the work in it. The installer registers the scheme.

## Portable kilna

A file named `kilna.portable` beside `kilna.exe` makes that copy portable: its
data, its log and the spaces it makes live in a `data` folder beside it, and a
space under it is remembered by its place there, so the copy keeps its spaces
when the drive it is on changes its letter.

## From one database to spaces

Until v0.109 one database held every profile, and the picker at the foot of
the menu switched between them. The first start of v0.109 moves that database
into spaces, once:

- a copy of it is kept beside it first, as `kilna.before-spaces.<moment>.db`;
- the profile with works - the one that was open when it has any - becomes the
  first space, and the database itself moves into its folder with its files;
- every other profile with anything in it becomes a space of its own;
- the shipped profiles nobody used are taken out;
- the window asks what each new space is called the first time it opens it.

[ADR 0077](https://github.com/lacodda/kilna/blob/main/docs/adr/0077-a-space-is-a-folder-with-a-database-of-its-own.md)
records why a space is a database and not a profile in a shared one.
