# 0043 — A card of the canon is a note, and what it knows is facts

Date: 2026-09-29
Status: Accepted

## Context

A body of work that runs for years grows a world: the people in it, the
places they go back to, the things that happened to them, and the channel
itself — its marks, its palette, its voice, the words it never uses. Until
now kilna kept that world as notes of the kinds `character`, `location` and
`lore` (ADR 0025): a page of free text each. A page answers "who is she" to
a person reading it and to nothing else. Three questions it cannot answer
are the ones a working studio asks every week:

- **What may a task see?** A cover may show her coat; it may not show the
  street she lives on. A song may use the detail of that street and never
  its name. A public text may say neither. A page mixes all three, so every
  task that reads it reads all three.
- **Is it still true, and where did it come from?** A fact first sung in a
  verse two years ago, a fact decided last week, a fact the next song is
  meant to settle — a page holds them in the same voice, and the one that
  stopped being true is deleted or, worse, left.
- **What does a generator get?** A picture of a character needs her looks in
  English, the same words every time, and needs to know when those words
  stopped matching what she is.

The channel's own vocabulary had no home at all: templates for a caption,
the variants of a mark, a palette, bans — kept in files beside the
workspace, invisible to every action.

## Decision

**A card is a note** whose kind names `sections` in the profile. The kinds
are the craft's words, as ADR 0025 made them: Studio names a channel,
characters, locations, objects, groups, events, themes and lore; Novel and
Podcast name theirs. A note of a kind without sections is a plain note, as
before. The note keeps its body as the card's free text — what the card
was before it had facts, and whatever is still not worth one — so a
workspace whose lore was pages keeps every word of it. A second table for
cards would be the duplicate ADR 0001 forbids.

**What a card knows is facts**, rows of `canon_fact`, one statement each,
filed under a section of the card's kind. A fact carries:

- a **layer** — `public` (said anywhere), `internal` (the works take the
  detail, never the name, the date or the address), `inWorks` (only inside
  the works);
- a **status** — `canon`, `open` (a live zone, left open on purpose),
  `draft` (proposed, not yet settled), `retired` (no longer true, with the
  reason it stopped being true — kept, because "why she no longer smokes"
  is itself canon);
- a **source** — a work and the line it was read off, a decision, or a
  document;
- a **time inside the world** — the words as told ("winter 2022/23") and a
  key it sorts by, read off the words when it can be, so the facts of every
  card lay out as one timeline.

A section has a **shape**, which is what lets the channel be a card like
the others: `facts` are statements; `slots`, `details`, `palette`, `marks`
and `styles` carry typed data beside the words (a caption's slot, a
detail's template and where it acts, a colour, a mark's code, a brick of
the style dictionary); `relations` and `appearances` hold no facts and are
read off other rows. One kind of card — the one marked `root` — is the
channel: one per workspace, the root of the world.

**Which task reads what is one rule, `seen_by`**, and every reader goes
through it: the screen that dims what a lens cannot see, the MCP tool, and
every prompt. A work reads everything still true or still being built. A
cover and a public text read only what is settled and public, on a card
that is public itself, in a section the craft gave that lens. The rule is a
function and not a column because it is a function of four things, three of
which change without the fact changing. A template names its lens:
`{canon}` is the work's, `{canon:cover}` and `{canon:public}` the outward
ones; a release field may read only `{canon:public}`, and a reply to a
comment is given the public canon without asking.

**A relation is one row per pair**, `canon_link`, whichever way it was
drawn, with a kind from the profile's `relation_kinds`, a label for each
side ("neighbour" one way, "neighbour and first listener" the other) and a
layer of its own. Two rows for one relation would be two places for the
pair to disagree.

**Where a card appears is computed, never stored**: the works it is the
hero of, the scenes that point at it, the versions that name it or one of
its aliases as a whole word, the covers that picture it, and the works its
facts cite. Stored, it would be wrong the first time a verse was edited.

**A card's pictures are assets with a role** — portrait, reference,
full-length, mood — and its **description for a generator** is a column on
the note with the fingerprint of the facts it was written from. When a
settled public fact of the kind's `describe_from` sections changes, the
fingerprint no longer matches and the description says it is stale. A scene
or a template that names a card (`[[card:id]]`) is given its description,
never its name: a model does not know who the card is.

**The assistant proposes; the person applies** (ADR 0016, 0018). An action
that `produces: "canon"` ends with a package — new cards, facts, changes
to facts, relations — and kilna reviews it before anything is written:
each fact is checked against the facts already on its card and a
contradiction is shown beside it. The package is kept whole or item by
item. Selected lines of a text are handed to an action with
`scope: "selection"` and become facts with that version and those lines as
their source. An agent outside the window reads the canon through the MCP
tool `canon` and proposes through `propose_canon`, landing in the same
review.

**A hero of one song lives at that work** — the card's `work_id` — and is
raised to the channel when it turns up in a second; the card says so once
it appears in three.

## Consequences

Every write is a gesture in the log (ADR 0040): a fact, a relation and a
picture's role are created, changed, reordered and taken back one at a
time, replay rebuilds the canon card by card, and the trash carries a
card's facts, relations and pictures with it. A relation whose other end
is already in the trash waits there with it and comes back when that end
does.

The profile grew three things — sections on a note kind, `relation_kinds`,
two scopes and two kinds of proposal — and a workspace made before them
gains the craft's cards on the next launch, keeping sections it renamed.

The notes of kinds that became cards leave the Notes screen for the Canon
screen. The note under them is unchanged.

Rejected: a table of cards beside the notes (the duplicate above); the
knowledge of a card as one long text with headings (the three questions
above are exactly the ones a text cannot answer); the lens as a column on
the fact (it is derived from the layer, the status, the section and the
card, and a stored copy would drift); relations as facts with a pointer (a
relation has two sides and a kind; a statement has neither); letting the
assistant write facts directly, even drafts (a proposal is read before it
becomes a fact — always).
