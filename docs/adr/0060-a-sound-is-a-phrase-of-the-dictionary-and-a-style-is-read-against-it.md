# 0060 — A sound is a phrase of the dictionary, and a style is read against it

Date: 2026-10-09
Status: Accepted

## Context

A song's style prompt is a line of phrases in a music generator's own
English - `female vocal, chopped amen break, everything clipping, 92 bpm`. The
owner wrote seven hundred of them across two hundred and forty songs, out of a
constructor kept outside kilna: a table of tags in nine blocks with a column
saying what each gives the track, a rule per block ("one groove", "up to four
instruments, the first is the lead"), and the habit of putting the most
important tag first, because the generator weighs the line from the left.

Two things were missing, and both are about meaning. Reading an old style, or
a style from an experiment, the owner wanted to know what each phrase does -
`noise guitar bursts`, `everything clipping` - in their own language. And
writing one, they wanted to pick knowingly, not from memory.

kilna already had a dictionary (ADR 0031): bricks of a type the profile names,
with a starter set (ADR 0048) and one analyser that reads every text (ADR
0044). It knew pictures only.

## Decision

**A sound is a brick of the same dictionary, of the form `phrase`.** Its
`description` is the phrase, word for word, in the generator's English; its
name is the phrase too. Beside it, every brick may carry an `explanation`:
what it is and what it gives, per language, as a shipped name is - for a
person, never for a prompt. Not a glossary table and not a translated
description: the phrase the editor underlines, the one the constructor
offers and the one the assistant is told about are one row, and the
generator reads one phrase, whatever the window's language.

**The profile says which role is written out of which types: `compose`.** A
composition names a role (`style`), the kinds that have it (`song`), its
blocks in the order a text is built - each a type of phrase with `min`,
`max` and a rule in a sentence - the work's fields that close the line, each
with a template (`{value} bpm`), and the generator's limit. kilna knows
compositions, never Suno: the ten types of sound, their phrases and their
rules are words of the Studio profile.

**One function writes the text.** The picks in the order they were picked -
not the blocks' order: the owner's own canon starts with the voice, not the
genre - a phrase picked twice said once, then the fields. The window and an
agent (`propose_version` with `bricks`) call the same function, so the same
picks are the same text. A block outside its bounds is said, never refused:
a style without a bass is a choice.

**The same composition says how the role is read.** `check_text`, given a
version's work and role, reads a composed role against the phrases of the
composition's types instead of looking for repeats and spent terms - a line
of phrases is not a lyric. A tag of up to six words that is a phrase is
marked with what it means; one that is not is unknown, offered to the
dictionary by hand or to the assistant's "Explain"; a longer piece is prose,
searched for phrases and otherwise left alone. The work's own fields are
neither: a number field's tag is recognised by its shape, any other by the
work's value.

**The dictionary grows from the owner's texts.** "From your texts" reads
every version of the role, counts the unknown tags by works and versions,
and sends the ticked ones to an action about phrases (`"scope": "phrases"`,
`"produces": "bricks"`), whose answer is a package kept brick by brick -
each an ordinary `style.create`, taken back by undo.

**The owner's canon is facts of the channel's card**, a `styles`-shaped
section, *House sound*, as house styles of a picture are: marked on the
card, offered first.

## Consequences

- The dictionary has halves: the types no composition reads are the
  picture's, each composition's types are its own half. A type of phrase in
  no composition is refused on save - a shelf nothing is built from.
- A phrase may not hold a comma: the line is read comma by comma. The set is
  held to that by a test, with every phrase explained in both languages.
- The fingerprint of a set brick leaves an absent explanation out, so every
  brick seeded before explanations existed still reads untouched.
- "Write as prose" is its own action (`compose-sound`): `compose-style` was
  already the picture prompt's, and one action carries one method.

Rejected: a glossary beside the dictionary (two truths about one phrase);
translating the description (the generator reads one phrase); the blocks'
order as the line's order (it throws away the most important tag); a
composer in the window (an agent would write a second text from the same
picks); the whole feature as a Suno plugin (a plugin owns no screen, no
dictionary and no undo - the exchange with Suno itself is one, later).
