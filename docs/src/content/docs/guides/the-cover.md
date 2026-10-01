---
title: The cover
description: Building a publication's cover from parts - the idea, the hero, a built frame, the styles, the channel's mark and details - and the prompt written from them.
---

A clip, an audio release and a short go out under a picture, and the picture
is drawn by a generator from a prompt. The **Cover** tab builds that prompt
from parts instead of having you write it whole: what the cover says, who is
on it, how they stand in the frame, the styles it is drawn in, the channel's
mark and signature details. Every choice is kept on the publication, and the
prompt is written from them on the spot.

The tab has two columns. On the left, what the cover is built from. On the
right, what that makes: the frame drawn in the shape of the release, the
prompt to copy, the files to hand the generator, and what came back.

## What a cover is built from

**Idea.** What the cover says, in your own words and language. It is never
sent to a generator; it is what you write the scene from.

**Scene.** What the picture shows, in English, for the generator.

**Hero.** Either the one the scene describes, or a card of the
[canon](/kilna/guides/the-canon/). A card brings its description for a
generator - written from its facts - and, if you tick it, its pictures as
references. Only public cards are offered: a cover reads the public layer of
the canon and nothing else. A card with no description yet is not named in
the prompt; the tab says so, and the place to fix it is the card.

**Frame.** Eight built-in layouts - an emblem on the right or the left, a
centre at full height, a hype poster, a title behind the head, a close-up, a
figure in the void, a split. Picking one sets the rest to its own: where the
hero stands on a three by three grid, how big (from small to past the edge of
the frame), how much of them shows (full figure to a detail) and where the
title goes (nine places). Each can then be moved on its own.

**Styles.** The picture's style, the typography and the dressing - the small
captions around the title - are bricks of the
[style dictionary](/kilna/guides/styles/), and the background is one of its
colours. A place asks for a kind of brick by what it is made of: the style is
any picture brick, the typography a lettering, the dressing a dressing. The
channel's house styles come first in the list.

**Title.** The words lettered on the cover are the song's, not the
publication's - "the song - clip" is a name for the catalogue. Write other
words, or tick *no title* for a cover of captions alone.

**Captions.** A dressing names slots - `{brand}`, `{micro}`, `{num}` - and
each is filled with the cover's own lines first and the channel's card
second. A slot with nothing in either drops out of the prompt with the
phrase around it, and the tab says which.

**Accent.** One colour the composition and the lettering lean on, from the
channel's palette or written by hand, with the words a generator is given for
it.

**Mark.** A variant of the channel's mark - the status whose meaning fits the
song - where it goes (a corner, hidden in the picture, none) and how it gets
there. **Laid over** by default: a generator distorts a logo, so the mark's
own file is put on the final picture when it is exported. **Drawn**, its
description goes into the prompt and its file to the generator as a
reference. Hidden in the picture, it can only be drawn.

**Signature details.** The channel's card lists its signature details, each
with where it belongs - covers, frames, scenes - and whether it is on by
default. They appear here as switches, in the card's order. A switch starts
where the channel puts it, and the cover remembers its own.

## The frame and the prompt agree

The scheme on the stage is the frame the prompt asks for, drawn: the hero's
box, the title's blocks, the accent's disc, the mark's corner. Both are
written from the same numbers on the Rust side - the part of the frame kept
for the title is one rectangle that the blocks are drawn in and the sentence
"keep the left 40% of the frame free" is written from. A test runs every
combination of layout, title place, size, crop, position and shape against
it. See
[ADR 0049](https://github.com/lacodda/kilna/blob/main/docs/adr/0049-a-cover-is-a-concept-its-prompt-is-built-from.md).

The shape is the release's: YouTube's 16:9, Shorts' 9:16, a streaming
service's square. A work that goes out in two places shows both shapes; one
it does not go out through yet is offered and said to be so.

## The prompt

Three blocks, each copied on its own or all at once:

- **picture** - the frame, the placement, the style, the composition, the
  hero, the scene, the ground, the details, the mark when it is drawn, and the
  title with its lettering and dressing;
- **negative** - what every cover keeps out, the channel's bans for pictures
  and the hero's, and your own words;
- **typography** - only when *the title as a second prompt* is on: the
  picture then asks for no text, and this block is an edit of the finished
  picture that letters it.

Under each block, **your own words** for it, added at the end as written. A
cover written by hand before the constructor existed is exactly its own words,
and nothing is added to it until you choose something on the left.

Copying keeps the prompt on the cover: what the picture was drawn from stays
with it if the channel's card or a style changes later, and the tab says when
the prompt has moved on since.

## Files to hand over

The hero's pictures, the mark's file when it is drawn, the style's
references: each is listed with a button that shows it in its folder, to
upload beside the prompt.

## What came back

Drop the generator's pictures onto the **Result** panel, pick them with
**Add a picture**, or paste one with Ctrl+V. Each arrives as a candidate.
**Make it the cover** chooses one: it becomes the release's preview - the
picture the catalogue, the calendar and the card's header show - and the
cover it replaces is a candidate again. Undo takes the choice back.

**Export** saves the cover for the platform, with the mark laid over it in
its corner when the mark goes on that way.

## The frame of an audio release

An audio release's [frame](/kilna/guides/files-and-covers/#the-frame) is
built from its cover by default: the same idea, hero, style, ground and mark,
no title and no dressing, in a layout of its own if the cover's does not suit
a picture with nothing written on it. The still's text is added as your own
words. **Own scene** keeps the still as you write it, whole - the way every
frame written before v0.88 stays.

## A scene of a clip

A scene of a clip can take a built frame too, in its drawer on the
[board](/kilna/guides/scenes/): the cover's layouts without a title, written
around the scene's picture block in the clip's style, with the characters the
scene is about as its heroes. Until a layout is picked the scene's blocks are
copied as written.
