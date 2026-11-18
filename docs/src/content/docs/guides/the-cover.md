---
title: The cover
description: Building a publication's cover from parts - the idea, the hero, a built frame, the styles, the channel's mark and details - starting from a board of ideas, and the prompt written from them.
---

A clip, an audio release and a short go out under a picture, and the picture
is drawn by a generator from a prompt. The **Cover** tab builds that prompt
from parts instead of having you write it whole: what the cover says, who is
on it, how they stand in the frame, the styles it is drawn in, the channel's
mark and signature details. Every choice is kept on the publication, and the
prompt is written from them on the spot.

The tab has two halves, switched at its top: **Ideas**, a board of ideas for
the cover, and **Constructor**, where one is built. A cover with nothing in it
opens on the ideas; one that holds anything opens on the constructor.

The constructor has two columns. On the left, what the cover is built from.
On the right, what that makes: the frame drawn in the shape of the release,
the prompt to copy, the files to hand the generator, and what came back.

## The board of ideas

There is no wizard: every cover starts here or straight in the constructor.

**Your idea** is optional - a sentence of what the cover could show, in your
own words. **Generate** puts it on the board as your own, and asks the
assistant for ideas: as many of its own as you pick (0 to 5), and, with
**Work my idea out** ticked, your idea worked out into one more - your hero
and your moment, with a frame, objects, a style and a mark added. With no
ideas asked from the assistant, the button just puts your idea on the board.
The eye beside it shows what will be sent before anything is.

While the assistant writes, skeletons stand where the ideas will be, with the
time it has taken and **Stop**. The ideas land on the board as the answer
comes, marked *new*.

Each card shows where the idea came from - yours, worked out, the
assistant's, a neighbour's - and its angle, the frame drawn small with the
style's picture in its corner, its headline and idea, and what it is built
from: the style and the ground, the layout and the status of the mark.
Three things to do with it:

- **★ To the shortlist** - the ideas you like; they also show the direction
  for more;
- **✕ Not that** - turned down: the card steps back, and the assistant is told
  to stay away from it. A turned-down idea can be taken off the board, into
  the trash;
- **Into the constructor** - the cover becomes what the idea decides: its
  idea, its scene and what the scene keeps out, the hero, the frame, the four
  bricks, the accent, the mark's variant and the captions. What belongs to
  the publication stays: the lettered title, where and how the mark goes, the
  switches of the channel's details and your own words. An idea of words
  alone changes only the idea.
  It is an ordinary edit of the cover - undo takes it back.

**More in this direction** asks for new ideas with the shortlist as examples
of what is wanted and the turned-down ones as what is not.

**From neighbours.** The covers of the other publications of the same song -
the clip's, the audio's, a short's - lie on the board as cards of their own,
fitted to this publication's shape: in a tall frame a layout built around a
side is centred, and a title kept to a side goes to the top. Taken into the
constructor, the neighbour's cover is copied, not linked; starred or turned
down, it is copied onto the board as it stands.

The filters - all, the shortlist, mine, from neighbours, turned down - count
what each holds. In the constructor, the shortlist stands over the idea:
one press builds the cover from another starred idea.

**Make… starts the ideas.** Making a clip, an audio or a short from a song
asks for ideas for its cover beside its release meta, and lands on the board
while both are written. How many is a setting of the profile - three unless
it says otherwise, 0 for none - in **Settings → Profile**.

### How the assistant thinks of a cover

The action is the profile's - **Cover ideas** in Studio, with its method in
**Settings → Skills**. It is given what is asked and what stands on
the board, the song the publication is made from, the canon a picture may
see, and everything a cover is built from, each with the id to name it by:
the layouts and their settings, the bricks with when to reach for each (the
channel's house styles marked), the palette, the variants of the mark with
what each means, the heroes of the canon, and what the channel never shows in
a picture. Its method asks for one moment of the song through one hero,
drawable objects from the song's own images, ideas that differ by angle
rather than colour, the mark's status chosen by the song's meaning, the house
styles more often, and never anything the channel bans.

It answers with a block of concepts. Every name in it is looked up by id or
by name; one that is not in the workspace is left out of its idea and said,
and the rest of the idea lands. See
[ADR 0050](https://github.com/lacodda/kilna/blob/main/docs/adr/0050-an-idea-is-a-concept-on-the-publications-board.md).

## What a cover is built from

**Idea.** What the cover says, in your own words and language. It is never
sent to a generator; it is what you write the scene from.

**Scene.** What the picture shows, in English, for the generator.

**Keep out.** What this scene must not show beyond the channel's bans, in
English: what the scene itself risks - a detail the song's lines invite and
the channel never shows, a word a generator's filter trips on. It belongs to
the idea: it comes into the constructor with the scene, and goes into the
negative of the cover and of the still drawn from it, after the channel's
bans and before your own words.

**Hero.** Either the one the scene describes, or a card of the
[canon](/kilna/guides/the-canon/). A card brings its description for a
generator - written from its facts - and, if you tick it, its pictures as
references. Only public cards are offered: a cover reads the public layer of
the canon and nothing else. A card with no description yet is not named in
the prompt; the tab says so, and the place to fix it is the card.

With no card chosen, **The picture is of** says what the frame places: *a
person* the scene describes, *a thing*, or *nobody*. It decides the frame's
words. A person is "the hero", with a head a title can stand behind and a
chest it can cross; a thing is "the main subject" and has neither; a picture
of nobody places nothing and frames the scene - *a wide view of the entire
scene*, *the focal point sits in the centre*. A cover with no hero never says
*hero*, *head*, *chest*, *face*, *figure*, *eyes* or *skin*, in any block
(since v0.115; until then every frame said "the hero" whatever the picture
showed). Unset, the picture is of a thing.

**Frame.** Eight built-in layouts - an emblem on the right or the left, a
centre at full height, a hype poster, a title behind the head, a close-up, a
figure in the void, a split. Picking one sets the rest to its own: where the
hero stands on a three by three grid, how big (from small to past the edge of
the frame), how much of them shows (full figure to a detail) and where the
title goes (nine places). Each can then be moved on its own.

A frame is geometry: where, how big, how much shows, how the frame is
composed. It says nothing of what is in the picture - that is the scene's -
or of how it is drawn, which is the style's. The layouts used to add their
own motifs to every prompt (symbolic objects interlocked, splatters and
doodles around them, marks on the skin in a close-up); they do not since
v0.115.

**Styles.** The picture's style, the typography and the dressing - the small
captions around the title - are bricks of the
[style dictionary](/kilna/guides/styles/), and the background is one of its
colours. A place asks for a kind of brick by what it is made of: the style is
any picture brick, the typography a lettering, the dressing a dressing. The
channel's house styles come first in the list.

A chosen background is the picture's **palette and mood** - its colours set
the picture, and the scene keeps the place it describes - or, switched to
**flat ground**, the whole ground as the background's description says it: a
studio backdrop with no scenery. Palette is where it starts: a scene on a
street was otherwise told, two lines later, to have no scenery.

**Title.** The words lettered on the cover are the song's, not the
publication's - "the song - clip" is a name for the catalogue. Write other
words, or tick *no title* for a cover of captions alone. A lettering letters
a title: on a cover with none it is left out of the prompt, and the tab says
so.

**Captions.** A dressing names slots - `{brand}`, `{micro}`, `{num}` - and
each is filled with the cover's own lines first and the channel's card
second. A slot with nothing in either drops out of the prompt with the
phrase around it. The tab says which only for a slot the dressing cannot do
without: one written in `[square brackets]` is an offer, and an offer nobody
took is nothing to report.

**Accent.** The colour the composition and the lettering lean on, with the
words a generator is given for it: from the channel's palette, from the
**Accent** styles of the dictionary, or written by hand. An accent of the
dictionary may be a gradient - *Sunset gradient*, *Gold foil*, *Chrome* - and
the prompt then names every stop in order. A colour typed by hand is one
colour.

The accent is a colour - *used for highlights, light edges and key accents* -
unless **A flat disc of the accent behind the subject** is switched on, which
asks for the circle and draws it in the scheme. Off by default: five of the
eight layouts used to put that circle behind every picture with an accent.

**Mark.** A variant of the channel's mark - the status whose meaning fits the
song - where it goes (a corner, hidden in the picture, none) and how it gets
there. **Laid over** by default: a generator distorts a logo, so the mark's
own file is put on the final picture when it is exported. **Drawn**, its
description goes into the prompt and its file to the generator as a
reference. Hidden in the picture, it can only be drawn.

**Signature details.** The channel's card lists its signature details, each
with where it belongs - covers, frames, scenes - and whether it is on by
default. They appear here as switches, in the card's order. A switch starts
where the channel puts it, and the cover remembers its own. A detail goes
into the prompt as its template says it, in the generator's English; one
whose words are in the channel's own language is marked, left out, and named
among the tab's notes until its template is written in the channel's card.

## The constructor's column and dock

Since v0.135 the starred ideas stand at the top of the constructor's column
as a list - where each came from, its headline and a line of it - held there
while the column scrolls, one press to build the cover from one; a long
shortlist is searched. Every section folds by its title, and a folded
section says in one line what is chosen in it; which sections are folded is
remembered for the whole application, not per work.

The frame's layouts are searched, a starred layout comes first on every
work, and **No frame** clears the frame in one press. How big the subject is
and how much of it shows are two scales of named stops; the second starts at
**No subject**, a picture of the scene alone. **Scene, look and optics**
holds a picker per layer the craft makes - look, environment, angle, pose,
lighting, optics, grading - each list filed under its sets, starred styles
first, opening on the one chosen, with what the contract says beside it.
The **Lettering** section says everything about the words in one place: the
title, where it goes, whether a second prompt letters it and whether its
room is kept, and the typeface.

A dock under the column stands however far it is scrolled: **Copy all**,
the second prompt, when the prompt was copied and whether it changed since,
**Check the cover prompt** - Studio's action that reads the built prompt for
contradictions and answers in the chat - and what came back, as a strip of
pictures to drop, paste or pick into.

## Layers and contracts

Since v0.134 a cover holds, beside its style, lettering, dressing and ground,
nine **layers** - each a brick of the type of its own name: composition,
environment, look, pose, angle, layering, lighting, optics and grading.
Studio ships the light, the lens and the grade (*Window light*, *Golden
hour*, *85 mm portrait lens*, *Macro lens*, *Bleach bypass*, *Black and
white*, and more); the pickers for all nine come with the next version of
the constructor.

A brick can carry a **contract**: what the picture must be for it to apply
(`needs` - `subject:person`, `crop:min:bust`, `scenery:yes`,
`title:present`), what it brings (`provides` - `color:mono`,
`camera:moving`) and what it cannot stand beside (`conflicts`). Crops run
detail < head < bust < waist < full, so a pose that needs the hero to the
waist does not go into a head-and-shoulders picture. One rule reads every
chosen brick in its order - the style, the ground, the layers, the
lettering - against the picture's tags and what the bricks before it
brought. A brick the picture cannot take is **left out of the prompt and
kept in the cover**: the constructor says which and why, with one button
that makes the change letting it in - show more of the hero, make the
picture of a person, take the ground as a palette, or take the brick that
stands in its way out. A monochrome style (`color:mono`) with an accent is
selective colour: the accent is the one colour in the picture.

New rules are data: tags in a set or a pack, never code.

## The frame and the prompt agree

The scheme on the stage is the frame the prompt asks for, drawn: the
subject's box - a figure for a person, a shape for a thing, nothing for a
picture of nobody - the title's blocks, the accent's disc when it is asked
for, the mark's corner. Both are
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

- **picture** - the frame's shape, its framing in one paragraph (where the
  subject stands, how big, how the frame is composed, the accent, the part
  kept for a title), the style, the hero, the scene, the ground, the details,
  the mark when it is drawn, and the title with its lettering and dressing;
- **negative** - what every cover keeps out, the channel's bans for pictures
  and the hero's, what the scene keeps out, and your own words;
- **typography** - only when *the title as a second prompt* is on: the
  picture then asks for no text and keeps no part of the frame for a title,
  and this block is an edit of the finished picture that letters it. **Keep
  the title's place clean in the picture** puts the sentence that keeps it
  back into the first prompt.

The tab warns when the title and the channel's mark are both sent to the
lower left corner.

How a prompt reads is held to the letter: ten pictures - with a hero, a
thing, nobody, a disc, a title apart, a frame, a scene - are kept whole in
the repository and compared on every change, so a reworded sentence shows as
a difference to read before it ships.

Under each block, **your own words** for it, added at the end as written. A
cover written by hand before the constructor existed is exactly its own words,
and nothing is added to it until you choose something on the left.

Every quotation mark in a built prompt is a plain one: *TITLE: the words
"Harbour lights"*. A generator draws the marks it reads, and guillemets or
curly quotes around a title - or around a sign the scene names - came out on
the picture beside the letters. Whoever wrote the part they stood in, the
title, a brick, the channel's captions or your own words, they reach the
generator as `"` (since v0.100.2). A cover written by hand keeps its marks as
you typed them.

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
a picture with nothing written on it. The still's text is your own words
about what it shows, and is said with the scene. **Own scene** keeps the still as you write it, whole - the way every
frame written before v0.88 stays.

## Movements in the loop

A frame's loop is built from **movements** of the dictionary's Motion branch
- up to five - read by their contracts like any brick: a camera push-in stays
out of a frame whose camera holds still, a blink out of a picture of a
thing. The loop says the kept ones in their order, then the person's own
words for what moves: `LOOP (6 s): rain falls...; the subject blinks...; a
curtain stirs.` A frame can also hold its own subject, hero, bricks, accent
and ground over the cover's; whatever it leaves empty is the cover's.

Since v0.136 the Frame tab picks them: **Add a movement** lists the ready
movements of the Motion branch, starred first, up to five; one that moves
the camera says so while the camera holds still, and the frame says why it
is left out with the one change that lets it in. The frame's layout uses the
same list, scales and **No subject** as the cover; **Own bricks** gives the
frame its own style and layers, starting from a copy of the cover's, and
switching it off draws the frame in the cover's again. A dock under the
prompts copies all three blocks at once and keeps what came back - a still
or an mp4 loop - among the work's files.

## A scene of a clip

A scene of a clip can take a built frame too, in its drawer on the
[board](/kilna/guides/scenes/): the cover's layouts without a title, written
around the scene's picture block in the clip's style, with the characters the
scene is about as its heroes. Until a layout is picked the scene's blocks are
copied as written.

Since v0.134 a scene has its own bricks over the clip's style - its light,
its pose - and its own movements, written as the shot's loop, as long as
the shot.
