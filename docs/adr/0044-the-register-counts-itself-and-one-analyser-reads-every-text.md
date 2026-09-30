# 0044 — The register counts itself, and one analyser reads every text

Date: 2026-09-30
Status: Accepted

## Context

A body of work repeats itself. Two hundred songs in, the same shadows and
the same lighthouse with nobody in it come back until a listener hears one
voice saying one thing. The predecessor kept a register of what was spent:
each term with how strictly it was off limits and a list of the songs it was
in, most of them typed by hand. It was the one subsystem of five that never
died, and it was used through its API
by the writing skills rather than by eye: "is this word spent?" before the
49th use of it.

Three things about it did not survive contact with the work:

- **The counts were written down.** "In 57 songs" was a number in a row, and
  it was wrong the day a verse was rewritten or a new song said the word
  again. The appendix of songs per term was worse: it listed songs by title,
  and a quarter of them predated the database.
- **Images and words were one kind of row.** "Shadow" can be found in a
  text; "a lighthouse nobody keeps" cannot - it is told a hundred
  ways - yet both were matched against drafts by the same prefix rule, and
  the images never hit.
- **The window counted repeats on its own.** kilna already marked the words
  a text leans on while it is written, with a stemmer written in TypeScript.
  A register checked by the backend with a second stemmer would disagree
  with the marks drawn over the same words.

## Decision

**A term is wording or a meaning.** Its `kind` says which: a noun, an
adjective, a verb or a phrase is wording and is looked for in a text by its
word and its `forms`, compared stem by stem, whole words only; an image, a
scene or a pattern is a meaning and is never looked for. A meaning is tied
to the works that carry it by rows of `term_work`, drawn by a person; a row
may name a work for wording too, where a text says it in a form the term
does not list. `strictness` - ban, limit, rare - is the register's own word,
because the code reads it: it decides how a hit is marked and what the
assistant is told.

**Which works carry a term is read off the works, every time it is asked.**
The works whose current version says the wording, and the works named for
it, each once. Nothing stores the number, so a rewrite that drops the word
drops the work from the count at once, and a form added to a term counts the
works that say it the moment it is saved. A work's current version is what
the work is; older drafts do not count.

**One analyser, in Rust, reads every text.** The stemmer moved out of the
window into `words`, and so did the repeat counter. The window asks for a
text to be checked - `check_text` - and gets back the words it leans on, the
terms it takes, and where to mark both, already cut into runs that never
overlap, each carrying at most one repeated word and the strictest term
covering it. The register screen's counts, the marks under a version, the
MCP tool and the prompts all go through the same functions, so they cannot
disagree about whether "окна" is "окно".

**Meaning is the assistant's.** "The same thing in other words" is not
something a stemmer can see, and kilna does not carry a model of its own:
the assistant is the person's installed CLI (the vision's BYO rule). The
action "Neighbours in meaning" is given `{register}` - the register grouped
by strictness, meanings last, and what the text already takes word for
word - and `{neighbours}` - the few works whose words stand closest to this
one, weighed by how rare each shared word is across the works, with their
texts - and answers with where the text says a spent thing in other words,
and which works it says the same thing as. The answer is kept as a version
in its own role beside the text, like a critique.

## Consequences

The register has no import of counts to trust: the predecessor's links are
brought in only for meanings, where they are the only evidence there is; for
wording they would be a second truth beside the texts.

Checking a text is a round trip rather than a function call in the window.
While a text is typed the marks wait for the answer and are drawn only for
the text they were computed on, so a mark never lands on the wrong letters;
in practice the answer is back before the next keystroke.

A term is thrown away into the trash with the works it named, and comes back
with them; a work thrown away takes its named terms' rows with it.

Rejected: storing the count and recounting on every save (a second truth
that is right only as long as every writer remembers to recount); matching
meanings by their words (it never hits, and when it does it proves nothing);
a local embedding model for meaning (tens of megabytes of runtime and a
model download in the core, for a judgement the assistant makes better);
keeping the TypeScript stemmer beside a Rust one (two answers to one
question, drawn over the same words).
