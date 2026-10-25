# 0068 — A pool reaches a prompt as a drawn handful, and a premiere keeps its titles for later

Date: 2026-10-25
Status: Accepted

## Context

v0.100.1 (ADR 0067) told Studio's release writer that the channel's template
is a set of moves, not an order. The owner asked for more: a pool of a
hundred and more of the channel's own moves - a memory, a reference, a
favourite book, a thing on the table - and of the ways a description puts
them together, so that even a move used on purpose a second time does not
read as a template.

A pool of a hundred lines in a prompt does not do that on its own. A model
handed the whole list reaches for the same few entries every time - the
first ones, the most obvious ones - and the sameness the pool was written to
break comes back through the pool itself. The choice has to be made outside
the model.

Two smaller things came with it. An audio track that opens its song goes out
as "Song (Song premiere 2026)" (ADR 0067), which is the right title for a
month and a stale one by spring; the owner wants two titles ready to switch
to. And the prompt of a cover said `TITLE: the words «Song»`: the image
generator drew the guillemets on the cover beside the letters.

## Decision

**A section of a card may be drawn from.** `CanonSection.draw: N` - the
number of its entries a prompt is handed, drawn at random each time one is
written (`canon::view::render`). The order of the handful is drawn too: the
first of a list is the one a model takes. The prompt says what it is given
("Moves (8 of 144, drawn at random for this text)"); the card shows every
entry and says how many a prompt reads. No memory of earlier draws: eight of
a hundred and forty seldom repeat, and the method still reads the recent
releases. Only a section of statements is drawn from.

**Studio's channel keeps three pools** after its description template:
`moves` (8 drawn), `assemblies` (3) and `pinned_forms` (4). They arrive in a
workspace that has the template (`SECTIONS_ARRIVED`). Their entries are the
owner's: they reach the canon as a proposal and are settled by the owner.
The release method builds the description from two or three of the moves
drawn, puts them together the way one of the assemblies drawn says, makes the
pinned comment in one of the forms drawn, and names which it took. A move is
a direction, never words to copy; a move a recent release used may come
again told from another side.

**A section's drafts settle in one gesture.** What an assistant proposes
lands as drafts so that nothing it wrote reaches a cover or a public text
unread; a pool proposed at once would be a hundred presses of "Make it
canon". `fact.settle` makes every draft of one section canon; its undo takes
back exactly those, and a replay settles the same.

**An audio track keeps two titles for later.** Two ordinary fields of
Studio's audio YouTube door, `alt_title_1` and `alt_title_2`, with the plain
tail " (audio)" and never a premiere's: they are for after it. Ordinary
fields, not a new kind of field: generation, proposals, MCP, the log, undo
and copying know them already. They reach a stored door after its title
(`FIELDS_ARRIVED`). The title of a YouTube door is counted to 100 characters,
the platform's limit; the 205 it shipped with follows where it was never
retyped (`LIMITS_BEFORE`).

**A built picture prompt quotes in plain marks.** Every typographic quote of
a built prompt - picture, negative, the lettering's second prompt - reaches
the generator as `"` (or `'`), whoever wrote the part it stood in. A cover
written by hand is its own words, marks included.

## Consequences

- Two prompts written for one release see different handfuls; "Show what a
  task receives" shows one draw, not the one the next run will get.
- A reply to a comment reads the public canon too, so it is handed a few
  moves it does not need; a handful is small enough not to matter.
- An agent over MCP reads the card whole, as the card is: the draw belongs
  to the prompts kilna writes, not to what the canon holds.
- The method of v0.100.1 follows the new one where it was never rewritten
  (`METHODS_BEFORE` keeps both earlier wordings).
- The titles for later are not swapped in by a button: the owner copies one
  to the platform and types it into the title when kilna should say what the
  track is listed under now. A title of a release that went out keeps the
  tail it carries.
