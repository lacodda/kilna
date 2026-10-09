---
title: The lab
description: Experiments with a board of trials - each tried out, heard, kept or dropped - and what a kept one becomes, a song's style or a phrase of the dictionary.
---

Some things are found by trying: a sound for a song, the opening of a
chapter, a headline. You write a dozen candidates, run them somewhere outside
kilna, listen, keep two and drop ten - and the two end up in a song. The lab
is where that happens: an **experiment** is a work with a **board of
trials**, and what a kept trial becomes remembers where it came from.

Studio ships one kind of experiment - an experiment with sound, its trials
style prompts for a music generator. Nothing in kilna knows about sound,
though: a [profile](/kilna/reference/profile-document/#lab) may make any
kind a lab and name what its trials go into.

## An experiment

Make one from the catalogue's **New work** with the kind *Experiment*, or
from a song's card: **Make an experiment from this** - a rework of the song's
style, which then stands on the board as its first trial (see
[Reworking a song](#reworking-a-song)).

An experiment is a work like any other: it has a card, versions, fields,
notes, links, a history, and it goes to the trash and comes back. Its
versions are a **Brief** - what the experiment is after; Studio's starts
from a skeleton: the direction, what to hold on to, who to listen to, the
vocabulary, the schemes, what to do when it goes wrong - and the
**Findings**, what you learnt. Its fields:

| Field | What it holds |
| --- | --- |
| Direction | What is being looked for, in a line. |
| Anchors | What every trial keeps, one phrase to a line - the tags without which the generator slides into something else. |
| Listen to | Who to listen to. Never part of a trial's text: a generator ignores band names, or refuses them. |
| BPM, Key, Tempo, Mood, Vocal, Instruments | What a song has too; BPM and the key close a trial written from the dictionary. |

Its status is a draft until you say otherwise: *Running*, *Finished*, *On the
shelf*. An experiment is never scored, booked or released.

## The board

An experiment opens on its **Trials** tab: the trials on the left, the open
one on the right.

The list is grouped by **series** - *A sweep of the field*, *Around: slower*,
*Rework: Harbour lights* - in the order each series began. Inside a series it
is a tree: a variation stands indented under the trial it varies. Each row
says at a glance what the trial is - kept (✓), dropped (✗) or not judged
yet (•), what it moves, a flag where a run starts, a warning where it lost an
anchor, how many takes it has, and how many places a kept one went. The chips
above the list narrow it to the kept ones, the ones not judged yet, or the
dropped ones; a dropped trial stays under *All*, stepped back - it is still an
example of what does not work.

The anchors of the experiment stand above the list. A trial keeps an anchor
when each of its words stands somewhere in the trial's text, in any order -
*overdriven bass fuzz* keeps `fuzz bass`, *galloping tom-heavy drums* keeps
`galloping toms`; the little words (`in the`) do not count, and what an
anchor line holds in brackets is a note. A trial that lost one says so, by
name: *Lost the anchors: `fuzz bass`*.

### A trial

The open trial's head holds **what it moves** - its angle, the line its row is
known by - the **verdict** (*Keep*, *Not judged*, *Drop*), **Run it first**,
and **Copy**, which puts its text on the clipboard to paste into the
generator.

Its **text** is read against the [dictionary of sound](/kilna/guides/the-sound/)
as it is written, the way a song's style is: a phrase the dictionary has is
marked, and the strip under the text says what each one means; a tag it does
not have is dotted, one press from **Add to the dictionary** or the
assistant's **Explain**. The text is saved a moment after you stop typing.

**Listen to** is who to listen to. A trial that reworks a song says which
version of its style it started from. **Takes** are the files the generator
gave - **Add takes** attaches them to the trial, and the player plays them
where the trial is judged. **What came out** is what you heard, and the
generator's links.

### New trials

**New trial** offers:

- **Empty** - a trial to write, in the series of the open one.
- **From the dictionary…** - the [constructor](/kilna/guides/the-sound/#writing-a-style-from-the-dictionary)
  of a style: phrases picked block by block, in the order picked, closed by
  the experiment's BPM and key. The trial remembers the phrases it was
  picked from.
- **A variation of the open one…** - its copy, standing under it in the tree;
  you say what moves. One thing at a time: two things moved at once, and
  nobody can hear which one did it.
- **From a song's style…** - a version of a song's style put on the board as
  it stands, to be reworked.

## The assistant

With an action of the profile that proposes trials - Studio's **Propose
trials** - the board asks the assistant for them, and they land on it as the
answer comes, wearing *new*:

- **Sweep the field** - three to twelve trials as far apart as the direction
  allows, in the series *A sweep of the field*. The eye beside the button
  shows exactly what will be sent.
- **Variations around this** (in the open trial's menu) - variations of it,
  each moving one thing and saying which, as its children.
- **What went wrong** (once *What came out* is written) - one variation that
  answers what was heard: the core kept, one line changed or added.

The assistant reads the brief, the findings, the anchors and the board as it
stands: the kept trials as what works, the dropped ones as what does not,
the rest as what not to repeat - and the phrases of the dictionary with what
each means, named by id. A phrase it names that the dictionary lacks is left
out of the trial's bricks and said; the text keeps it.

## Where a kept trial goes

A trial goes anywhere only once it is kept. Then, under **Where it goes**:

- **Into a work…** - choose a song: the trial becomes a version of its
  style, **beside** the song's current style, not over it - a found sound is
  tried in the song before it wins. A trial that reworks a version of that
  song is written from it, a branch beside its original in the song's
  [tree of versions](/kilna/guides/writing-a-version/). Make it current there
  when it wins.
- **Into the dictionary…** - a phrase cut from the trial - the whole text, or
  the line you leave in the box - kept as a brick of sound, with what it
  means.
- **Make a work** - a new song whose first style is the trial's.

Each remembers the trial, and the trial lists where it went - its row counts
them, and a click opens the song's version or the brick. A song's version
taken from a trial says so in the [export](/kilna/reference/data/).

## Reworking a song

**Make an experiment from this** on a song makes an experiment named after
it, linked to it, with the song's newest style as the first trial - the core
- in the series *Rework: Harbour lights*. Vary it, sweep around it, keep what
works and take it back into the song: it lands as a branch beside the style
it started from.

The link says *lab* on both cards. It is not a link of making: the song is
not released when a song found in the experiment goes out, and a song made
from a trial has nothing to do with the experiment's folder or title.

## Agents

An agent reads a board with the [MCP](/kilna/reference/mcp/) tool `trials`,
proposes trials with `propose_trials` - a later trial may vary an earlier one
of the same proposal by its number - and a whole experiment, brief, anchors
and board, with `propose_work`. Nothing lands until you put it on the board.
