# 0052 — One word, one record

Date: 2026-10-02
Status: Accepted

## Context

v0.85 gave the register of repeats its terms (ADR 0044): a word or a meaning,
how strictly it is spent, the works it is in. The owner asked for three more
things about words on 2026-10-01:

- **A bank of words** - words kept for songs to come, sorted into blocks the
  owner names ("space", "the kitchen", "for the slow one"), a word in several
  blocks; fresh, set aside or given up on; and where it was sung.
- **How a word is sung** - "Марсель" sung "МарсЭль", "пульсар" sung
  "пульсАр" - so a lyric is written for the singer and the public text is
  not.
- The assistant reading the bank and proposing words for it.

Three tables would have been the obvious shape - a bank, a pronunciation
dictionary, the register. But they are three facts about one word: "пульсар"
collected in the bank and sung in a song that went out is spent from that
day, and the guard of repeats (ADR 0054) must see that without a second copy
to keep in step.

## Decision

**A word is one row of `term`, with facets that may each be absent.**

- `strictness` - in the register, and how strictly. Loosened from NOT NULL:
  a word of the bank is not spent. (Migration 0035 rebuilds the table; the
  migration runner gained `rebuilds`, which runs a step with foreign keys
  off and checks every key before it commits - with them on, dropping the old
  table deletes every row that points at it.) An operation logged before the
  bank existed said nothing about strictness and meant a limit; `NewTerm`
  still reads an absent strictness as a limit and an explicit `null` as "not
  spent", so the log replays as it was written.
- `bank` - fresh, parked, dropped; absent is not collected. Where it was
  sung is never stored: it is found in the texts, as the register's counts
  are (`uses`).
- `sung` - a list of `{written, sung}`, one per written form, the last word
  on a form winning. The capital vowel is the stress, the way the owner
  writes it; a changed letter is a respelling.

**Blocks are their own table** (`term_block`, with a position the owner
sets) and **a word in a block is a row of the pair** (`term_block_word`) -
the shape a term and the works named for it already have. A block goes to
the trash with the words put in it; a word goes with its places in blocks
and comes back into them.

**The register's screen reads the same list by facet**: all, each
strictness, in the bank, sung its own way. The bank's screen is a chip of the
Notes screen: blocks on the left, a block's words on the right, words typed
in a line or pasted as a list, dragged into a block.

**Words proposed are one package** (`register::proposal::WordsPackage`):
words for the bank and their block, ways of singing, a strictness for the
register, the works a meaning is in. An agent proposes it with
`propose_words`; an action that `produces: words` answers with it (the
Studio action **Meanings**); kilna proposes it from the owner's own texts
(the stresses and respellings they already wrote). It is kept whole or word
by word (`word:N`), and keeping adds to a word the record holds - never
takes a facet away, never makes a second row.

**The assistant reads words** through `{words}` (the fresh words of the bank,
by block, each with how it is sung) and `{words:<block>}`, and through the
MCP tool `words`.

## Consequences

- "One word" is decided by `words::plain`: case, ё and stress marks do not
  make a second word.
- A bank word that is also a register term is spent and banked at once; the
  guard reads both.
- The bank is not a note kind. Phrases and ideas stay notes (ADR 0045): a
  phrase is a line, a word is a key of the record.

Rejected: a bank table beside the register (two rows for "пульсар", and a
guard that has to join them); pronunciations as notes (free text where the
check needs a written form and a sung one); a block as a tag on the word
(the owner orders blocks, and a tag has no order).
