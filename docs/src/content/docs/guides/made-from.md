---
title: Made from
description: A video from a song, a video from an article — how one work is linked to what it was made from, and what kilna says when the source moves on.
---

A video is made from a song. An article becomes a video, a novel an
audiobook. kilna keeps the link: in the song you see its clips, in the clip
you see the song, and either can start the other. The link is also how a
song goes out at all. A song is the thing - its text, its style, its score -
and it has no release of its own: its clip, its audio release and its shorts
are works made from it, each going out through its own doors (since v0.86;
see [Planning a release](/kilna/guides/planning-a-release/)). They are the
song's **publications**.

## A song's publications

A song's **Overview** leads with its publications: each with its kind, where
it stands - *Out 22.07*, *Booked for 03.10*, a draft - its comments, and, for
a short cut from the clip, which clip. Under the list, what the song's status
stands on (*Out as "Harbour lights — audio" on 22.07*) and how many comments
its publications gathered. The song's own status is read from them: it is out
when any of them went out, booked when any holds a day. See
[Statuses](/kilna/guides/statuses/).

**Make…** above the list makes one - a clip, an audio release, a short - in
one step:

- the new work is named the way its kind names what is made from a song, in
  the language the window is in: *Harbour lights — clip*, *Harbour lights —
  audio*, *Harbour lights · short 5* - numbered among the works of that kind
  already made from the song;
- it takes the song's overview fields its kind has, and the ones its kind
  starts with (an audio release starts as the *original* variant);
- it is linked to the song at the version the song is on;
- it is planned one release through its first door - YouTube for a clip or an
  audio - with no day yet: making it means you are going to put it out;
- when the profile has a **Release meta** action, it starts in the background
  on that release and writes what it goes out under - the title, the
  description, the tags, the pinned comment - in the channel's voice (see
  [Planning a release](/kilna/guides/planning-a-release/#what-it-goes-out-under));
- the new work opens on its **Cover** tab.

## Making a work from another

On any card, the **Links** tab ends with one button per other kind of work
your profile has — *Make a Video from this*, *Make a Short from this*. The
header menu offers the same. The new work opens at once, with:

- a **title** from its kind's `made_title` - or the source's title, when the
  kind names none - to rename if you like;
- the source's **overview fields** that the new work's kind has — mood,
  duration, language, premise — copied **once**, now, and never again;
- a link to the source in the **donor** role, remembering which version of
  the source it was taken at;
- for a kind that goes out somewhere, one release with no day yet.

Its text is not copied. A video's roles are its own — a plot is not a
lyric — and the source is one click away on the same tab.

The other direction works too: narrow the [catalogue](/kilna/guides/the-catalogue/#narrowing-the-list)
to *Video* and **Add** makes a video; its Links tab then names a source by
typing a title — or names none. A work can have several sources, and a
source can have several works made from it.

## When the source moves on

The link remembers the source's version at the moment it was made. When the
song gains a new version, or the very version it was taken at is edited,
the clip's Links tab marks the source **changed since r2 → r4** with a way
to the diff of the source's own versions, both selected.

Nothing else happens. Scenes are not rewritten, fields are not refreshed,
the status does not move: the fact is stated, the decision is yours — the
same rule a [stale score](/kilna/guides/scoring-a-work/) follows. Remove the
link and add it again to take the source at its current version.

## What else the link touches

- The **history** says *"Harbour lights" is made from "Harbour lights"* when
  a link is made, and when it is removed.
- The **trash** keeps links with the work it takes down, from either side:
  restore the song and its clips are linked again, restore the clip and it
  knows its song. A link whose other work is still in the trash waits for
  it.
- The **export** writes a *Made from* section on the work's page.
- An agent reading the card over [MCP](/kilna/reference/mcp/) sees
  `sources` — with whether each has moved on — and `derived`.

The link is one table for every role a link will ever have: *donor* today,
a remix or a sequel later without a change of shape. See
[ADR 0019](https://github.com/lacodda/kilna/blob/main/docs/adr/0019-a-link-between-works-carries-a-role-and-the-version-it-was-taken-at.md).
