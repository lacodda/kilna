# 0065 — A work is prepared on a conception, and its boards are one mechanism

Date: 2026-10-20
Status: Accepted

## Context

The owner prepares a song outside kilna. A chat with an assistant gathers
sources from literature, film and painting; a prompt is assembled by hand -
the style, the title, the idea in their words, the sources pasted in -
and another assistant writes the lyrics and sends them into kilna. What kilna
held of all that was the finished text. The owner asked for the preparation
inside kilna, "for a novel as well as a song": a technical title, the style,
what the work is about and how they see it, the notes attached; scenarios to
choose from ("not complicated - expressive"); sources gathered in blocks that
can be added, starred, dragged; and an order any assistant can work from
over MCP. The research of 15.10 (`2026-10-15-conception`) split it into
three versions; this ADR is the second, v0.99: the brief, the idea, the
mechanism of a board with scenarios as its first consumer, and the order.

Four questions. Where the brief lives, and what keeps it from being taken for
the work. What a board is, so that the sources of v0.100 are the same thing.
How the assistant's cards reach a board. And what the order is.

## Decision

**A kind of work names its conception** (`conception` on a work kind): the
version role its brief is written in, the field the brief is refined into,
the roles whose text goes with the order, and the boards it lays out. The
brief is an ordinary version role - its revisions, its history, the trash,
the export all come with being one - and the profile's validation holds the
named role to `counts_as_version: false`: a brief is written about the work,
not as it. Not a new flag on a role: every place that asks "is this the work
itself" already reads `counts_as_version`, and a second answer to the same
question would drift from the first. Studio's experiment's `brief` becomes a
conception too: roles are merged across kinds by key, and a brief that counts
on one kind and not on another would make the catalogue's count depend on
which kind is listed first.

**A version that is no draft is never the work's current text.** Before
v0.99 nothing held a style prompt back from becoming the song: typing into
its lane made it current, and the owner had two songs whose current text was
a style prompt. The rule now sits where a version is made: `make_current` is
ignored for a role that does not count, `set_current_version` refuses one
(`version.notADraft`), and a workspace is settled once on open - a work whose
current version is no draft points at its newest draft, or at nothing. Not a
refusal on creation: operations logged before v0.99 ask for it, and a replay
must not fail on them. The upgrade carries `counts_as_version` onto a stored
role that says nothing, which is how the owner's style prompt never said it.

**A board is a row of `board_card` per card, whatever the board**
(migration 0042). A cover's ideas and an experiment's trials have tables of
their own because what is on them has a shape of its own; a card of a
conception is words in named fields, and a board of scenarios differs from a
board of sources only in its names. So the profile declares boards
(`boards`): kinds of card, each with its fields, the first being its
headline, and a rule for the writer (`one`: rivals, written by one; `all`:
everything not turned down, starred first). A card has a source (own,
refined, ai) and one verdict (star, rejected, none), as an idea does
(ADR 0050), and its fields are one object merged by key, so an undo of one
field never takes back another.

**A topic is a word on the card**, like a trial's series (ADR 0061), not a
row of its own: what a card is grouped under has no other attribute, and a
topic with no card is nothing to keep. Renaming a topic, merging two (a
rename onto another's name, in any letters) and dragging a card are all one
operation, `card.arrange`, which writes the board's whole order and logs the
order before it - each undone in one step, each replayed as written.

**Cards asked for from a board land on it; an agent's wait.** An action of
the scope `board` produces `cards:<board>` and reads `{cards}`: the board's
kinds and fields, what is asked (how many, the person's own card worked out,
where to dig, more of what is starred) and what stands on it. The answer is a
block of cards with a topic each and two to four directions to dig next; a
kind or field the board lacks is left out and said. In the window it lands
at once, as a cover's ideas do; over MCP, `propose_cards` waits for **Put on
the board**. The directions are read off the proposal the last landed answer
was kept as - they are what that answer said, not a fact of the board.

**"Refine" proposes; it does not write.** An action producing `conception`
answers with the idea and the brief set straight, and the answer becomes a
package for the work (`Proposal::Work`): the idea's field and a revision of
the brief written from the newest one (ADR 0055). The owner applies it, or
not - their words are the brief.

**The order is one text, built once** (`conception::order`): the title and
what is to be written, the idea, the brief, the texts of `with`, the fields,
each board by its rule, the notes and phrases attached, the channel's card as
the public reads it and the cards the work is about, the register. It is
`{order}` in a template (Studio's **Write from the brief**), the MCP tool
`brief`, and **Copy the order** - three ways out of one function, so the
three cannot disagree. Nothing written yet is a refusal, not an order of a
title alone.

**The Notes tab is the Conception tab.** What is attached to a work is part
of what it is prepared from; a kind with no conception has the tab with only
that.

Rejected: a new role flag for "brief" (above); topics as a table (a second
copy of a word, renamed by rewriting a row the cards point at, for no
attribute it would hold); a table per board (the mechanism the research asked
for is one); applying a refinement at once (it rewrites the owner's own
words); building the order in the window (the agent and the chat would read
another).

## Consequences

- Migration 0042 adds `board_card` with tombstones and field clocks; the
  trash holds a `card`, and a work takes its boards with it and back. The
  table count is 39.
- The profile gains `boards` and a kind's `conception`; every shipped profile
  carries a brief with a skeleton, a scenarios board and the actions
  `refine`, `scenarios` and `write`; the upgrade brings them into stored
  copies and relabels Studio's premise «Идея» where it still reads as it
  shipped. A kind that opened on `notes` opens on `conception`.
- Placeholders `{conception}`, `{starred}`, `{starred:<board>}`, `{notes}`,
  `{order}` and, in a board's action, `{cards}`; validation holds each to the
  kinds that can fill it.
- MCP gains `brief` and `propose_cards`; `workspace` names the boards and
  each kind's conception. A pending proposal says the board it is for.
- The export is format 5: a work's page carries its boards.
- The window's `lab` scope was read as a work's (its action offered on every
  experiment's card) and a lab's task key as naming a work called `lab`; both
  are read as their own since this version.
