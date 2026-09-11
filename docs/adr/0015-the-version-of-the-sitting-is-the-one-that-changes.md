# 0015 — The version of the sitting is the one that changes

Date: 2026-09-11
Status: Accepted

## Context

Since ADR 0002 a version has been immutable: revising meant copying its text
into the editor and saving the copy as the next revision. That kept every
score honest — a score is a snapshot tied to one version, and the text it
judged could never change underneath it — and it made every revision a
deliberate act.

It also made every revision a *ceremony*. The owner's pilot on 207 songs
(2026-09-10) named it directly: to fix one line you open a form, copy the
version, edit the copy, name it, tick "make current", save. The form for a
new version was on screen the whole time, whether or not a version was
wanted. And the two obvious alternatives are both wrong: a new version per
opening of the editor produces a dozen revisions a day, none of which means
anything; a new version every N minutes cuts the history at moments that
have nothing to do with the writing.

## Decision

**Clicking into the text is the revision.** The first change to an open
version mints the next revision from it, makes that the open one, and every
change after goes into that same version, written as you go. The person
never chooses to make a version; the sitting does.

**One version per sitting.** A sitting is one role of one work, and it ends
when the person leaves the text — another version or role is opened, the
panel is left — or pauses for twenty minutes. The next change after that
starts the next revision. The owner's rule, chosen over "per opening" and
"per N minutes" on 2026-09-10.

**A version's body can change in place, until something judges it.** The
backend accepts one edit to a version, its body, and refuses it with its own
error kind (`frozen`) once a score points at the version. The frontend does
not show that refusal: the change is what the person meant, so it starts the
next revision instead — which is exactly what a first change to that version
would have done. ADR 0002's promise is kept where it matters: the text a
score read never changes.

**The form is for the two moments a revision needs a decision first**: there
is no version in this role yet, or the person wants a copy to work on (a
rewrite that keeps the original open beside it). It is the only place a
version is given a name, and it is not on screen otherwise.

**Edits are logged and undoable.** Each write is a `version.edit` operation
carrying the body before, so a replay rebuilds the text and `Ctrl+Z` puts it
back; the version a sitting minted is undone into the trash like every other
undone creation. `work_version` gains a field clock for `body` (migration
0014), which 0011 left out because bodies never changed.

## Consequences

- Revising costs a click and typing. The history still reads as revisions,
  one per sitting, each with a parent.
- A version is mutable for as long as it is unscored. Anything that keeps a
  reference to a body — a comparison, an export — reads the body at the time
  it runs, as it always did; nothing caches text across a sitting.
- A score taken mid-sitting freezes the version: the next keystroke starts
  a new one. That is the right answer — the score judged what it saw — and
  it is silent, because the person did nothing wrong.
- The operations log grows by one entry per pause in typing, each carrying
  the whole body twice (before and after). A body is a few kilobytes; a
  sitting is tens of entries. The log was never the place to save space, and
  a diff would be a second way of storing text, which 0002 ruled out.
- "Versions never change" is no longer a sentence the documentation can say.
  It says instead what is true: the text a score read never changes.
