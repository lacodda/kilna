# 0018 — A proposal is applied by the application, and marked

Date: 2026-09-11
Status: Accepted

## Context

ADR 0016 let an agent outside the window propose a version, a score or a
note, one message each, into a chat on the work. The first week of use —
an agent working a new song end to end — showed three things the shape
could not do:

- **The work itself had to be made by hand.** Every proposal needs a work
  to land on; there was no way to propose one. The person created the
  song, then asked for the rest.
- **The overview had no channel.** A premise, a mood, a tagline are fields
  of the profile on the overview tab; the agent could only offer them as a
  version or a note, both the wrong place.
- **Five proposals were five clicks, and nobody remembered which were in.**
  Each proposal was applied by the component that showed it, through the
  command a hand uses, and "added" was a variable in that component. The
  transcript is refetched every ten seconds and the tab is left and
  returned to; the mark was gone on the next fetch, and the message in the
  database said nothing. The person walked the tabs to check.

## Decision

**Applying is one command in the backend, over any kind of proposal, and
it marks the message.** `apply_proposal(message_id)` reads the proposal out
of the message, writes it, and stamps `meta.applied` on the message with
what it made — the work, the versions, the score, the notes, the fields.
The chat reads the mark from the transcript, so it survives every fetch and
every window. The buttons that applied a score or a note from the component
now go through this; the *insert as version* dialog goes through it too when
the answer is a proposal, carrying the person's choices (role, name, make
current) as overrides. A second application is refused.

**The command writes what a hand writes.** Each row goes in as the same
operation the hand-driven command records — `work.create`,
`version.create`, `score.create`, `note.create`, `work.update` — with the
same params and the same journal line, so a replay cannot tell a proposal
applied from a thing typed, and undo takes each back on its own. The
command is listed in the coverage gate as recording one level down, and a
test holds the exact sequence of operations a package writes.

**A package is a proposal.** `propose_work` proposes a whole work — title,
kind, overview fields, versions by role, a score, notes — or, given `work`,
a package of those for an existing one. It is one message: the proposal in
`meta`, and the body a rendering of everything the package would write, so
it is readable before it is written (plain roles in a fence, so a lyric's
lines stay lines). One click applies all of it. A new work lands in the
client's chat on nothing, since there is no work to land on yet; the mark
then links to the work that was made.

**"Apply all" is a loop.** `apply_pending_proposals(chat_id)` applies every
unapplied proposal in a chat, oldest first, and stops at the first failure
naming it, with the ones before it applied and marked. Shown when a chat
holds more than one.

Rules decided on the way, each the least surprising reading:

- A score proposed by an agent outside the window is judged by that agent:
  `rater` is the client's name, so the history does not read it as the
  author's own.
- On a new work the package's version in the vocabulary's first role — the
  lyrics, not the style prompt — becomes current, and the score judges it.
  A work has one current version; the others are read as the newest of
  their role, as the card reads them.
- On an existing work a package's versions wait beside the current one, not
  current — the dialog's own default for an answer worth keeping.
- Overview fields the profile does not have are named and left out; a blank
  value does not blank a field. An unknown role or kind refuses the package
  whole, before anything is written.

## Consequences

- `Proposal` gains a `work` variant and is deserialised as well as
  serialised: the backend reads its own proposals back. The `kind` of the
  new work is `work_kind`, because `kind` is the variant tag.
- `meta.applied` on a message is the one mark; there is no table of
  applications. A message proposed before this version has no mark and
  reads as pending, which it is.
- A package that fails halfway leaves what it wrote and no mark; the error
  says what landed. The checks that can fail run first, so this is rare.
- `workspace` now lists the overview fields, and the server's instructions
  send an agent to `propose_work` whenever it has more than one thing to
  say.
