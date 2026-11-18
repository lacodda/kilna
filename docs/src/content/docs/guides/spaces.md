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
shorts), **Blog**, **Novel**, **Podcast**, **Poetry** (poems, translations and
reading sets, since v0.129), **Newsletter** (issues, special editions and
sequence emails, since v0.129) or **Empty** - and speaks that
craft's words from then on. The kind is a starting point, not a cage: the
profile inside the space is its own copy, and editing it in **Settings →
Profile** changes that space alone. Two spaces of the same kind are as normal
as two of different kinds.

**Empty** is for a craft kilna does not ship: one kind of work with a text,
and the rest - kinds, axes, statuses, fields, actions - named by you in the
profile editor.

## Types

A **type** is a profile kept as what a new space is made of. When a space's
vocabulary has grown into your own - axes reworded, statuses added, actions
rewritten - **Settings → Profile → Save as a type…** keeps it under a name,
and **New space…** offers it beside the shipped kinds. A second channel then
starts with the first one's words instead of the shipped ones.

A type lives on this machine, in kilna's own data, and is listed in
**Settings → Spaces → Types**, where it can be renamed, written out to a file
or deleted - and where **Import a type…** reads one from a file, so a
vocabulary travels between machines as one document.

A space made of a type holds its own copy from the moment it is made: editing
the type's source space, or deleting the type, changes nothing in it. It stays
a space of the kind the type grew from, so what a later version of kilna adds
to that kind - a new action, a new field - reaches it as it reaches any space
of that kind. Works are not part of a type: a space made of one is empty.

## The first start

On a new machine kilna makes a space so the window has somewhere to stand,
and welcomes you into it (v0.129): whether Claude Code is on the machine -
the assistant waits without it, nothing else does - then what the space is
made of, its name, colour and mark, and **Start with a demo**; then your
first work, by its title, opened on its card - or **Look around first**. A
Studio stays the space kilna made; any other kind is a new space, and the
untouched first one goes. **Later** puts the welcome off to the next start.

**A demo** is a handful of made-up works in English at every stage -
versions to compare, a score, notes, a card of the canon and, in Studio,
releases booked for the coming days - so each screen has something on it.
It is written through the ordinary gestures, so it is in the history and is
deleted like anything else, and it goes only into a space with no works:
**New space…** offers it too.

**First-visit hints.** The first time a screen opens on a machine it says,
in two sentences, what it is for; **Got it** puts that one away, **No more
hints** all of them, and **Settings → General** brings them back.

## Making one and switching

The button at the foot of the menu shows the space that is open. Press it to
see the others, newest first, and open one: the window swaps to it without a
restart and starts on its dashboard. **New space…** asks for a name, what the
space is made of - a shipped kind or one of your types - a colour and a mark,
makes the folder and opens it. **Manage spaces** opens
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
