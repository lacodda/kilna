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

The dictionary has two halves. **Picture** holds what a cover, a frame and a
scene are built from, and is what this page is about. **Sound** holds the
phrases a song's style prompt is written from - see
[The dictionary of sound](/kilna/guides/the-sound/). The switch at the top of
the screen moves between them.

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
- **Colours** - a background, and an accent: one colour picked or typed as
  `#RRGGBB`, or up to four in order, which make a gradient. The card paints
  the gradient, and the description says it in words, stop by stop, since
  that is what the generator reads. A background is the ground a cover stands
  on; an accent is offered as the cover's accent, beside the channel's
  palette.

An image style's palette and the ground of a lettering sample are edited the
same way, one box per colour.

An image style is also filed under a **family** - *Tattoo*, *Classic*,
*Street*, *Digital*, *Eras* - and the dictionary narrows by it.

The channel's own - its mark, signature details, captions and palette - is
not a style: it lives on the **Channel** card of the
[canon](/kilna/guides/the-canon/).

## The starter set

A new workspace does not open on an empty dictionary. Studio ships three
hundred and three styles for pictures - image styles in twelve families,
typography with live samples, dressings, backgrounds and accents, flat and
in gradients - and two hundred and twelve phrases of sound, with names in
English and Russian and descriptions in English, the language a prompt is
written in. The
families run from tattoo and the old masters through the street, the digital
and the eras to Japan, China, India, Eastern tales, world folk, antiquity
and what is in fashion now.

A newer kilna may bring more of them. They reach a workspace that already
exists at the next start, and so do the families they are filed under.

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

## Packs from a registry

Since v0.131 the dictionary also grows from **packs**: themed sets of styles
- film photography, light and angle, lo-fi textures, the sound of an era,
camera moves - published in a registry. **Get styles** on the Styles screen
opens the catalogue: packs by half (picture, motion, sound), category and
words, each saying whether it is **Not installed**, **Installed** or has an
**Update available**, or is **Not for this space** when its profile has no
type for its styles. A pack opens into its styles; **Install the pack**
puts them all in, under a family of the pack's own, and **Install only this
style** one of them.

A pack's style is a shipped entry like the starter set's: the same pack
never installs twice, one you never touched takes a pack's newer words, one
you reworded keeps yours, and one you deleted stays out of a pack installed
whole - installing that very style brings it back. Each install is written
as styles made by hand are, so the history says it and an undo takes it
back.

**Where packs come from.** kilna ships a snapshot of a registry beside the
application, read without a network. **Settings → General → Style packs**
adds folders and sites - any static site, GitHub Pages among them - that
serve a registry, and can stop the sites being read at all. A registry is an
`index.json` listing its packs (`id`, `version`, the SHA-256 `digest` of the
pack's file, `title` and `description` in every language, `half`,
`category`, `brick_count`, `updated_at`, `manifest_url`, an optional
`preview_webp_url`), and a file per pack listing its styles in the shape of
the starter set (`key`, `type`, `label`, `family`, `description` - in
English, naming no artist - `when`, `explanation`, `colours`, `sample`), with
an optional `preview_webp` and what a style `needs`, `provides` and
`conflicts` with. A pack whose file does not match its digest is refused.

Since v0.137 the snapshot holds thirty packs: black and white, a photo studio,
Soviet print, cinematic light, the city at night, film stocks, stage looks,
rain and snow, small movements of a hero, and sixteen sound packs **in the
spirit of** an artist - the name stands in the pack's title only; every style
in it is production tags, and a test keeps any artist's name out of them. The
picture packs set their contracts: a monochrome style brings `color:mono`, a
place needs `scenery:yes`, a look needs a person shown to the bust. Whoever
keeps a registry of their own runs `node tools/style-registry.mjs` after
editing a pack: it writes each pack's digest, count and date into
`index.json` (`--check` says what is out of date).

## Sets, stars and the switch

Every style kilna ships is filed under a **set** - since v0.132 the
lettering, the dressings, the backgrounds and accents and every phrase of
sound too: *Hand-drawn*, *Street*, *Scripts of the East* for lettering,
*Pastel* and *Gradients* for colours, *Voice* and *Backing* for vocals, and so
on. The sets stand in a panel beside the dictionary, type by type, each with
how many styles it holds. Pressing one narrows the dictionary to it; **No
set** is listed only where you have styles of your own filed under none.

Each set has a **switch**. Switched off, none of its styles is offered - not
by the cover constructor, the composer of a style prompt, the picker of a
background or an accent, nor to the assistant choosing styles - while every
style in it keeps its own status, so switching the set on again brings back
exactly what was there. The switch is part of the profile, written as one
change the history says and undo takes back.

A card has two controls of its own over its top. The **star** marks a
favorite: favorites come first in every picker, right after the channel's
house styles, and the **Favorites** chip narrows the dictionary to them. The
**switch** takes one style out of the pickers - its status reads **Off** -
and back, without losing a word of it; a draft is offered once it is
described. Neither counts as a change to a shipped style: a starred or
switched-off style of the set still takes the set's newer words.

A style from a pack that ships a preview picture gets it as its first
picture when it is installed, kept in the space's own folder like any
reference.

**Phrases that say nothing of what they mean.** In the sound, **Explain N
phrases** at the top asks the profile's action about phrases for the meaning
of every phrase that has none, and an open phrase has **Write what it means**
for itself. The answer is kept like any package of phrases; an item that is a
style already in the dictionary says so and, kept, writes what it means,
when to take it and its set onto that style - only where it says nothing,
never over your words. **From your texts** can now tick every phrase at once.

## Motion

Since v0.133 the dictionary has a third branch beside the picture and the
sound: **Motion**, the movements of an animated frame. Its four types are
**Atmosphere** (rain, snow, fog, wind, dust), **Subject** (a blink, a glance,
breathing, hair and cloth), **World** (light and screens, water and glass,
machines) and **Camera** (a locked tripod, a breathing camera, a push-in, a
parallax slide, a rack focus). A movement is said in words - what moves, how
much, how often, and what stays still, because a frame that moves everywhere
is noise - and shown by its pictures once it has an animated preview, by its
words until then. The pack *Camera moves* from the snapshot installs into
Studio's Camera type. Over MCP every style says its `branch` - `picture`,
`motion` or the composition that writes a text out of it - and `styles` can
be narrowed to one.

## Heroes from the canon

A hero is written once, in the canon. Where the dictionary has the
*Character* type, it shows the canon's characters above that type's styles -
their portraits, their layers - each opening its card; nothing is copied into
the dictionary. In the cover constructor the hero picker offers the public
cards first and **Show every layer** adds the inner ones, each with its layer.
A hero picked by hand is shown on purpose: the cover takes its description,
its pictures and its bans whatever the card's layer, while each of its facts
keeps its own.

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

The `style` **version role** on a song — the production line kept beside its
lyrics — is a finished prompt for one work. A brick is a part that finished
prompts get built from: a song's style is written out of the phrases of
[the dictionary of sound](/kilna/guides/the-sound/), and read back against
them.

## Where the dictionary lives

**Styles** in the rail, under *Library*. The screen appears only where the
craft names style types; a profile with none has no dictionary, and kilna
does not offer a door to an empty room.
