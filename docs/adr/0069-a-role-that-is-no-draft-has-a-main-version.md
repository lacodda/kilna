# 0069 — A role that is no draft has a main version, and a release asked again writes another take

Date: 2026-10-25
Status: Accepted

## Context

A work has one current version, and since v0.99 it is always a draft: the
text the work is (`version.notADraft`). That closed a real defect - a style
prompt typed into could become the song, its text on the overview, the
corpus the guard of repeats reads - but it closed the only way a person had
to say which style prompt the song is made with. The owner keeps six of them
for one song and sings one; with "make current" gone from the style lane,
every reader of the style took the newest, and a seventh prompt written to
try something replaced the one they sing.

Separately, running Studio's release writer again on a release that already
had words came back with the same description, three words changed. The
fields' contents reached the model as "what is written there now", and the
method said a written field is "rewritten in its spirit". A person who runs
the action on a written release wants other words.

## Decision

**A version of a role that is no draft can be marked main.** One per role of
a work at most, held by a partial unique index (`work_version_one_main`, the
way `scene_frame_one_selected` holds one chosen frame per scene), so two
mains is not a state the schema can hold. Marking one takes the mark off the
other in the same unit; the gesture `version.setMain` carries what was main
before, so an undo puts it back - or puts back that nothing was - and a
replay marks it again. A draft is refused (`version.draftIsMadeCurrent`):
its choice is the current version, and two marks naming the work's text
would be two truths.

A mark on the row rather than a pointer on the work: the roles are the
profile's words, not columns, and the mark travels with the row to the trash
and back. A main version restored while another was made main meanwhile
comes back unmarked - a restore returns the version and does not overrule a
choice taken since.

**One reader for "a role's text": `version::for_role`.** The current version
when it is of that role, the main one, and otherwise the newest. `{role:*}`,
`{donor:*}`, a publication's `{source}`, the board's source text, a trial
started from a song, the conception's brief and an agent asking for a role
over MCP all read through it. Nothing marked reads exactly as before.

No choice is recovered from the operations log for the five songs that had a
style prompt made current before v0.99: on the one where it mattered, a
newer prompt was written after that choice, and guessing which one the owner
sings would be worse than letting them say it.

**A release asked again writes another take.** `{release}` says that what the
fields hold is an earlier take shown so that nothing of it comes back, and
the method has a section on it: every field afresh from the work as it stands
now, nothing taken from the earlier take's opening, details, moves, pinned
form or titles for later, the tags gathered again, and only the title left as
its template makes it. The method of v0.100.2 is a prior edition
(`METHODS_BEFORE`), so an untouched copy follows.

**A proposal is compared as the field would keep it.** A waiting proposal's
fields get the tails the place adds - "(audio)", a premiere and its year -
before they are compared and shown, so a title that differs from the stored
one only by its tail is not offered as a change.

## Consequences

- The versions tab offers *Make this the main version* on a style prompt's
  row and above it, and labels the main one; a draft keeps *Make current*.
- A role with nothing marked behaves as it always did, so a workspace sees no
  change until a choice is made.
- What the earlier take said no longer steers the next one; a person who
  wants a field polished rather than replaced edits it by hand or turns the
  proposal down field by field.
