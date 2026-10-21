# 0066 — A work of culture is one entry of the library, and a quote reaches a writer only as checked

Date: 2026-10-21
Status: Accepted

## Context

The owner researches a song before writing it, in a chat with an assistant:
"quiet disappointment in friendship" gathered seventeen works of literature,
film and painting, each with why it fits and a quote; "who were the
Pharisees?" started with a concept explained, went on to verses of the
Gospel, novels, films and paintings, and was then sorted by topic - "holy
things as goods", "faith without show". Seven of the seventeen went into the
prompt, their text copied by hand. The quotes were the assistant's memory:
some were right, some were not, and nothing said which.

v0.99 gave a work boards (ADR 0065) and said a board of sources was the same
thing as a board of scenarios with other words. This ADR is the third version
of the research of 15.10 (`2026-10-15-conception`): the sources themselves.

Four questions. Where a work of culture lives, so that the same film in two
songs is one film. What a card says about it, and what the application must
know of its fields. How a quote is told checked from remembered, and what
reaches a writer. And what a run that reads the web may do.

## Decision

**A work of culture is a note of the kind the profile marks `library`** - one
entry for the whole workspace, not a copy on each board. The note already is
most of an entry: its title, its author as the attribution ("whose words",
ADR 0064), its other titles as aliases, what it is about as its body; search,
the trash, the export, MCP's `notes` come with it. Two things are its own and
get columns (migration 0043): the medium, by the key of one the library's
kind declares (`media`), and the year, as said ("c. 1308"). Columns rather
than words in the title, so an entry is found again by its name whatever its
year says. Every shipped profile names `source` as the library; Blog's, a
plain kind since it shipped, becomes it.

**A card links to its entry** (`board_card.note_id`) when its kind is marked
`library`, and says only what the entry is to this work - why it is here,
what to take, the quote, the caveat. Its headline is the entry's, read by a
join, never copied: an entry put right is right on every board. A card
proposed by an answer names its work under `entry`; when it lands the entry
is found by any of its names - and authors that do not contradict each other
- or written into the library first, as a gesture of its own, so undoing the
card leaves the library as it grew. What the entry did not say yet is filled
in from the answer; a name it did not know is kept. An entry a board still
links to is not deleted (`library.onBoards`); the link is `ON DELETE SET
NULL` for a card restored after its entry went.

**A field says what it holds** (`shape`): words, a `quote`, the `proof` of
where the quotes were checked, or `chips`. The application has to know three
things of a card of research - which words are somebody else's, where they
were checked, and which short things can become phrases - and a field's shape
is where the profile says so, as a section's shape says what a canon card's
entries are (ADR 0043). One proof to a kind, and only beside a quote; a proof
is a link or it is refused.

**A quote reaches a writer word for word only with a link it was checked
at.** One from memory is left out of the order, the prompts and `{starred}`,
with a line that says to take its sense and never its words - one rule in
`card_line`, which every reading of a card goes through. A medium the library
marks `quoted: false` - music - is never quoted at all: an answer's quotes of
a song are dropped when it is read, and the card names the song and says why.

**A run that reads the web gets only the web** (`web` on an action). Research
checks its quotes against pages: the run is started with the CLI's two tools
for the web as its only tools, allowed without a prompt nobody would answer,
and with no MCP server (`--tools WebSearch,WebFetch --allowedTools ...
--strict-mcp-config`). A page is a stranger's words; an answer written from
one must not be able to call the person's other tools on its say - write a
file, propose into kilna, send mail. A follow-up in the chat is a web run
too, read off the chat's action as its method is.

**Research comes before the idea.** `{prepared}` reads the brief and the idea
as `{conception}` does, and says there is nothing yet instead of refusing:
"who were the Pharisees?" is asked before any song about them. The scenarios'
template reads `{starred:research}`, so the ways a song could go grow out of
what the person kept.

**A source again is a repeat.** The guard (ADR 0054) holds a song against the
songs out or booked by its sources as well as its words: an entry both
boards hold, not turned down, is an orange finding named by the entry's
title, kept the way a word is. A song still researched, with no line yet, is
held by its sources alone. On the board, a card says where else its entry
worked.

Rejected: a table of its own for the library (the note is already an entry
with search, trash and export; a second record of the same work would be the
copy ADR 0001 forbids); the work's identity as words in the card's fields
(two songs, two "Old Joy"s, and no repeat to see); the quote's verdict as a
flag the answer sets (a word with nothing to open is not a check); trusting
the method to keep a song's lyrics out (a rule the application holds cannot
be talked out of); the person's whole toolset for a run that reads pages.

## Consequences

- Migration 0043: `note.medium`, `note.year`, `board_card.note_id`; the
  note's and the card's clocks and the card's tombstone written again.
- The profile gains a note kind's `library` and `media`, a card kind's
  `library`, a field's `shape`, an action's `web`; every shipped profile a
  `research` board laid out before the scenarios, the `research` action and
  the library. The upgrade carries them into stored copies, and moves the
  scenarios' template untouched since v0.99 to the one that reads the
  research.
- `Proposal::Cards` carries a card's `entry` and, from an agent only,
  `starred`; `Proposal::Work` carries cards board by board, so a new song
  arrives with its research in one click.
- MCP: `propose_cards` takes `entry` and `starred`, `propose_work` takes
  `boards`, `brief` returns each card's entry and where else it worked,
  `workspace` names the library and the shapes.
- The window: a card of the library is headed by its entry and says where it
  worked; a quote is marked checked or from memory; chips become phrases of
  the work; a board of several kinds reads by topic or by kind. The Notes
  screen has the library, its entries' fields and where each worked.
- The export is format 6: a card is called by its entry, and an entry says
  its medium and year.
