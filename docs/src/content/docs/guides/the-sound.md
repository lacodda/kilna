---
title: The dictionary of sound
description: The phrases a song's style prompt is written from - each with what it means - picked block by block, read back in every style, and grown out of your own texts.
---

A style prompt for a music generator is a line of phrases in the
generator's own English: `female vocal, chopped amen break, everything
clipping, 92 bpm`. Each phrase does something to the track, and the line is
only as good as your sense of what. kilna keeps those phrases as the second
half of the [style dictionary](/kilna/guides/styles/): **Styles → Sound**.

## A phrase and what it means

A brick of sound is one **phrase**, written exactly as the generator reads
it - it goes into the text word for word - and **what it means**: what it is,
and what it gives the track, in the language of the window. *noise guitar
bursts* - short bursts of guitar noise between the phrases: grit and nerve
that break up a smooth mix. The phrase never changes language; the meaning
is the part that is translated.

Every brick has a **type**, and Studio's types of sound are the blocks a
style is built from, in the order it is built:

| Type | What it holds |
| --- | --- |
| Genre core | The genre the sound stands on, filed under rock, dream and gaze, electronic, folk and chamber, pop, scenes and regions, hybrids |
| Vocal delivery | How the voice comes: close, whispered, doubled, belted, bare |
| Groove | What the drums play - breaks with their tempo, a live kit, machines, next to nothing |
| Instrument | One instrument and how it is played |
| Bass | The low end |
| Mood | The feeling, in a word or two |
| Texture | The surface of the recording: grain, noise, space |
| Tempo | How fast it feels |
| Dynamics | How the arrangement moves from start to end |
| Knob | One line added to any core to push one thing - one at a time |

Studio ships two hundred and twelve phrases with their meanings in English
and Russian. They are yours from the start, as every brick of a
[starter set](/kilna/guides/styles/#the-starter-set) is. A phrase you edit
keeps your words; **What it means** is edited in the window's language, and
the other languages a phrase carries stay as they are.

## House sound

The phrases that are *yours* - the sound the channel is known by - are
facts of the **House sound** section of the Channel card in the
[canon](/kilna/guides/the-canon/), each naming a brick. A house phrase wears
**House** on its card and is offered first wherever phrases are picked.

## Writing a style from the dictionary

On a song's **Versions** tab, in the *Style prompt* lane, **From the
dictionary** opens the blocks in the order a style is built. Each block says
how many it takes and its rule - *one groove: the strongest lever against
songs that sound the same*, *up to four instruments, and the first is the
lead*. Click phrases to pick them; a hover says what each one means. Search
finds a phrase by its words, its name or its meaning, in either language.

**The order you pick in is the order of the words.** A generator weighs a
line from the left, so the line below the blocks is your picks as you picked
them - move one earlier or later with its arrows. The song's own **BPM** and
**Key** close it as the profile writes them (`92 bpm`, `E minor`). The line
is written by kilna, not the assistant: the same picks are the same text
every time. It shows how many characters it is, against the generator's
thousand.

**Use as a new version** puts the line in the version form, under the label
*From the dictionary*; from there it is a draft like any other. **Write the
style as prose** hands the picks to the assistant instead, which writes them
up as a few sentences - keeping each phrase as written - and proposes the
result as a version.

A block picked outside its bounds - no groove, two vocal modes - is said in
the block's count, never refused: a style without a bass is a choice.

## Reading a style by the dictionary

Every style prompt is read against the dictionary, while it is read and
while it is written. A phrase the dictionary knows is washed in the accent,
and hovering it says its type, what it means and when to take it; the strip
under the toolbar lists them all. A tag it does not know is underlined with
a dotted line and listed as **Not in the dictionary**, each with two ways in:

- **Add to the dictionary…** - keep it by hand: its type, what it means,
  when to take it.
- **Explain** - the assistant proposes a brick for it: its type and what it
  means in English and Russian, kept with one click. **Explain all** sends
  every unknown tag of the text at once.

The book in the toolbar opens the **dictionary beside the text**: the
blocks and their phrases, searchable in any language. A press puts the
phrase in at the caret, with the commas it needs - write a style knowing
what each phrase will do.

The song's BPM and key in a line are the song speaking, not a phrase: they
are neither marked nor called unknown. Nor is prose - a sentence between two
commas is searched for phrases, and the rest of it is left alone.

## Your own dictionary, out of your own texts

**From your texts**, on the Sound half of the dictionary, reads every style
prompt you ever wrote - every version, not only the current one - and lists
the tags the dictionary does not know, the most widely written first, with
how many works and versions say each. The first twenty are ticked; **Explain**
sends the ticked ones to the assistant in one task, and the answer comes
back as bricks to keep one by one. A single one can be kept by hand from the
list.

## Over MCP

An agent reads the dictionary with `styles` - by type, by composition, or by
a word in any language - and proposes a style made of it with
`propose_version` and `bricks` instead of a body: kilna writes the text as
its own constructor does. See [MCP](/kilna/reference/mcp/).
