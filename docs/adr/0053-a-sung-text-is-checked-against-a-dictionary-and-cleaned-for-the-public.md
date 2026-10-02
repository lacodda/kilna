# 0053 — A sung text is checked against a dictionary and cleaned for the public

Date: 2026-10-02
Status: Accepted

## Context

The owner writes lyrics for a singer that reads letters, not intentions: a
homograph ("замок" - a castle or a lock) is sung whichever way it guesses, a
name it has never seen is guessed at, an "е" that is "ё" is sung as "е". So
the owner writes the stress into the text, with a capital vowel - "сЕти",
"пульсАр" - and respells what no stress can save: "МарсЭль". Two things
followed that kilna did not do:

- Nothing told the owner which words of a lyric needed a mark: a homograph
  without one, a word the singer does not know, a mark on the wrong vowel, an
  "е" that should be "ё", a respelling the owner keeps but forgot here.
- Nothing took the marks off again: a description that quotes the lyric
  went out to the audience in capitals.

The plan said "open data, CC0, compressed" and "no AI for the dictionary".

## Decision

**A language pack, compiled into the binary** (`lang/ru/`, read by
`words::pack`): two finite-state maps (the `fst` crate), read where they
lie - no loading, no copy in memory, a lookup in microseconds:

- **stress** - 3.2 million Russian word forms, keyed as `words::plain` writes
  them (lowercase, ё read as е, marks dropped), each with the vowels it may
  be stressed on (two or more is a homograph), the vowels written ё, and
  whether the е spelling is a word too ("все"/"всё"). Built from the
  dictionaries of **ruaccent** (MIT). 5 MB.
- **frequency** - the rank of the language's first 100 000 stems, keyed by
  kilna's own stemmer so a rank answers for the same key every count in
  kilna groups by. Built from Koziev's word-form frequencies (**CC0**).
  1.1 MB. Read by the guard of repeats (ADR 0054).

Not CC0 throughout: the only CC0 stress dictionary (Koziev's) keeps one
stress per form and no ё, which is the half of the check that matters. MIT
is as free for this code, and the notice travels with the pack
(`lang/ru/README.md`). `cargo run --example lang_pack` rebuilds both from
the published files.

A word is Russian by its letters: a Cyrillic word is looked up, any other is
not. No setting.

**A version role says it is sung** (`VersionRole.sung`; Studio's lyrics
are). Only a sung text is checked for its stresses (`check_text(text,
sung)`), and the notes come back on the same runs as the repeats and the
register's terms, never overlapping (ADR 0044):

- **homograph** - a word of two stresses with no mark; its readings, marked,
  to choose from;
- **unknown** - a word the dictionary does not know, with no mark;
- **against** - a mark on a vowel the dictionary never stresses;
- **yo** - "е" where the word is written with "ё" (not where the е spelling
  is a word too);
- **unsung** - a word the owner sings another way, written the plain way;
  the owner's spelling to apply.

A mark is what the owner writes: a capital vowel past the first letter when
it is the word's only capital there (two or more are an abbreviation or a
shout), an acute over a first letter that is a capital already, or ё. The
window's gesture writes the same - Alt+click on a vowel - and "show the
stresses" draws an accent over each word's stressed vowel: the marked one,
or the dictionary's when it has only one.

**A public text is cleaned** (`words::sung::clean`): every respelling the
record keeps put back the way the word is written, every mark taken off - in
Cyrillic words only, so "LinkedIn" keeps its capital. A release field
written from a template (`{donor:lyrics}` in an audio's description) and a
field an answer or an agent proposes are cleaned before anyone sees them;
"Copy clean" puts a lyric on the clipboard the same way. A field typed by
hand is left as typed.

**Kilna proposes what the owner already wrote**: the capital vowels of the
owner's sung texts that the dictionary cannot give on its own (a word it
does not know, a stress it does not give) and every hard "э" that stands for
an "е" of a known word, as a package of words (ADR 0052) - never a
homograph's mark, which is the song's choice and not the word's.

## Consequences

- The binary grows by 6 MB; the installer compresses it.
- The frequency list must be rebuilt when the stemmer changes; `words::pack`'s
  tests hold the pack to a few known answers.
- A word the owner respells and the dictionary also lists ("марсэль") is not
  "unknown": the respelling is caught by its hard э.

Rejected: a model for stresses (tens of megabytes of runtime, and wrong on
exactly the homographs that need a person); the assistant for every word
(slow, paid, and a dictionary answers most of it); loading the dictionary
into a hash map (hundreds of megabytes for what an FST answers from five).
