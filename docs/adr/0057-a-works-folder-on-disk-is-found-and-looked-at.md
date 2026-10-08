# 0057 — A work's folder on disk is found by its name and looked at, never owned

Date: 2026-10-08
Status: Accepted

## Context

A person keeps most of what they make for a work outside kilna: renders,
takes, the stills a generator gave back, the mix exported last night. Those
files are written by other programs, often in a layout older than kilna - on
the owner's machine, a folder per song with `images/`, `clip/v1/`,
`shorts/v1…v6/` and `audio-release/v1/` inside, 1 664 files and 50 GB across
208 songs (2026-10-08). The plan asked for that folder to show on the card
"without attaching by hand" (#18).

ADR 0027 answers the other question - a file a person *attaches* is copied
into the workspace, so a cover survives a tidied downloads folder and a
backup carries it. That answer does not fit here. Copying fifty gigabytes of
video into the workspace would make every backup the size of the disk, and a
copy of a take would go stale the moment the take was rendered again.

## Decision

**A work's folder is looked at where it lies.** kilna lists it, plays what is
a sound, opens a file in the program the system opens it with, shows where a
file lies, and copies a file in when the person asks - which makes that one
file an asset under ADR 0027. It never moves, renames or deletes anything in
the folder. The one thing it writes there is the folder itself, when the
person asks for it to be made.

**The folder is found, never recorded.** Two facts say where it is, and no row
says which folder a work has:

- **the media folder** - one per workspace profile on this machine, in the
  table `media_root` (migration 0036);
- **the kind's template** - `folder` on the work kind in the profile:
  `songs/{origin.title}`. `{title}` reads the work's title, `{key}` a field,
  `{origin.title}` / `{origin.key}` the same of what the work is all made from
  (`publication::origin` - the song, for its clip and a short cut from the
  clip; the work itself when it is made from nothing).

A template changed today points every work at its new place at once, and a
folder made by hand is seen the moment it has the right name. Recording the
folder per work would be a second truth to drift from the disk.

**The media folder is the machine's, the template is the craft's.** The root
is `D:\` on one computer and `/Volumes/media` on the next, and it is a private
path: in the profile it would be wrong on every other device and would leave
in every profile a person exports. So it lives in its own table with no field
clocks and no tombstone, and is written past the operation log, as the open
profile is (`operation_coverage.rs` names the exemption) - a replay or a sync
must not carry it. The template is how a craft lays its disk out, and travels
with the profile; the shipped profiles name one for every kind, and
`carry_forward` brings it to a workspace that predates it.

**A value is made safe, never trusted.** A title becomes a folder name with
the characters no folder may hold on Windows left out (the tightest rules,
since a template written on one machine names folders on all of them), spaces
run together and trailing dots dropped. A value can never become a step of
the path, and the resolved folder is checked to lie under the root anyway. A
template that is a place on a disk, climbs with `..` or names an unknown kind
of placeholder is refused when the profile is saved. A field the template
reads and a work leaves empty is reported by name: a hole in the path would
land on the folder above and show the wrong files.

**The window reads the folder, and opens only media from it.** The asset
protocol's scope is widened for every media folder at start and for a new one
the moment it is set (ADR 0027 widened it for `media/` the same way); the CSP
gains `media-src` so a clip and a sound can play. Opening a file in another
program goes through `open_media`, which accepts only a picture, a clip, a
sound or a plain document under a media folder or the workspace's `media/` -
the window names the path, and a path the window names must not become a way
to run a program.

**The listing is read again when the window comes back.** The query refetches
on focus (the query client listens for the window's focus, not only its
visibility, which a desktop window rarely changes), so a render finished in
another program is there when the person returns. No file watcher: the moment
a person looks is the moment the list has to be right, and between looks a
watcher would only spend.

## Consequences

A song gains a Files tab: it goes out only as what is made from it (ADR 0047),
but it has a folder. The tab rule is now "the kind names doors, or names a
folder, or the work holds files"; a song's tab sets no cover.

A work with a sound - attached or in its folder - has a player under its name
on the card, for whatever craft; a switch per profile would be a setting
standing in for what the files already say.

A layout of the owner's (`songs/{id}/clip/v1`) is a template, not code: the
code knows nothing of songs, clips or the owner's disk.
