---
title: Singing a text
description: Stresses marked for the singer, a dictionary that says which words need them, your own respellings - and a public text with the marks taken off.
---

A singer reads letters, not intentions. A homograph - *замок*, a castle or a
lock - is sung whichever way it guesses; a name it has never seen is guessed
at; an *е* that is *ё* is sung as *е*. So a lyric is written for the singer:
the stress as a capital vowel - *сЕти*, *пульсАр* - and a word no stress can
save respelled - *МарсЭль*.

kilna checks a text that is sung against a dictionary of three million
Russian word forms and against the ways of singing you keep, and takes the
marks off again for the text the public reads.

## Which texts

A version role the profile marks as **sung** - Studio's lyrics. A style
prompt, a plot or a review is not checked. See `sung` in
[the profile document](/kilna/reference/profile-document/#version_roles).

## The stresses strip

Under a sung version's toolbar, **Stresses** lists what the singer could get
wrong, in the order it stands in the text:

- **needs ё** - written with *е* where the word has *ё*: *еще*, *ждет*,
  *звезд*. One click writes the ё.
- **against the dictionary** - a stress you marked on a vowel the dictionary
  never stresses. One click puts it where the dictionary does; leave it if
  you mean it.
- **you sing it differently** - a word you keep a way of singing for,
  written the plain way: *Марсель* where you sing *МарсЭль*.
- **not in the dictionary** - a word the singer will guess at. Mark its
  stress yourself.
- **Homographs** - words of two readings with no mark, folded into one
  chip: *Homographs · 11*. Open it to choose each word's reading.

**Apply all** writes every answer that leaves nothing to choose - the ё, the
dictionary's stress, your respellings - and leaves the homographs to you.
**Show it in the text** selects the word in the editor.

Words a dictionary cannot know but a singer can - a sung-out vowel
(*любооовь*), a run of syllables (*ла-ла-ла*), a compound of known parts
(*кем-то*) - are not called strangers.

## Marking a stress

While writing a sung text:

- **Alt + click** on a vowel puts the stress there - that vowel a capital,
  any other mark in the word taken off. Again on the same vowel takes it off.
  On *е* it goes round: *е* → *Е* → *ё* → *е*.
- **Alt + '** puts it on the vowel at the caret - the same key on a Cyrillic
  layout, where it types *э*.

A stress on a word's first letter is written as an accent over it (*о́блако*),
because a capital there would read as the start of a line. A word with two
capitals past its first letter is an abbreviation or a shout, and is never
read as a mark.

**Show the stresses** in the toolbar draws an accent over every word's
stressed vowel - the one you marked, or the dictionary's when it knows only
one - without moving a letter. Homographs are marked in the text only while
it is on.

## Ways of singing

A word's record keeps how its forms are sung where that is not how they are
written: open it in the [register](/kilna/guides/the-register/) and add
*Марсель* → *МарсЭль* under **How it is sung**. From then on a text that
writes *Марсель* is told you sing it otherwise, and a text that writes
*МарсЭль* is left alone. **Find in my texts** in the
[bank of words](/kilna/guides/the-bank-of-words/#found-in-your-texts)
proposes the ones you already use.

## The public text

What the audience reads has no marks for a singer:

- **Copy without the singer's marks** in the toolbar puts the lyric on the
  clipboard with every capital-vowel stress lowered, every accent dropped and
  every respelling you keep put back - *В МарсЭль, где пульсАр* becomes *В
  Марсель, где пульсар*. Latin words keep their capitals: *LinkedIn* is not a
  stress.
- A release field written from a template - the audio's description, which
  reads the song's lyric - and a field the assistant or an agent proposes are
  cleaned the same way before you see them. What you type yourself stays as
  you typed it. See [Planning a release](/kilna/guides/planning-a-release/#without-the-singers-marks).

## The dictionary

The stresses come from the dictionaries of **ruaccent** (MIT), the
frequencies the [guard of repeats](/kilna/guides/the-guard-of-repeats/) reads
from Koziev's word-form frequencies (CC0). Both ship inside kilna - no
download, no network - and answer a word in microseconds. Only Cyrillic words
are looked up. Why it is built this way:
[ADR 0053](https://github.com/lacodda/kilna/blob/main/docs/adr/0053-a-sung-text-is-checked-against-a-dictionary-and-cleaned-for-the-public.md).
