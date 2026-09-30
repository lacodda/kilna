---
title: The register of repeats
description: What a body of work has already spent — words counted by the works that say them, images tied to the works that carry them — marked in every text as you write, and handed to the assistant.
---

Two hundred songs in, the same shadows and the same dust come back until a
listener hears one voice saying one thing. The **register** is where you
write down what is spent, so every text you write is checked against it
while you write it.

**Register** in the menu (or `G` `R`) opens it.

## A term

Each entry is a **term**:

- **The word** — as you read it: *окно*, *on the edge*, *a lighthouse nobody
  keeps*.
- **Forms** — other words that count as the same term: *окон* beside *окно*,
  *горячий* beside *тёплый*. Cases need no form: words are compared by
  their stems, so *окно* already finds *окна*, *окне* and *окном*, and
  *ladder* finds *ladders*.
- **Kind** — a noun, an adjective, a verb or a phrase is **wording**; an image,
  a scene or a pattern is a **meaning**. The difference matters, below.
- **Strictness** — **banned** (spent: one more use says nothing new),
  **limited** (each use has to earn its place), or **rare** (a word used so
  seldom that even two or three uses stand out). It decides how the term is
  marked in a text.
- **Topic** — your own grouping: *the kitchen*, *physics and space*.
- **Note** — why it is spent, or what to reach for instead.

## How many works

Beside every term the register says how many works carry it **now**. The
number is not stored anywhere: it is read off the works each time you look.

- **Wording** is found in each work's **current version** — the text the
  work is today, not its old drafts. Rewrite a verse without the word and the
  work leaves the count; add a form to the term and the works that say it
  join. Whole words only: *чай* is not in *чайка*. What stands in square
  brackets — `[Chorus]` — is a section's name and never counts.
- **A meaning** cannot be found by its words — *a lighthouse nobody keeps* is
  told a hundred ways — so kilna never looks for it. You **name** the
  works that carry it: open the term and **Name a work**. A work can be named
  for wording too, when a text says it in a form the term does not list.

Open a term to see the works: how many times each one says it, and which
ones you named. **×** lets a named work go.

## In every text

Under a version's toolbar a strip lists the terms of the register this text
takes — *тень ×3* — banned first, and each term is marked where it stands:
a banned term with a wavy line, a limited one underlined, a rare one dotted.
The number after a term is how many works carry it across the workspace.
Click a term to open it in the register.

The strip is there whether you are reading or writing. While you write, the
[repeated words](/kilna/guides/writing-a-version/#repeated-words) are tinted
as well, and both marks can sit on one word.

## Adding a term

- **From a text**: select a word or a line in a version and press **To the
  register** in its toolbar. The word is filled in, the kind is guessed — one
  word a noun, several a phrase — and the form says how many works already
  say it before you save.
- **By hand**: **+ Term** on the register.

A word is kept once, whatever its case: a second *Окно* is refused and the
first one named.

## Meaning, and the assistant

The register catches words, not meanings — which is exactly why it keeps
images and scenes. The Studio profile's **Neighbours in meaning** action
reads a lyric beside the register and beside the few works whose words stand
closest to it (shared words weighed by how rare each one is), and answers
with the lines that say a spent image in other words and the works this one
says the same thing as. Its answer is kept as a **Neighbours** version beside
the text, like a critique.

Any action can read the register: `{register}` in a template is the whole
register grouped by strictness, meanings last, with what the text already
takes; `{neighbours}` is the nearest works with their texts. See the
[profile document](/kilna/reference/profile-document/#template-placeholders).

## Deleting

**Delete** sends a term to the [trash](/kilna/guides/the-trash/) with the
works you named for it, and **Undo** brings it back. A work you throw away
takes its named terms with it, and brings them back when it is restored.
