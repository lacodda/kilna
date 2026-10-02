---
title: The bank of words
description: Words kept for songs to come, sorted into blocks you name - the same record as the register of repeats, so a word you sing is known to be spent.
---

A writer collects words before there is a song for them: *пульсар*,
*сухогруз*, *брусчатка*. The **bank of words** is where they wait, sorted the
way you think of them - *space*, *the kitchen*, *for the slow one*.

**Notes** in the menu, then the **Words** chip, opens it.

## One word, one record

A word of the bank is the same record as a term of the
[register of repeats](/kilna/guides/the-register/) and the word's way of
being [sung](/kilna/guides/singing-a-text/). *Пульсар* kept in the bank and
sung in a song that went out is spent from that day - and the
[guard of repeats](/kilna/guides/the-guard-of-repeats/) sees it, because
there is no second copy to keep in step. The register's screen shows the
same list read by facets: **In the bank**, **Sung their way**, each
strictness.

A word in the bank stands somewhere:

- **Fresh** - kept, and waiting for a song.
- **Set aside** - not now, not thrown away.
- **Dropped** - given up on, and kept so it is not collected again.

Where a word was sung is never written down: **sung in 3 works** is read off
the current texts every time you look, the way the register counts.

## Blocks

The left side lists your blocks in your order, with **All words** and
**In no block** on top. Name a new block in the field at the foot and press
Enter; double-click a block to rename it; drag its grip, or use **Move up**
and **Move down** in its menu, to change the order; **Delete the block**
sends it to the [trash](/kilna/guides/the-trash/) - its words stay in the
bank.

A word may stand in several blocks. Drag a word onto a block on the left to
put it there, or use **Put in a block** on its row; **Remove from this
block** takes it out of the open one only.

## Adding words

Type a word into the field above the list and press Enter, or a list by
commas - a list pasted line by line turns into commas so you can read it
first. The words go into the open block, or into none. A word the record
already keeps - a term of the register, say - is not written twice: it joins
the bank as fresh.

## Found in your texts

**Find in my texts** reads the texts you have written for a singer and
proposes what they already say about singing: the stresses you marked with a
capital vowel where the dictionary reads the word otherwise or does not know
it, and the respellings you use - *МарсЭль* for *Марсель*. Each one says which
songs it was found in. Untick what is not right and **Keep** the rest: every
text after them is checked against your way of singing, and every public
text gets the written form back. A homograph's mark is never proposed - which
reading a song means is that song's choice, not the word's.

## For the assistant

An action reads the bank through `{words}` - the fresh words, by block, each
with how it is sung - or `{words:space}`, one block by name. An agent reads
it with the MCP tool `words` and proposes words with `propose_words`; the
proposal waits in the chat, word by word, and is kept in one click. See
[the profile document](/kilna/reference/profile-document/#template-placeholders)
and [MCP](/kilna/reference/mcp/). Why one record:
[ADR 0052](https://github.com/lacodda/kilna/blob/main/docs/adr/0052-one-word-one-record.md).
