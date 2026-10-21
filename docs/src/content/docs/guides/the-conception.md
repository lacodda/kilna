---
title: The conception
description: Preparing a work before it is written - the brief in your words, the idea it is refined into, the research with its quotes checked on the web, boards of scenarios to star and turn down - and the one order a writer works from, in kilna, over MCP or copied out.
---

A song, a chapter, an episode is often prepared before a line of it is
written: you say what it is about and where it comes from, try a few ways it
could go, keep one or two, and only then write - or hand all of it to a
writer. The **Conception** tab is where that happens, and what comes out is
the **order**: everything you prepared, in one text, the same whoever writes
from it.

Studio prepares the song and the experiment; Novel, Podcast and Blog prepare
every kind they ship. A kind the profile prepares on no conception still has
the tab, holding only the notes and phrases attached to the work - what its
Notes tab held until v0.99. Which kinds are prepared, and on which boards, is
the profile's ([`conception`](/kilna/reference/profile-document/#conception)).

## The brief

The **brief** is the work in your words. Studio's song starts from four
sections - *What it is about*, *Where it comes from in me*, *How I see it*,
*What the text should be like* - and one line under the first is enough to
start. **Start the brief** writes the skeleton in; from then on it saves
itself as you type, one revision per sitting, the way every text of a work
does.

A brief is written *about* the work, never as it: it is not counted among
the work's versions, it is never its current text - the text the overview
shows, the guard of repeats reads, an action's `{body}` is - and its lane is
not on the Versions tab. The same holds for a song's style prompt and for
every commentary.

## The idea

The **idea** - Studio's *Premise* - is what the work is about in two to four
sentences, sharper than a brief is. Write it in place, or press **Refine**:
the assistant reads the brief and what you starred, and proposes the idea and
the brief set straight under the same headings - nothing invented, nothing
of yours lost. The proposal waits in the work's chat as a package; **Apply
the package** writes the idea and a new revision of the brief, written from
the one it refines. Refine again whenever the brief has grown.

## Boards

A **board** gathers candidates: Studio's **Scenarios** are ways the song
could go - one telling move (*a lighthouse keeper counts the ships that never
come*), the image that runs through it, how it closes. Each card says where
it came from - yours, worked out, the assistant's - and every field of it is
written in place.

**Your card** is optional: a line in your words. **Generate** puts it on the
board and asks the assistant for cards - as many of its own as you pick, and,
with **Work my card out** ticked, yours worked out into one more. **Where to
dig** steers the run: *Eastern literature*, *the sea as a border*, *not
Russian*. While the assistant writes, skeletons stand where the cards will
land, with the time it has taken and **Stop**; the cards land as the answer
comes, marked *new*. The eye beside **Generate** shows what will be sent.

After a run the board offers two to four **directions** to dig next; one
click asks for cards that way.

**★** stars a card - *write by this one* - and **✕** turns it down. **More in
this direction** asks again with the starred cards as examples and the
turned-down ones as anti-examples.

### Topics

Cards can stand under **topics**. The assistant may give a card one; you give
one from a card's **Topic** menu - under a topic the board has, under none,
or under a new one. Drag a card by its grip to another place or another
topic. A topic's heading is its name, written in place: rename it there, or
give it another topic's name to merge the two. Each drag, rename and merge is
one step for **Undo**.

## The research

The **Research** board gathers what the work could draw on, before or after
the brief is written - a question like *who were the Pharisees?* can come
first. It holds three kinds of card:

- **Work** - a novel, a film, a painting, a piece of music, a myth: what is
  there and why it fits, the quote that carries it with where in the work it
  stands, what to take, and a caveat when only part of it fits (*only the
  opening - it gets loud later*).
- **Primary source** - a text with its address, *Matthew 23:27*, quoted as it
  stands.
- **Concept** - a word explained: what it is, what it means said of a
  person, and where that figure is unfair to the real thing.

**Research** in the board's composer runs the assistant with a web search:
it checks each quote against a page that holds those very words and gives
that page as the card's proof. A quote with a proof is marked **checked** and
links to it; one the assistant could not find is **from memory**. A run that
reads the web has the two tools for it and nothing else - no files, no shell,
none of your other assistant tools - because a page is a stranger's words.

A work of music is never quoted: its card names the song and who sings it,
and says why it fits.

**Images and names** on a card are chips - an image, a name, a place, a
term. Press one to keep it as a phrase of the work, with the phrases on the
[Notes](/kilna/guides/notes/) screen; a chip already kept is marked.

The board reads **by topic** - the angles you gave it, *holy things as
goods*, *faith without show* - or **by kind**: works under their medium,
concepts apart. By kind it is a reading; drag and rename by topic.

### The library

A work of culture is kept once for the whole workspace, in the **Library** on
the Notes screen: its title, its author, its other titles - the original, a
translation - its medium and year. A card on a board links to its entry and
says only what it is to this work, so putting an entry right puts it right on
every board. A card the assistant proposes finds its entry by any of its
names, and fills in what the entry did not say yet; your own card names one
by its title.

A card that names an entry another work already drew on says **Already worked
in** that work, and the [guard of repeats](/kilna/guides/the-guard-of-repeats/)
counts it for a song. An entry a board still links to is not deleted: take
its cards off the boards first.

## The order

The order is everything prepared, in one markdown text: the title and what
is to be written; the idea; the brief; what the text keeps to - a song's
style prompt; the work's fields; each board as its rule reads it; the notes
and phrases attached, each phrase with how it is sung; the channel's voice
and bans, and the cards of the canon the work is about; the register of what
is worn out.

How a board of scenarios reads: **one starred** - write by it; **several
starred** - choose one or combine them; **none starred** - every card not
turned down, the choice left to the writer. The research adds up: every card
not turned down, the starred first. A turned-down card is never in the order.

A quote reaches the order word for word only when it was checked, with the
page it was checked against; one from memory is left out, and the writer is
told to take its sense, never its words. **Refine** and the scenarios read
what you starred in the research, so the idea and the ways it could go grow
out of it.

The order goes three ways, and is the same text each time:

- **Write from the brief** in the tab's head - the assistant writes the text
  in the work's chat. Talk it over there; nothing becomes a version until you
  insert it.
- **Copy the order** - for a writer outside kilna.
- An agent over MCP reads it with [`brief`](/kilna/reference/mcp/), and
  proposes the text with `propose_version`, or cards onto a board with
  `propose_cards`.

## Attached

Under the boards stand the notes and phrases kept for the work, with a form
for a new note - see [Notes](/kilna/guides/notes/#a-note-and-a-work). The
order carries every one of them.
