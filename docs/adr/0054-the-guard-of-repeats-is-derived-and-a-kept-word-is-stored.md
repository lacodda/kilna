# 0054 — The guard of repeats is derived, and a kept word is stored

Date: 2026-10-02
Status: Accepted

## Context

The register (ADR 0044) says what a body of work has spent, word by word,
while a text is written. The owner asked on 2026-10-01 for the step before a
song goes out: a song that repeats one that already went out should light up
before it takes a day in the calendar - orange for a spent word ("кофе",
"шторы", "холодильник"), red for a rare word the audience heard lately
("пульсар" last month). Rarity without a model: the language's frequency,
the owner's own corpus, the owner's bank.

## Decision

**The guard is read off the texts and the calendar every time it is asked**
(`register::guard`), as a finding on the dashboard is (ADR 0010). A song
that has not gone out - nothing made from it has - is held against every
song that went out and every song booked: their sung texts (the latest
version of each sung role), and their days, read off their own releases and
those of everything made from them (a song is out when its audio is).

- **Orange**: a term of the register both texts say, or a meaning named for
  both - whatever its strictness and however long ago; or a rare word the
  other song said outside the window.
- **Red**: a rare word the other song said - or will say, booked - within the
  window.

A word is **rare** when the owner keeps it in the bank, or when it is a
Russian word both past a rank in the language's frequency list (or not on
it) and in no more of the owner's works than a small number - a word the
owner says in twenty songs is their own, and spent only if the register says
so. A word in another script has no frequency to read and is rare only when
the bank says so. The window, the rank and the number are the profile's
(`guard`: 90 days, 20 000, 2).

The rule was measured on a copy of the owner's workspace (2026-10-01, 248
songs, 150 terms, 35 songs out). Read as rare words, the register's
"rare" terms - which the owner uses as "use seldom", and has used in twenty
songs each - lit 69 songs red; English lines, with no Russian frequency,
lit a dozen more. As spent terms they are orange, and at rank 20 000 the
red songs are 14, for words like "брелок", "звездопад", "параллакс". Orange
stands on most drafts: the register is large, and that is what it says.

**What a person says is stored**: "I know, I am keeping it", on the pair of
a song and a word (`repeat_kept`) - a finding kept stays drawn and stops
counting; the gestures `repeat.keep`/`repeat.unkeep` are in the log and
undo. No key on the work, like a dismissed complaint: the decision outlives
a trip to the trash, and the startup sweep drops the ones whose song is
gone for good.

**The mark is one answer for every place** (`repeat_marks`): each work's
loudest finding not kept - a song its own, a publication its song's - drawn
in the catalogue, the calendar's chips and queue, the card's header and
overview, and given to agents in MCP `catalogue` and `register`. The
calendar warns when a publication would go out within the window of a song
it shares a rare word with (`SlotPreview.repeats`), and lets it.

**A meaning the stemmer cannot see is the assistant's**: the Studio action
**Meanings** reads the song beside the songs that went out (`{released}`)
and the register, and answers with a package of words (ADR 0052): meanings
and the works they are in, named for the register's entries when they are
one. Named works feed the guard like any term.

## Consequences

- Every catalogue and calendar read asks the guard once for the workspace;
  a few hundred songs cost milliseconds - the texts are stemmed once.
- A finding appears when its words meet and leaves when either text drops
  the word or the other song's day leaves the window: nothing to reconcile.

Rejected: storing findings and refreshing them on save (a second truth that
is right only while every writer remembers); judging rarity by a model
(the owner asked for none, and a frequency list answers it); comparing
every pair of songs both ways (a song that went out is not held - its words
are already the audience's).
