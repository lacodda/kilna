---
title: The canon
description: The world your works share, kept as cards of facts — each with a layer, a state, a source and a time — so every task reads exactly what it may, and the assistant proposes what a new text adds.
---

A body of work that runs for years grows a world: the people in it, the
places they go back to, what happened to them, and the channel itself — its
marks, its palette, its voice, the words it never uses. The **canon** keeps
that world as **cards**, and what a card knows as **facts**: one short
statement each, with who may hear it, whether it is settled, where it came
from and when it happened in the world.

A card is a note of a kind the profile gives sections — in the Studio
profile a channel, characters, locations, objects, groups, events, themes
and lore. Notes of those kinds live here rather than on the
[Notes](/kilna/guides/notes/) screen; the note itself is still there, as the
card's free text. A craft whose profile names no kind of card has no canon
and no Canon screen. The kinds and their sections are words of the craft,
in the [profile document](/kilna/reference/profile-document/#cards-of-the-canon).

## The Canon screen

**Canon** in the menu opens every card: the list on the left, grouped by
kind — the channel first, the heroes of single works last — the open card in
the middle, its pictures and relations on the right.

- **The search** finds a card by its name, its other names and its facts.
- **The chips** narrow the list to cards holding settled facts, drafts to
  settle, or live zones. A card with drafts shows how many beside its name.
- **New card** asks for a kind and a name; a card can live at one work — a
  hero of one song — from the start.

## A card

Along the top: the card's face (its newest portrait, or its initial), its
name, its **other names** — the forms the texts use, comma separated, which
is how kilna finds where it appears — and **who may know it exists**:
known to all, internal only, or only in the works. The chips below jump to
one section; **All** reads the whole card.

Every section is a list. **+ fact** opens the editor: the statement, the
layer, the state, the source, and when it happened. `Ctrl+Enter` saves,
`Esc` leaves.

### Layers

Every fact has one of three layers, marked **A**, **B** and **C** beside it:

- **A — public.** Said anywhere: a cover, a release's text, a reply.
- **B — internal.** The works take the detail, never the name, the date or
  the address. "Lives above a bakery" goes into a verse; which bakery never
  does.
- **C — only in the works.** Never said outside them; asked directly, the
  answer is the formula the channel uses.

A card has a layer too: a card that is internal does not exist for a cover
or a public text, whatever its facts say.

### States

- **Canon** — settled.
- **Open** — a live zone, left open on purpose: the next song is meant to
  settle it. The works read it; a cover does not.
- **Draft** — proposed and not settled yet. Everything the assistant
  proposes arrives as a draft; **Make it canon** settles it.
- **Retired** — no longer true. Retiring asks why, and the fact is kept
  crossed out with its reason, so the next picture or text does not bring
  it back. **Bring back into the canon** undoes it.

### Source and time

A fact can cite **a work** and the line it was read off (the source links to
the work), **a decision**, or **a document**. A fact can be marked as
holding for one work only — an outfit for a single release.

**When in the world** is written as told — *winter 2022/23*, *the summer
before the flood* — and kilna places it on the timeline from the words when
it can: a year, a month, a season, a day. When it cannot, or places it
wrong, type the place yourself as `YYYY`, `YYYY-MM` or `YYYY-MM-DD`.

**Timeline**, at the top of the screen, lays every dated fact of every card
out in the order it happened, year by year, each under the card it is a
fact of; the lens dims it the way it dims a card. Click a name to open the
card.

### Sections of other shapes

Most sections hold statements. Some hold more than words, which is what
lets the channel be a card like the others:

- **Slots** — a caption's slot and its words; a template's parts.
- **Details** — a signature detail: its name, an English template for a
  picture generator, where it acts (covers, frames, scenes) and whether it
  is on by default. `[[card:id]]` in a template brings in that card's
  description.
- **Palette** — a colour and what it is for.
- **Marks** — a variant of the mark: its code, what it means, its
  description for a generator.
- **House styles** — bricks of the [style dictionary](/kilna/guides/styles/).
- **Bans** — what must not appear. A cover reads a picture's bans into its
  negative; a public text reads the bans of texts. Studio keeps them apart on
  the channel and on a character - *Bans* for what is said, *Picture bans*
  for what is drawn - so moderation words and the rules of the voice stay
  out of a picture's prompt.

Each shape has its reader in the [cover constructor](/kilna/guides/the-cover/):
details are its switches, marks the family a cover picks from, the palette
its accents, house styles the bricks it offers first, slots the captions of
a dressing, picture bans its negative.

The channel card lays these out as a board of panels rather than one
column.

### Relations and where it appears

**Draw a relation** joins two cards: a kind from the profile (*family*,
*neighbour*, *member*, *place*…), a word for each side — *neighbour* one way,
*neighbour and first listener* the other — and a layer of its own. A
relation is one line between two cards; it shows on both, from each side.

**Where it appears** is counted, never typed: the works it is the hero of,
the scenes that point at it, the texts that name it or one of its other
names as a whole word, the covers that picture it, the works its facts
cite. A hero of one work seen in three says it may belong to the whole
channel; **Raise to the channel** lets it go of the work.

### Pictures and the description

Paste a picture (`Ctrl+V`) or drop one onto the side panel. Each has a
role — portrait, reference, outfit, mood, still, mark — and the newest
portrait is the card's face.

A picture generator does not know who the card is; it needs the card in
words, the same words every time. The box **For the generator** holds that
description, in English, under the section it is written from — a
character's looks. **Describe** asks the assistant to write it from the
settled public facts of those sections and the reference pictures; you can
also write it by hand. When one of those facts changes, the box says the
description is **stale**. A scene or a template naming a card is given its
description, never its name.

## Through the eyes of a task

The switch at the top — **Everything**, **Cover**, **The work**, **Public
text** — shows the card as that task sees it, dimming what it may not read:

- **The work** — a lyric, a chapter, a script — reads every fact that is
  not retired, drafts and live zones included.
- **A cover** reads only settled, public facts of a public card, in the
  sections the profile gave covers — a character's looks, outfits, symbols
  and picture bans in the Studio profile.
- **A public text** — a release's text, a reply to a comment — reads only
  settled, public facts in the sections given to it. A card that publicly
  does not exist is not seen at all.

It is one rule, and everything reads through it: this screen, the
assistant's actions and an agent over [MCP](/kilna/reference/mcp/). A draft
of a reply to a [comment](/kilna/guides/comments/) is given the canon a
public text may see, and nothing more; a release's fields read it the same
way (`{canon:public}` in the
[profile document](/kilna/reference/profile-document/#template-placeholders)). **Show
what a task receives** opens the very text the assistant is handed for the
card through the lens chosen.

## The assistant proposes

The assistant never writes the canon. It proposes, and you keep what you
want:

- **On a song** — *Gather facts into the canon* reads the text and
  proposes the cards, facts and relations it adds, each fact citing the
  song and the line it was read off.
- **On a selection** — select lines in a version's text and press **To the
  canon** in the version's tools: the facts come back with that version and
  those lines as their source.
- **On a card** — *Gather facts from the note*, in the card's menu, turns
  its free note into facts under its sections; *Describe for the
  generator*, under the description, writes the description.

A proposal lists every item under a box: new cards, facts on cards old or
new, sharper wordings of facts already there, facts to retire with the
reason, relations. A fact that **contradicts** one the card already holds
shows both, side by side, before anything is written. **Keep all**, or
untick what you do not want and keep the rest. Waiting proposals for a card
show in its side panel with **Review**.

An agent outside the window reads the canon through the `canon` tool and
proposes through `propose_canon`; its proposals land in the same review.
It can also bring a card its **pictures** — files on this machine, each with
a role, on the card or on one fact of it, such as an outfit — and its
**description for the generator**. They are items like the rest: a picture
is copied into the workspace only when you keep it, and a description kept
replaces the one in the box, its facts taken as they stand at that moment.
See [the MCP server](/kilna/reference/mcp/).

## Undo, history and the trash

Every change to the canon is in the [history](/kilna/guides/the-history/)
and undone one at a time: a fact added, reworded, moved, retired; a
relation drawn or changed; a picture's role. Deleting a card sends it to
the [trash](/kilna/guides/the-trash/) with its facts, relations and
pictures; restoring it brings them back. A relation to a card that is
itself in the trash waits there and comes back with it.
