# 0084 — A frame places a subject, not a hero, and says geometry alone

Date: 2026-11-17
Status: Accepted

## Context

A cover's prompt is written from what is chosen (ADR 0049), and its frame -
the layout, the position, the size, the crop, the title's place - is built
into the code. Every sentence of it was written for one picture: a person,
head and shoulders, in an emblem of symbolic objects with a disc behind.

So every cover got that picture's words. With no hero chosen the frame still
said "place the hero", put the title "behind the hero's head" or "across the
hero's chest", and a close-up asked for "marks on the skin, reflections in
the eyes". A cover of an empty street was told where the hero's face goes.
Five layouts of the eight added "a large flat circle in {accent} sits behind
the hero like a moon" to any cover with an accent. A background brick's own
description says "perfectly flat, no scenery", and went in beside a scene
that described a place. A detail of the channel written in Russian went into
the English prompt as it stood; a lettering was described on a cover with no
title to letter; an empty slot inside `[brackets]` - an offer - was reported
as a problem; and a title sent to a second prompt still had its room kept in
the first.

The frame was also said twice, as PLACEMENT near the top and COMPOSITION
after the style, and the two could disagree.

## Decision

**A frame places a subject, and the subject is one of three.** `Subject` is
a person, a thing, or nobody. A cover with a hero from the canon is of a
person; without one it is what `staging.subject` says, and a thing when that
says nothing. Each sentence of the frame is written per subject: a person is
"the hero" and has a head and a chest; a thing is "the main subject" and has
neither; a picture of nobody places nothing and frames the scene. The words
*hero, head, chest, face, figure, eyes, skin* are said only of a person.

**A frame is geometry and nothing else.** Where, how big, how much shows,
how the frame is composed. The motifs every layout added - interlocked
symbolic objects, splatters and doodles, marks on the skin - are gone: what
is in the picture is the scene's to say, and how it is drawn is the style's.

**One paragraph.** `FRAMING:` says the position, the size, the crop, the
composition, the accent and the part kept for a title, in that order.

**The accent is a colour unless a disc is asked for.** `staging.disc`, off
by default, in every layout when on.

**A background is a palette unless taken flat.** `staging.ground`: palette
by default - the brick's name and colour set the picture's colours, the
scene keeps its place - or flat, where the brick's description is said as
written.

**What has nothing to say is left out and said in the tab.** A detail with
no words for a generator (`DetailNeedsTranslation`), a lettering on a cover
with no title (`TypographyWithoutTitle`). Only a slot outside brackets is a
`SlotEmpty`. A title that goes apart keeps no room in the first prompt
unless `lettering.reserve` asks for it. The sentence "the lettering matches
the picture's style and colours" is not added to a lettering.

**The same rules for every picture.** The frame under a track and a scene of
a clip are written by the same function with the same subject: a frame takes
its cover's, a scene is of a person when it names a character and a thing
otherwise. A frame's own words for the still are said with the scene.

**The prompt's text is held to the letter.** Ten pictures are written out
whole into `src-tauri/tests/golden/prompts/` and compared on every run; a
change to how a prompt is built shows as a diff of those files.

## Consequences

- Covers built before v0.115 read differently: no disc unless switched on, a
  palette instead of a flat ground, no motifs, "the main subject" where no
  hero is chosen. The prompt last copied out is kept on the cover (`sent`),
  and the tab says it has moved on.
- The new fields are nested under `staging`: the search reads the words at
  the top of a cover's record, and a subject is not a word of the cover.
- A scene has no subject of its own to choose yet; FR-07 7.2 gives the frame
  and the scene their own overrides.
- The scheme draws a thing as a shape and nobody as nothing, and a close
  look at a pair of eyes inside the frame rather than at twice its height.
