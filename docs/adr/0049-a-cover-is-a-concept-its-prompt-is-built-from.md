# 0049 — A cover is a concept its prompt is built from

Date: 2026-10-01
Status: Accepted

## Context

Since v0.73 a work's cover was three blocks of text a person wrote and
copied into a generator - `picture`, `negative`, `typography` - named by the
craft in the profile (`cover_blocks`). The owner did not write them as prose.
They built each prompt from a page of parts: an image style, a lettering, a
dressing of small captions, a ground, a layout of where the hero stands and
where the title goes, the channel's signature details, its mark. The page
lived outside kilna; the choices were lost between sessions and only the
pasted text survived. By v0.87 every one of those parts was in kilna - the
style dictionary with its starter set, the canon with the channel's card -
and nothing put them together.

Two decisions of the owner frame the answer (2026-09-27): the layout of the
frame is **built in and not edited** - eight layouts, a position on a three
by three grid, six steps of size, how much of the hero shows, nine places for
the title, from which both the scheme and the prompt are drawn; and the
title is **drawn by the generator in the same prompt** by default, as a
second prompt only when asked.

## Decision

**A cover is a concept, one record per publication in `work.cover`, and its
prompt is built from it in code.** The concept holds the idea (the person's
words, never sent), the scene (the generator's English), the hero (a card of
the canon, or the one the scene describes), the built frame, the bricks of
the style dictionary by place - style, typography, dressing, background - the
accent, the lettering (the title, whether it goes apart, the cover's own
captions for the dressing's slots), the mark (a variant of the channel's
family, where it goes, and whether it is laid over or drawn), the switches of
the channel's signature details, and the person's own words for each block.

**The parts are the application's.** A kind says only whether its works have
a cover (`cover: true`), as it says whether they have a frame (ADR 0046).
Format 3 of the profile reads format 2's `cover_blocks` as the flag. A list
of block names in the profile would be a second truth about parts the code
writes, and one the code could not fill if a craft named them otherwise.

**The blocks written before are the cover's own words.** `picture`,
`negative` and `typography` stay at the top of the record and are written
after whatever is built. A cover with nothing chosen in the constructor is
one written by hand, and its prompt is those words exactly - nothing is
added to it, not even the channel's details on by default. The owner's 132
covers read as they did, and an operation logged before v0.88 replays.

**The frame is geometry, computed once.** `cover::framing` turns a frame
into the placement sentence and into the scheme's shapes from the same
numbers: the zone a title is kept to is one rectangle that both the title
blocks of the scheme and the "keep the left 40%" of the prompt are written
from; the hero's box is the size and the third the prompt names. The window
draws the shapes it is sent and computes none. A test runs every layout,
place, size, crop, position and shape, with an accent and without, reads the
numbers back out of the prompt text and checks them against the shapes.

**What the channel's card gives is read through the cover's lens** (ADR
0043), each section by its shape: details are the switches, marks the
family, a palette the accents, house styles the bricks offered first, slots
the captions, and a new shape, `bans`, the negative. Studio splits its bans
in two: the bans of texts (`bans`, read by public texts) and the bans of a
picture (`picture_bans`, read by the cover's lens) - the channel's bans were
mostly about the voice and moderation words, which have no place in a
negative.

**The frame of a track and a scene of a clip are the same constructor
without words.** A frame built from the cover takes its idea, hero, style,
ground and mark, in a layout of its own if one is picked, with the still's
text as its own words; a frame whose still was written whole stays so. A
scene with a built frame (`scene.framing`, migration 0033) is written around
the block the kind marks as the picture (`scene_blocks[].picture`), in the
clip's style, with the scene's characters as its heroes.

**The mark is laid over, not drawn, by default.** A generator distorts a
logo. The exported cover is composed in the window: the final picture and
the mark's own file, the mark in the box the scheme draws it in.

**What came back is candidates, and one cover.** A picture added on the
Cover tab is a `candidate`; one is chosen as the `cover`, and the cover it
replaces is a candidate again (`asset.chooseCover`, taken back by undo). The
catalogue, the calendar and the header show the cover, as before.

**A copied prompt is kept on the cover** (`sent`): what the picture was
drawn from, when the channel or a brick changes later.

## Consequences

- Migration 0033 adds `scene.framing`; the cover needs no migration of data.
- The idea generator of v0.89 writes concepts, not text: everything it
  proposes is a choice this constructor already shows.
- A cover is one record for every shape of its doors: an audio release on
  YouTube and on a streaming service is one concept seen in two shapes, and
  has one final picture. A final picture per shape waits for a work that
  needs two.
- The built-in sentences are the application's English. A channel that
  needs other wording writes its own words into the blocks, or its details.
