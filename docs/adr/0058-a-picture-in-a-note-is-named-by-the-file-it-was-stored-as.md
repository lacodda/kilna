# 0058 — A picture in a note is named by the file it was stored as

Date: 2026-10-08
Status: Accepted

## Context

A note's body is markdown in a text box. The plan asked for a picture pasted
from the clipboard to go into it (#83) - a screenshot of a board, a reference
copied from a page. Three questions follow: where the bytes go, how the text
names them, and what happens to them when the note goes.

The line has an answer waiting for the second question: scheda's editor,
packaged as `@lacodda/scheda-editor` for kilna to use (scheda v0.13), draws
`![alt](link)` and asks its host two things - turn a pasted picture into a
link, and turn a link into a URL the page can load. Whatever kilna writes now
has to be what that host would write, or the move onto the shared editor
becomes a rewrite of every note.

## Decision

**The bytes are an asset of the note**, copied into the workspace's `media/`
like any attached file (ADR 0027), with the kind `inline` and the note as its
owner. A backup takes it, the trash takes it with the note (the trash already
captured a card's pictures by `note_id`), and a purge removes the file when
the last row naming it is gone.

**The text names the file as it was stored**: `![board](media/5f1c….png)` -
plain markdown, relative to the workspace. Not an id the renderer has to look
up: the name is the id and its extension, so the link resolves without a
round trip and keeps resolving if the row is ever lost. Not an absolute path:
the workspace moves as a folder, and a path into the old place would break.
Only a bare name after `media/` resolves; anything that would step out of the
folder is left as written.

**The export carries the pictures** (format 3): each name a note links to is
copied into `media/` beside the pages, and a page one folder down links it as
`../media/…`. The promise that an export is readable without kilna covers what
the text shows.

**An inline picture is not a picture of a card.** A card of the canon is a
note, and its gallery (`asset::for_card`) shows its portraits, references and
outfits - what the card *is*. A picture pasted into the card's free note is
something written about it, shown where it was written, and kept out of the
gallery and of what a generator is handed. An inline picture needs a note: on
a work it is refused, because nothing would draw it.

**The functions are the shared editor's host as they stand.** `pastePicture`
and `pictureLine` are its `assets.paste`, `urlOf` its `assets.resolve`. A text
box calls them today; adopting the editor is wiring them in.

## Consequences

A picture removed from the text stays attached to the note until the note
goes: the undo of an edit can bring the line back, and a file deleted under it
would leave the line pointing at nothing.

The note's text box reads a pasted file the way it reads a pasted picture -
`clipboardData.files` - so a picture file copied in the file manager pastes as
well. A picture file dropped on the window while a note is being written goes
in the same way; anything else dropped is left alone.
