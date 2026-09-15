---
title: Files and covers
description: Attaching pictures to a work — a cover that shows everywhere, references kept beside it, and why a file is copied into the workspace rather than pointed at.
---

A work can carry files: a cover, a reference, a picture of the thing. They
live on the card's **Files** tab.

## Attaching a file

Two ways in. **Attach a file** opens the picker; **drop files onto the tab**
does the same for as many as you drag at once. Either way kilna takes
pictures — PNG, JPEG, WebP, GIF, AVIF.

**Set a cover** attaches a file and marks it as the work's cover in one
step. Setting a second one changes the cover: the newest is the one that
shows, and the one it replaced stays attached rather than disappearing
behind your back. Remove it and the older one is the cover again.

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

Removing a file takes the copy with it. The original, wherever it came from,
is never touched. This is not the trash: what the trash promises is that a
deletion can be taken back, and a row restored beside bytes that are gone
would be that promise broken.

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
