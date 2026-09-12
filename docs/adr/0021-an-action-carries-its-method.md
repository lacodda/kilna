# 0021 — An action carries its method, and a task is about a version

Date: 2026-09-12
Status: Accepted

## Context

A profile action was a short message with placeholders — *"Here are the
lyrics of {title}… be hard on it"* — sent to Claude Code in an empty
directory. The way the owner actually critiques and scores lived
elsewhere: in skill files on their own machine, pages long, with a role,
a checklist in blocks, an output format, stop-words and a rule for the
verdict. The model started from a blank slate on every run, and another
author installing kilna got the short message and nothing of the craft.

Two smaller gaps sat beside it. The scoring instruction handed the model
only an axis key and a ceiling — `"hook": <0-10>` — while the profile
already held the label, the question the axis asks and the marks of its
rubric; the shipped `score` template even named other axes by hand. And an
action always read the work's *current* version, so a critique started
while reading revision 2 judged revision 4, and its score landed on
whichever version was current when the button was pressed.

## Decision

**An action carries its method.** `prompts[].method` is a markdown
document — the role the assistant takes, what it checks and in what order,
the shape of the answer, what it must never say — shipped with the profile
in English and edited in Settings beside the message. It reaches the model
as a system instruction (`--append-system-prompt-file`, through a
temporary file: a method has newlines, and a `.cmd` argument may not), on
every turn of the chat the action opened, because a follow-up in a
critique chat is still a critique. The message stays short; the method
carries the how. Skill files on a machine are not shipped: they are the
owner's, in their language, about their project, and the model in an
empty directory is not obliged to use them anyway.

**The scoring instruction states the axes as the profile does** — label,
description, rubric marks, weight, and the tiers with what the total
means. A `score` template still reading exactly as it shipped before this
is moved to the new wording at the next start; one the owner reworded is
theirs.

**`produces` gains `version:<role>`.** The whole answer is offered as a
version in that role, the way an agent's `propose_version` is; Studio's
`critique` produces `version:critique`, so a critique is kept beside the
text rather than lost in a chat.

**A task is about a version.** Started from the versions tab, an action
carries the open version: the template reads it as `{body}` and as
`{role:<its role>}`, and the chat remembers it (`chat.version_id`) with the
action (`chat.action`). Applying a score binds to that version; applying a
version in a commenting role writes `meta.about` with it, and the versions
tab pairs commentary by that first, by revision number only for commentary
that has none. A chat opened from the overview is about the work as it
stands, as before.

Rejected: shipping the owner's skill files (see above); a method per
profile rather than per action (a critique and a score are done
differently); binding by revision number alone (a critique added later has
a number of its own); a `version_id` on the proposal itself (the chat is
what is about a version; every proposal in it inherits that).

## Consequences

- Migration 0017: `chat.action`, `chat.version_id`.
- `start_task` takes an optional version; the versions tab shows the
  profile's actions under the open text, named for its revision.
- Settings edits the whole action: label, hint, message, method, what it
  produces; actions can be added and removed.
- Studio ships methods for `critique` and `score`; the other profiles ship
  the scoring method. A workspace whose actions predate methods gains
  them where its copy names none.
- The panel's composer path is unchanged: picking an action there still
  fills the message for reading; the method rides with tasks started from
  a card or a batch.
