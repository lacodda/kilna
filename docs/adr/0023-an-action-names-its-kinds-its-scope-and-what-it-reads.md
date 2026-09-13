# 0023 — An action names its kinds, its scope and what it reads

Date: 2026-09-13
Status: Accepted

## Context

ADR 0021 gave an action a method and ADR 0022 let an agent outside the
window propose a storyboard. The Scenes tab still had no way to ask for
one from inside kilna, and the actions it would offer had three holes the
predecessor fell through for a month each.

An action was offered on every work. *Critique the lyrics* sat on a video,
and a click sent a prompt with `{role:lyrics}` left in it — visibly, by
the rule of ADR 0021's era, but sent. A template edited in Settings could
lose the placeholder that carried the text, and nothing said so until the
answers came back about nothing. And what a card sent was not what the
panel showed: the composer filled with the template, the task appended an
instruction and a method the composer never saw.

Producing a storyboard from inside the window needed one more thing the
proposal of ADR 0022 did not have: a way to change one scene without
touching the rest. *Replace* sends every unnamed scene to the trash; *add*
puts new rows after the last; neither is "write the prompts for scene 7".

## Decision

**An action names the kinds it is for and what it is about.** `kinds`
lists the work kinds the action is offered on; empty is every kind, which
is what every stored action was. `scope` is `work` (absent) or `scene`: a
scene action is started from a row of the board, reads it as `{scene}`,
and is keyed by the scene as well as the work, so the prompts of scene 2
can be written while scene 1's are still going.

**A template is checked at save against the vocabulary it reads.** A
placeholder nothing fills, a `{role:x}` not every kind of the action has,
`{scenes}` or `{scene}` on a kind without a storyboard, a scene action
that never reads `{scene}`, `produces` naming a role the kinds lack — each
is a named problem and the profile is refused, the way a duplicate axis
key is. Loading stays lenient: a profile written for a later kilna still
opens, and its unknown `produces` reads as prose.

**A role the work has no version in is refused at render, not blanked.**
"“Harbour lights” has no Plot yet: write it first" is the whole message.
The same for a donor: an action reading `{donor}` or `{donor:lyrics}` on a
work made from nothing says to link the source first. A role the kind
does not name stays visible in the prompt, as before — the save-time
check is what catches it.

**The preview is the composition.** `compose` builds the prompt and picks
the method; `prepare` is `compose` plus a chat; `preview_task` is
`compose` alone. There is one text, so the eye on a card's button shows
exactly what the click sends, and the test asserts the two are equal.
Reference files ride with a task by path: listed in the prompt, their
folders handed to the CLI with `--add-dir`, checked to exist before the
run starts rather than by the run.

**A storyboard proposal has three changes, and `produces: scenes` names
one.** `add`, `replace` and `revise` replace the `replace` flag of ADR 0022
on `Proposal::Scenes` and on `propose_scenes`. *Revise* rewrites only the
numbered scenes, only in the fields given — a field left out is kept, the
blocks are set together — and creates a number nobody holds; nothing goes
to the trash. `produces: scenes` is the whole board (replace),
`scenes:add` and `scenes:revise` the other two; a scene action must
produce `scenes:revise`, and its answer is held to its own number.

**An answer that asked for a proposal and could not become one says why.**
`meta.proposal_refused` carries the reason — the kind of shot the kind
does not have, the scene the revision strayed to — and the chat shows it
under the answer. A button that never appears is otherwise a silent
nothing, which is the failure this whole ADR is against.

Rejected: a `requires` list of placeholders per action (the vocabulary
already says what a template may read; a second list would drift from
it); rendering a missing role as empty text (a critique of nothing);
storing the scene on the chat (the task key already names it, and the
proposal carries the number); two booleans `replace` and `revise` on the
proposal (both true means nothing).

## Consequences

- `PromptTemplate` gains `kinds` and `scope`; `carry_forward` brings both
  from the shipped copy where the stored action has none, and Studio's
  song actions become `kinds: ["song"]`. Studio's `score` reads `{body}`
  rather than `{role:lyrics}` so it stays an action for every kind; a
  stored copy still reading exactly as it shipped in v0.61 follows.
- `Proposal::Scenes { replace }` becomes `{ change }`; the `replace`
  argument of `propose_scenes` becomes `change`. Stored proposals from
  v0.62 without `change` read as `add`, which is what `replace: false`
  meant; a stored `replace: true` is not migrated — no workspace holds one
  outside the owner's, and that one was applied.
- Studio ships three actions for a video or a short: *Plot from the
  source* (`version:plot`, reads the donor's lyrics), *Storyboard from
  the plot* (`scenes`), *Prompts for the scene* (`scenes:revise`, on a
  scene). The Scenes tab offers the work actions above the board and the
  scene actions on each row; an empty board points at them.
- `start_task` takes `scene_id` and `attachments`; `preview_task` is new.
