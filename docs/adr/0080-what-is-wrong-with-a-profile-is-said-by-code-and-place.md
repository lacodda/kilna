# 0080 — What is wrong with a profile is said by code and place

Date: 2026-11-16
Status: Accepted

## Context

A refusal travels as a code and the values its sentence needs, and the window
says it in its own language (ADR 0041). One refusal was the exception that
mattered most to a Russian window: `profile.invalid` carried its problems as
finished English sentences - some hundred and fifty of them, built with
`format!` in `ProfileConfig::validate` - "work kind 2 (`song`): axis 3
(`hook`) has the scale 0; it must be above zero". The same was true, on a
smaller scale, of the reasons a plugin cannot run, and of the body a proposal
from an agent is shown with in a chat.

A sentence has a second cost beside its language. The profile editor could
only show the whole list in a toast after Save: nothing in a sentence says
which row of the editor it is about.

## Decision

**A problem is a reason** (`crate::error::Reason`): a key under `problem.*`
and its values. `ProfileConfig::problems` returns them; `validate` remains as
their English rendering, read from the window's own English locale, for the
log, the tests and an agent on `kilna --mcp`. There is one English wording,
and it is the locale's.

**Where a problem is, is a value of it**: the `place` parameter is itself a
reason under `place.*`, with the entry's number and key, and its parent in
`in`: `place.axis { n, key, in: place.workKind { n, key } }`. The window says
a place by nesting the sentences, and reads it as a path: the editor marks the
part of the profile a problem is in, and says the problem under the row it is
about. A problem with no place is one of the profile as a whole.

**The check is asked as the profile is edited** (`check_profile_config`), a
moment after the last keystroke, and the bar that saves the draft says how
many things stand between it and a save instead of offering a Save that will
be refused.

**The same rule for the two neighbours.** Why a plugin cannot run is a reason
under `plugins.why.*`. A proposal an agent sends over MCP keeps the English
body it was stored with - a transcript is exported as it was written - and
the window is handed the body rendered again from the typed proposal, in the
window's language, when it reads the chat.

## Consequences

- Every sentence the validator can say exists twice, in `en.json` and
  `ru.json`, and the gates hold them to each other and to the source: a key
  the source names and no locale has fails `refusal_keys.rs`, and so does a
  sentence nothing names any more.
- A plural that English hid inside a verb ("has"/"have") is two keys, so a
  language that declines differently is not forced into English's shape.
- The English of a problem is no longer greppable in the Rust source: the
  code holds the key, the locale holds the words. `validate` and
  `Reason::english` are how a test reads them.
