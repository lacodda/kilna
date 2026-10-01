---
title: Styles
description: One dictionary of the parts a picture prompt is built from — a look, a character, a place — each of a type that says what it contributes.
---

A picture prompt is not one idea. It is a render recipe, plus a character,
plus a place, plus the lettering, plus the angle the camera is at — and each
of those outlives the picture it was first written for. The same character
stands in thirty videos.

So kilna keeps them as **styles**: one dictionary, owned by the workspace
rather than by any work, that a prompt is built out of. Each style is a
brick of it.

## What a style is

A style has four things:

- **A type** — *Image style*, *Typography*, *Dressing*, *Background*,
  *Character*, *Look*, *Environment*, *Camera angle*, *Pose* in the Studio
  profile. The type is a word of the craft and lives in the profile
  document, so a craft that works differently names its own.
- **A name** — what you call it. Two styles of one type may not share a
  name; across types they may, so a character and a look can both be
  *Ranger*.
- **A description** — the text that goes into a prompt **word for word**.
  This is the whole of what a style contributes.
- **References** — the pictures it was written from. Drop them on the open
  style from a folder, paste one with `Ctrl+V`, or add them with **Add
  pictures**. They are copied into the workspace like every other file, so a
  style survives you tidying your downloads.

There is a fifth field, the **steer**, and it is the odd one: it says what
to look at when the style is *described* — "only the jacket", "ignore the
background". It reaches the assistant when it writes the description. It
never reaches a generator, because it is an instruction about the
description rather than part of one.

Beside those, a style says **when to take it** - "loud songs with one strong
figure; light grounds" - for whoever picks styles for a picture. The cover's
idea generator reads it. It is not the steer: the steer is about how a style
is described, this is about when it is the right one.

## What a style is made of

A type decides what its styles are made of, and so what their card shows:

- **Pictures** - an image style, a character, a place. The card leads with
  three references, or with the style's palette while it has none.
- **Lettering** - typography. The card draws a live sample of the typeface.
  The sample is for the eye only: on a picture the lettering is drawn by the
  generator from the description.
- **A dressing** - the small furniture around a title: labels, numbers,
  stamps, stickers. Its description names captions as slots - `{brand}`,
  `{micro}`, `{micro.0}` for the first of several - and the editor says which
  of them the channel's card fills. A phrase in `[square brackets]` whose
  caption is empty drops out of the prompt whole: *a tiny line "{brand}"*
  with no brand is not an instruction to draw an empty label.
- **A colour** - a background: the colour itself, picked or typed as
  `#RRGGBB`, and the description that says what the ground is like.

An image style is also filed under a **family** - *Tattoo*, *Classic*,
*Street*, *Digital*, *Eras* - and the dictionary narrows by it.

The channel's own - its mark, signature details, captions and palette - is
not a style: it lives on the **Channel** card of the
[canon](/kilna/guides/the-canon/).

## The starter set

A new workspace does not open on an empty dictionary. Studio ships ninety
styles - image styles in five families, typography with live samples,
dressings and backgrounds - with names in English and Russian and
descriptions in English, the language a prompt is written in.

They are yours from the start: edit, retire or delete them. Each card says
where its style came from:

- **from the set** - nobody has touched it. A newer kilna may improve its
  wording at the next start.
- **changed from the set** - you edited it. It keeps your words; **Restore as
  in the set** in the open style puts the set's back, and `Ctrl+Z` takes that
  back too.
- **your own** - one you made.

A style of the set you delete stays deleted. Narrow the dictionary by where a
style came from, and by where it stands, with the chips under the types.

## Types the craft no longer builds from

*Layers* and *Frame composition* are retired in Studio: where the hero
stands, how big, what sits over what and where the title goes is set by the
cover constructor now. Styles you made of them still read, with their word;
new ones are not made, and the type's chip says why.

A hero with a card in the canon needs no *Character* style either: the card's
description is built from its facts. The type stays for heroes without one.

## Making and editing a style

The dictionary is a grid of cards, each led by up to three of its pictures,
and the last card is dashed: **New style**. Drop pictures on it from a
folder, or paste one with `Ctrl+V` while no style is open, and a new style is
made around them and opened, called *Untitled style* with the name ready to
be typed over. Pressing the dashed card picks the pictures from disk instead;
**New style** at the top makes one with none.

A style opens beside the dictionary rather than over it: the grid narrows to
a column, so the other styles stay in reach, and the open one takes the rest
of the window. There is no Save button - every field writes itself a moment
after you stop typing, and the line beside the name says when it has. A
picked type or status is written at once. `Escape` or the cross closes it.

A name the dictionary cannot take - none at all, or one another style of the
type already has - is said under it, and the style keeps its last name until
it has a new one; the rest of what you type goes on saving meanwhile.

## The type is the point

The type is not a label for sorting. It carries a **hint**: what to describe
for a style of that type. Given the same photograph, `image-style` asks for
the render technique and `character` asks for the person — and never the
other way round.

That is what makes one dictionary richer than several flat ones. An
assistant told *"Environment: a flooded car park at dusk"* knows the
sentence is the place and not the person, and can write one prompt instead
of gluing three together.

The hint shows under the type picker while you are choosing, because it is
what the description will be held to.

## Where it stands

A style is a **draft** until it carries a description, **ready** once it
does, and **retired** when you put it away without losing what it was.

Only ready styles are offered when a prompt is being built. A draft is
unfinished by definition, and a picker quietly full of things nobody has
touched is how a dictionary rots.

Writing a description — by hand or from the references — lets a draft out
into ready in the same move. Describing a retired style does not revive it.

## Describing from references

Add the pictures, pick the type, and press **Describe from the
references**. The assistant is given the type's own question, your steer,
the pictures and whatever the style says today, and it writes the
description that style will carry. Whatever you typed is saved first, so the
description is written from the style as you left it — the steer you typed a
moment ago included. The style stays open while it is written.

The answer arrives in its own chat, the way every AI action's does, with
**Keep as its description** under it; the bell says it is waiting, and its
line opens that chat. Nothing is written until you keep it — the assistant
never touches the workspace itself. Kept, the answer becomes the style's
description word for word, lets a draft out into ready, and is an edit like
one made by hand: **Ctrl+Z** takes it back. A style open at the time shows
the kept description in its box, unless you are typing in that box yourself.

With no references at all, the prompt says so rather than pretending: it
asks for a description from the name and says plainly that there was
nothing to look at.

## Deleting a style

The bin beside the open style's name moves it to the
[Trash](/kilna/guides/the-trash/) together with its reference pictures, like
every other deletion: the message offers *Undo*, and the Trash restores the
style with its pictures for as long as you want.

To put a style away without deleting it, set it to **retired** instead: it
keeps its description and pictures and stops being offered.

## Building a prompt

On a video or a short, **Build a prompt from styles** — in the card's AI
actions — opens the picker: search the dictionary, click the parts you want,
and press build. Only **ready** styles are offered.

The action takes the styles you picked and writes one prompt out of them.

Each part reaches the assistant under the word for what it contributes:

```
Environment — Flooded car park
Standing water to the ankles, concrete pillars, sodium light from one working lamp.

Character — The keeper
Sixty, weathered, close-cropped grey hair, a long grey coat worn open.

Image style — Cold north
Grainy monochrome film, sodium streetlight, heavy vignette.
```

**The order you pick them in is kept.** A person who picks the character
first and the place second has said something about which matters more, and
re-sorting it would throw that away — so the first is the spine of the
picture and the last is a detail.

A style with no description yet still goes in, by name, marked as not
described. A silent gap would read as *"there is no character"*, and you
picked it on purpose.

## A style is not a style prompt

The `style` **version role** on a song — the production paragraph kept
beside its lyrics — is unchanged and is not the same thing. That is a
finished prompt for one work. A style is a part that finished prompts get
built from. The constructor turns the second into the first.

## Where the dictionary lives

**Styles** in the rail, under *Library*. The screen appears only where the
craft names style types; a profile with none has no dictionary, and kilna
does not offer a door to an empty room.
