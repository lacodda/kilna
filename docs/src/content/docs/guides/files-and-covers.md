---
title: Files and covers
description: Attaching files to a work — a cover that shows everywhere, references kept beside it, why a file is copied into the workspace rather than pointed at, the work's folder on disk, and the player on the card.
---

A work can carry files: a cover, a reference, a mix, a subtitle track, the
lyrics as a text. They live on the card's **Files** tab - beside the files
kilna finds in the work's [folder on disk](#the-works-folder-on-disk), which
it looks at and never takes over.

## Attaching a file

Two ways in. **Attach a file** opens the picker; **drop files onto the tab**
does the same. Either way it is any files, and as many as you pick or drag at
once: a picture, a sound, a clip, a text, subtitles, a PDF, an archive (only
pictures until v0.121). A tab with nothing attached draws the place to drop
the first ones. The one thing that has to be a picture is a cover.

## Looking through the files

The bar under the buttons arranges the tab - the attached files and the
folder on disk together - and is remembered on this machine:

- **Images / Audio / Video / Other** - press one or several to show only
  those; each says how many there are. A file is filed by its ending:
  everything that is not a picture, a sound or a clip is *Other*.
- **By type / By folder** - gathered by what the files are, or by where they
  lie: what an attached file is for (the cover, its candidates, a scene's
  stills and clips), and for the folder on disk the folder each file is in,
  the work's own first. Every folder below is read, however deep.
- **Sort by** name, date changed, size or type, and the arrow beside it
  turns the order round.
- **Large icons, tiles, list, table** - the table has the type, the role, the
  size and when the file last changed beside each name.

A file says what it is to the work with a tag: **Cover**, **Candidate**, or
**Finished file**.

## Looking at a file

Click a file and it opens inside kilna (since v0.122; a click used to hand it
to another program):

- **a picture** - fitted to the window, with its size in pixels; **+** and
  **−**, or `Ctrl` and the wheel, enlarge it up to four times its own size,
  **Fit** and **100%** go back;
- **a clip** - played with its sound and its own controls, the whole screen
  among them, at the speed you pick;
- **a text** - a lyric, subtitles, notes, a table (`txt`, `md`, `srt`, `vtt`,
  `lrc`, `json`, `csv`, `log`, `yaml`) - to read, with **Copy the text**. The
  first 512 kB of a longer one, and the viewer says so;
- **a PDF** - in the window's own reader;
- **anything else** - what is known of it: its name, kind, size, when it
  changed and where it lies.

A sound is not looked at: it plays in the [player on the card](#the-player),
and the file that is playing is marked among the rest.

The arrows at the foot - and `←` `→` on the keyboard - go to the file before
and after, in the order the tab shows them. **Show where it lies** opens the
file manager on it, and **Open in its program** hands it to the program your
system opens it with - offered for a picture, a clip, a sound and a plain
document, never for a program.

## What can be done with a file

A right click on a file opens its menu. For any file: **Look at it**, **Open
in its program**, **Show where it lies**, **Save a copy…** and **Copy the
path**; a picture can be made the cover. An attached file is removed from
here - it goes to kilna's trash.

A file of the folder on disk has four more: **Rename…** - a name, in the
folder it lies in, never onto a file that is already there; when it is the
file the work is finished as, the choice follows the new name - **Make it
the finished file**, **Copy into the workspace**, and **Move to the recycle
bin**, which asks first. The file goes to your system's recycle bin and is
brought back from there: kilna's own undo does not reach a file on disk.

**Set a cover** attaches a file and marks it as the work's cover in one
step. Setting a second one changes the cover: the newest is the one that
shows, and the one it replaced stays attached rather than disappearing
behind your back. Remove it and the older one is the cover again.

Where the cover is drawn rather than photographed, it is built on the card's
**Cover** tab - see [The cover](/kilna/guides/the-cover/). The pictures a
generator gives back land there as **candidates**, and one is made the cover;
here they are a group of files of their own.

## The cover's shape

The place a release goes out decides the shape of its picture: YouTube wants a
16:9 preview, Shorts a 9:16 frame, a streaming service a square. The shape is
the kind of release's (`cover_format` in the
[profile](/kilna/reference/profile-document/)), so a work that goes out in two
places needs two covers. The **Cover** tab of a clip, an audio release or a
short offers the shapes of the doors its kind has, and says of a shape it
does not go out through yet that it does not.

## The frame

An audio release on a video platform is a track with a picture on it: one
still that stays on screen for the whole song, and a short loop of what moves
in it - a leaf drifting across, dust turning in a beam. A kind whose works
play under one (`"frame": true`; Studio's **Audio**) has a **Frame** tab:

- **the still** - what the picture shows, as you would describe it to a
  generator;
- **what moves** - the one thing alive in a picture that otherwise holds
  still;
- **the loop's length** - 4, 6, 8 or 10 seconds;
- **the camera does not move** and **seamless: the last frame matches the
  first** - two switches, on by default;
- **the negative** - what must not appear.

Since v0.88 the still is **built from the cover** by default - the same idea,
hero, style, ground and mark, without the title - and what you write is added
to it as your own words; **own scene** keeps the still as you write it,
whole. See [The cover](/kilna/guides/the-cover/#the-frame-of-an-audio-release).

Beside them, three blocks to copy into the generator - the still, the loop and
the negative. The loop's block is written for you from the settings - *LOOP
(6 s): the beam turns. The camera does not move at all. Seamless loop: …* -
because a still camera and a seamless join are instructions a model reads,
not a mood it guesses from the picture. Each setting says its sentence and
nothing else does. See
[ADR 0046](https://github.com/lacodda/kilna/blob/main/docs/adr/0046-a-frame-is-a-still-and-a-loop-written-from-its-settings.md).

## Where the file goes

**A file is copied into the workspace**, not pointed at where it lay. The
copy sits in a `media` directory beside the workspace database, named by an
id of its own.

This is the difference between a cover that keeps working and one that
breaks the day you tidy your downloads folder. It also means the pictures
travel: **a backup takes the files with the database**, into a directory
named after the backup, and restoring brings both back. A workspace is one
folder, and moving it is moving a folder.

The name the file arrived under is kept and shown — it is what you
recognise it by, and often what a generator wrote in it. Inside the
workspace the file is named by its id, so two files called `cover.png`
cannot collide.

Removing a file sends it to the [trash](/kilna/guides/the-trash/), copy and
all: **Undo** brings it back, and so does **Restore** on its row there, until
the trash is emptied. The same goes for a reference taken off a style and a
picture taken off a card of the canon. The original, wherever it came from,
is never touched.

## Where a cover shows

Everywhere a work is drawn small:

- the banner at the top of its card,
- beside its title in the catalogue,
- on its chip in the calendar,
- on the dashboard and in the search palette.

A work with no cover keeps the colour it has always had — a gradient derived
from its id, so a row of works reads as one family. The picture is laid
*over* that colour rather than replacing it, so a cover still loading, or
one whose file went missing, shows the work's own colour instead of a white
hole.

## Files in groups

The tab shows a work's files grouped by what they are for: the cover first,
then the candidates for it, then the stills its scenes are drawn from, then the clips animated from
those, then everything else. A group with nothing in it is not drawn at all.

The grouping earns its place on a storyboard. Scene stills and clips are
files on the work like any other, so a board of fifty scenes with four
candidates each puts two hundred pictures in this tab — in one flat list the
cover would be somewhere in the middle of them. Scene material is normally
handled on the board itself (see [Scenes](/kilna/guides/scenes/)); this tab
is where you see all of it at once, and what it costs.

A file may belong to more than one work: copying a board points the copy at
the same pictures rather than duplicating them. Removing it from one work
leaves the other looking at it, and the bytes go only when the last work
using them lets go.

## The work's folder on disk

Most of what you make for a work never needs to be inside kilna: the renders,
the takes, the stills a generator gave back, the mix you exported last night.
They live in a folder of your own, written by other programs. Since v0.93 the
**Files** tab shows that folder under **On disk** - found by its name, with
nothing to attach by hand.

Two things say where it is:

- **The media folder** - where this workspace keeps its media on this
  machine. Choose it in **Settings → Data**. It is a fact about the machine,
  like which profile is open: it stays out of exports and the profile,
  because on the next computer the same media lives somewhere else. The files
  in it stay yours - a backup or an export does not copy them.
- **The kind's folder** - how a work of the kind names its folder under the
  media folder, set per kind in the profile editor (**Folder on disk**), or as
  `folder` in the [profile document](/kilna/reference/profile-document/#a-works-folder-on-disk).
  `{id}` reads the work's id and `{kind}` its kind; `{title}` reads the
  work's title, `{key}` one of its fields, and `{origin.title}` or
  `{origin.key}` the same of what it is made from - the song, for its clip
  and its shorts. `{n}` is the work's number among the works of its kind made
  from the same song - the 2 of *Tide (short 2)* - so each short can keep a
  folder of its own.

Every profile kilna ships names a folder by the work's id, one per work, flat:
`{kind}s/{id}` in Studio - `songs/<id>`, `videos/<id>`, `audios/<id>`,
`shorts/<id>`, `experiments/<id>`. A folder named by the id stays where it is
when the work is renamed, which a folder named by the title did not (the
shipped templates until v0.120: `songs/{origin.title}`, `.../video`,
`.../shorts/{n}`). The id is not a name to read - you get to the folder with
**Open the folder** on the Files tab or in a catalogue row's menu, not by
looking for it.

Inside a work's folder, the top is for what is finished - the render, the
master, the cover - and what it was made out of lies below: `raw/`,
`scenes/`, `images/`, `drafts/`, `candidates/`. The names are a habit, not a
rule kilna enforces; what holds is that the [finished file](#the-finished-file)
is looked for at the top and never in a folder below, so a take in `scenes/`
is never taken for the finished clip.

A layout of your own is one template away:

| Kind | Folder | A song with the field `code` = `h042` |
| --- | --- | --- |
| Song | `songs/{code}` | `songs/h042` |
| Video | `songs/{origin.code}/clip` | `songs/h042/clip` |
| Audio | `songs/{origin.code}/audio` | `songs/h042/audio` |
| Short | `songs/{origin.code}/shorts/v{n}` | `songs/h042/shorts/v2` |

A work's number is given when it is made from another, past the greatest one
given in its family - a short deleted leaves a gap, never a number given
twice, so a folder named after it is never another short's. The works made
before v0.101 took the number their title carried. When the folders on disk
say otherwise - the second short's files lie in `v3` - type the number over
beside the folder's path on the Files tab.

The code can be one kilna gives by itself. A
[numbered field](/kilna/reference/profile-document/#numbered-fields) -
`"numbered_from": "h001"` - gives each new song the next code as it is made,
so its folder has a name before the first file is rendered; the songs made
before the field was numbered get theirs from **Next code** on the overview,
or all at once from **Fill "Code"** on the catalogue's bar.

A field that a template reads and a work leaves empty is said by name rather
than guessed around. A title is made safe for a folder's name - the
characters no folder may hold are left out - so `AC/DC: Live?` looks in
`ACDC Live`.

**Looked at, never owned.** kilna does nothing to a file in the folder by
itself. It lists what is there, arranged as
[the bar](#looking-through-the-files) says, and a file is
[looked at](#looking-at-a-file) where it lies. **Copy into the workspace** is
how a file of the folder becomes the work's own - a cover, a reference - with
everything [above](#where-the-file-goes) that a copy brings. Two things
change the folder, and each only when you ask for it by name: a file is
renamed, and a file is moved to the recycle bin
([above](#what-can-be-done-with-a-file)).

The one thing kilna makes there is the folder itself, and only when it is
needed. A work is made without one - a year of planned works does not leave a
few hundred empty folders on the disk - and **Open the folder** makes it and
opens it in the file manager, there to drop the first file in. (Until v0.120
a work whose folder was not there offered **Make the folder** first.)

And the one thing kilna removes is a folder with nothing in it: when a work
goes to the trash - deleted, or its making undone - a folder that holds no
file goes with it, and comes back the next time it is opened if the work is
restored. A folder with one file anywhere under it is never removed: not by
deleting the work, not by emptying the trash, not by an undo.

The list is read again whenever you come back to the window, so a render
finished in another program is there when you return.

### Moving the folders to a new layout

A folder is found by its template, so a template changed in the profile
points every work at a new place at once - and the files stay where the old
one put them. A space made before v0.120 keeps its templates for that reason.
`kilna --rehome-media` moves the files and switches the templates together,
to the layout the shipped profile names:

```sh
kilna --rehome-media                      # a plan: nothing is moved
kilna --rehome-media --also ~/Downloads   # ...looking into another folder too
kilna --rehome-media --apply              # carry it out
```

It prints the folder of its report - `migration_plan.md` to read,
`migration_plan.json` whole - under `rehome/` in kilna's data folder. Read the
plan before applying it; `--space <name>` plans for a space other than the
last one open, and kilna's window should be closed when it is applied.

- Every file in a folder the stored template finds for a work moves to the
  work's new folder and keeps its place inside. Where one work's folder lies
  inside another's - a clip's inside its song's - a file belongs to the
  deepest work that holds it.
- What lies where the old templates wrote (`songs/` of `songs/{title}`) and
  belongs to no work goes to `_unsorted_migration/` by the path it had.
  Nothing is deleted.
- A folder under `--also` is taken in only when it is called exactly what one
  work is called: what is finished at its top (a clip, a sound, a picture
  named `cover…`) goes to the top of the work's folder, the rest under
  `raw/`. Everything else in that folder is left alone.
- A file that would land on one already there is not written over: it goes
  to `_unsorted_migration/_conflicts/`.
- `_shared/`, anything beside the old templates' folders, and a folder two
  works both resolve to are left exactly as they are, and the report says so.

Applying takes a copy of the database into the report's folder first, moves
the files, removes the folders the moves emptied, and only when every move
landed switches the templates. Run again after a failure, it plans from the
disk as it now is and finishes.

See [ADR 0057](https://github.com/lacodda/kilna/blob/main/docs/adr/0057-a-works-folder-on-disk-is-found-and-looked-at.md).

## The finished file

Of everything in a work's folder, one file is the work itself: the rendered
clip a short is cut from, the short a cutter made, the mixed track. Since
v0.101 kilna knows which.

- **The kind's rule** finds it: **Finished file** in the profile editor, or
  `file` in the [profile document](/kilna/reference/profile-document/#a-works-finished-file) -
  a path inside the work's folder where `*` stands for any name and `?` for
  one character. The newest file it matches is taken. Studio ships `*.mp4`
  for a clip, an audio release and a short; a clip kept as
  `clip/v1/video/v2.mp4` is `v*/video/*.mp4`.
- **Your choice** stands over the rule: **Make this the finished file** on a
  clip's or a sound's tile. A chosen file that is gone is said, not replaced
  by a guess; **Back to the rule** gives the choice back.

The Files tab says above the files which one it is and how it was found, and
marks its tile. A short's [cut list](/kilna/guides/shorts/#the-cut-list)
hands the donor's finished file to a cutter, and a file a cutter made becomes
the short's own when the rule would not have found it.

See [ADR 0070](https://github.com/lacodda/kilna/blob/main/docs/adr/0070-a-work-is-finished-as-a-file-in-its-folder.md).

## The player

A work with a sound - attached, or in its folder - gets a player at the top of
its card, under the title: play and pause, the file's name, a bar to move
along, and the time. It stays while you switch tabs, so a song plays while
you read its text or give it a score.

The newest sound comes first: the take finished last is usually the one you
opened the card to hear. When there are several, the name opens a list of all
of them, each with the folder it lies in. A sound's tile on the **Files** tab
plays in the same player.

kilna plays MP3, WAV, FLAC, M4A/AAC, Ogg and Opus.
