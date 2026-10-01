# 0050 — An idea is a concept on the publication's board

Date: 2026-10-01
Status: Accepted

## Context

ADR 0049 made a cover one concept per publication, its prompt built from it
in code, and said what came next: "the idea generator of v0.89 writes
concepts, not text". The owner had started each cover from several ideas
before settling on one - their own, the same worked out, a handful from other
angles, the cover a neighbouring publication of the same song already had -
and lost all but the chosen one. The mockup of 27.09 draws a board for them
on the Cover tab and no wizard: an own idea, "work my idea out", how many
ideas from the assistant, cards with the frame drawn small, a star, "not
that", "more in this direction", "into the constructor".

Four questions. Where an idea lives. What an assistant's answer is and how it
reaches the board. What taking an idea in does to the cover. And what a
neighbour's cover is on this board.

## Decision

**An idea is a row of `cover_idea` (migration 0034).** It has its
publication, where it came from - `own`, `refined`, `ai`, `sibling` - the
neighbour it was copied from, an angle, a headline, a concept of the cover's
own shape (`Cover`, read by the same struct), and one verdict column: `star`,
`rejected` or none. A list inside `work.cover` would make every star a
rewrite of the cover, and an undo of a star a reversal of whatever else
changed in it; two flags would let an idea be starred and turned down at
once. Ideas are logged (`idea.create`, `idea.update`), undone, replayed,
go to the trash on their own and with their work, like a comment. They are
not searched: an idea is a candidate, and the cover chosen is in the search
through `work.cover`.

**The assistant answers with concepts, read against the workspace.** A
profile action with `"scope": "cover"` and `"produces": "cover-ideas"` reads
`{ideas}` - what is asked, with what stands on the board: the shortlist, the
turned-down ideas as anti-examples, the rest not to repeat, the cover and the
neighbours' covers, and for a short the song it comes from - and
`{choices}`, everything an idea is built from with the id to name it by. It
answers with a fenced block of ideas. Every brick, card and variant is looked
up by id, then by name; one the workspace does not have is left out of its
idea and said (`dropped`), and the rest of the idea lands. Refusing a whole
answer for one misspelt name would throw away a run's worth of angles; a
silent drop would hide it.

**Asked for from the board, ideas land on it at once; an agent only
proposes.** The run's answer becomes a `Proposal::CoverIdeas` and is applied
as it arrives, the way v0.86's release meta fills its empty fields: the board
is itself where a person judges candidates, and nothing about the work
changes until an idea is taken in. `propose_cover` over MCP writes the same
proposal and it waits - in the chat on the work and above the board - for
**Put on the board**. `cover` reads the board and the choices.

**Taking an idea in replaces what an idea decides and keeps what the
publication holds.** The idea and the scene, the hero, the built frame, the
four bricks, the accent, the mark's variant and the captions come from the
idea, an absent one absent - a merge "by what is present" could not say "no
hero card", and would leave a card beside a scene describing someone else.
The lettered title and "title apart", where and how the mark goes, the
switches of the channel's details, the person's own words and the prompt as
last copied stay: they were set for this publication, not for this idea. An
idea of words alone - an own idea never worked out - changes only the idea.
It is an ordinary `work.update` of the cover, with its `before`, so undo puts
the cover back.

**A neighbour's cover is offered live and copied when judged.** The other
publications of the same song - everything made from the nearest work up the
chain that never goes out itself - whose cover is built lie on the board
fitted to this publication's shape: in a tall picture a layout built around a
side becomes the centred one and a title kept to a side goes to the top;
everything else is shares of the frame and reads the same in any shape. A
star or a "not that" copies the fitted concept onto the board as a `sibling`
idea, which then stands for it; taking one in copies it into the cover
without a row.

**"Make…" starts the ideas beside the meta.** How many is the profile's
`cover_ideas`, 0 to 5, three when absent; 0 asks for none. One number for the
craft, not one per kind: a short that wants none is a short whose board is
asked by hand.

Rejected: ideas as a list inside the cover (above); ideas as notes with a
kind (an idea's concept is the cover's shape, not a note's text, and a note
has no verdict); a separate `star` and `rejected` (above); a wizard (the
owner's decision of 27.09, "no master"); applying an agent's ideas without a
click (ADR 0018: an agent proposes); storing the neighbours' covers on every
board (a second copy of each cover, stale the first time the neighbour
changed).

## Consequences

- Migration 0034 adds `cover_idea` with tombstones and field clocks; the
  trash holds an `idea`, and a work takes its board with it and back.
- The profile gains `cover_ideas` (optional, format unchanged); Studio ships
  `cover-ideas` for the clip, the audio and the short, carried into stored
  profiles as a missing action.
- `PromptTemplate` gains the `cover` scope and the `cover-ideas` product,
  validated like the release's: only a kind with a cover, only `{ideas}` and
  `{choices}` there.
- `stop_task` stops a run by its task key: the board did not necessarily
  start the run it shows - "Make…" may have.
- The markdown export does not carry the cover's concept or its board yet
  (wish recorded).
