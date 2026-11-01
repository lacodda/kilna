# 0072 — A conversation is kept, starred and exported like the rest of the record

Date: 2026-10-31
Status: Accepted

## Context

Until v0.106 a chat was the one part of the workspace that could not leave
it and could not be marked. The markdown export (ADR 0002) carried works,
notes, comments, the register and the canon, and no conversation; an answer
worth coming back to was found again by scrolling. Three things were asked
for at once: a star on an answer with a list of the starred ones, a chat
saved as Markdown, and a prompt that could read the context a judgement is
made in - the score, the version before, the collection, the week.

A message had been written once and never changed, so its `created_at` was
its only clock (migration 0011). Its `meta` holds the proposal the answer
carried, and marking a proposal applied replaces that object whole.

## Decision

- **The star is a column**, `chat_message.starred_at` (migration 0047), not
  a key in `meta`. The meta is the proposal's; a star kept beside it is one
  write away from being dropped by the next `set_meta`. As a column the
  starred answers of every chat are one indexed read, ordered by when they
  were starred. Only an answer takes a star (`assistant.starOnlyAnswer`),
  and starring a starred answer keeps its first moment, so a second press
  does not reorder the list.
- **The star is clocked.** It is the first thing a person changes on a
  message in place, so a trigger stamps `field_clock` for it as 0011 does
  for a chat's title: a second device has to know which of two stars came
  later. Chats stay outside the operations log, as renaming one always was.
- **One writer for a conversation going out**, `exchange::chat`, used by
  **Save as Markdown…** on one chat and by the workspace's export, which
  carries every chat in `chats/` (format 7). The window asks the backend for
  the page and the file name and writes it where the person picks; it never
  composes a transcript of its own.
- **The person's day comes from the window.** `{week}` counts from the
  Monday of the day the window passes with every prompt about a work
  (`render_prompt`, `TaskAbout.today`, a batch's queue) - the backend knows
  only UTC, and a Sunday evening west of Greenwich is already Monday there.
  A prompt that reads `{week}` without a day is refused rather than guessed,
  and only an action about a work, a selection or a scene may read it.

## Consequences

- A message is no longer written once: `starred_at` changes in place. Every
  other column of it still does not.
- An export now holds what was asked of the assistant, prompts included, and
  is larger by the size of the conversations.
- `{score}`, `{previous_version}` and `{collection}` follow the rules the
  screens already follow - the catalogue's speaking score, the Versions tab's
  predecessor, a collection's order - so a prompt and the screen it was
  started from mean the same thing.
