# 0063 — A requirement lives on what it requires

Date: 2026-10-14
Status: Accepted

## Context

Readiness (v0.21, ADR-less, `readiness.rs`) answered one question - can this
release go out today - from two facts: a version of every role the release
kind `requires`, and a score speaking for the work. The window drew the
answer as `roles: [{role, present}]` beside a `scored` flag, and five screens
(the chip, the queue, the catalogue, the dashboard, the release's row) each
turned the pair into words their own way.

v0.97 "Before and after a release" asks for more of the same question: a
place that will not go out without a cover, a release that cannot go out
with its title or description empty. Two shapes were on the table for the
profile:

- **One list on the door**: `ready: ["cover", "field:title", "role:plot"]`.
  One place to read the checklist, but keys of three namespaces in one list -
  a role named `cover` against the cover - and a requirement that outlives
  what it names: drop the `title` field and `field:title` still blocks, the
  class `requires` already has for a role the profile lost.
- **Each requirement on what it requires**: the roles stay in the door's
  `requires`, the cover is the door's `requires_cover`, a field's is the
  field's own `required`.

And for the cover, whether to read it off `cover_format`: a door with a shape
goes out with a picture. The owner's workspace says otherwise - no cover in
kilna for any of 149 shorts: YouTube takes a frame of a short when nobody
gives it a picture. A shape says what a picture would be, not that the place
refuses to go without one.

## Decision

**A requirement lives on what it requires.** `ReleaseKind.requires` (roles,
unchanged), `ReleaseKind.requires_cover`, `ReleaseField.required`. A field
taken out of the profile takes its requirement with it, so a requirement on a
field that does not exist cannot be written.

**Both flags are three-valued** (`Option<bool>`): absent is "the profile says
nothing", `false` is "the owner took it off". The upgrade that carries a
shipped profile into a stored copy (`profile::carry_forward`) fills only what
says nothing - so a box unticked in the settings is not ticked back at the
next start, which is what a plain `bool` would do, the way `sung` does.

**Readiness is one ordered list**: `Readiness { checks: [{need, done}],
ready }`, `Need` one of `role {key}`, `score`, `cover`, `field {key}`, in that
order - the roles in the door's order, the score when the kind names axes,
the cover, the fields in the door's order. What the door does not ask for is
not listed. A role the profile lost is listed by its key: it still blocks,
and now says what blocks.

Every screen reads the one list: `missing()` for the gaps, `sayNeed()` for
their words, `answeredOn()` for the tab that closes one (versions, score,
cover - or files for a kind without a Cover tab - and the overview, where a
release's fields are written). The first gap a chip shows is the first gap
every screen shows.

**A field is written** when it holds words: blank, a list of commas, or the
field's own `suffix` alone (" (audio)" is not a title) is empty.

## Consequences

- The wire shape changed (`roles`/`scored` → `checks`); the window and its
  test fixtures moved with it. No user had the old shape stored - readiness
  is derived on every read, never written.
- The settings show the whole checklist per door in one block and write the
  three places; the score is not a box, it follows the kind's axes.
- Studio ships the checklist: clip and audio upload need a cover, a title and
  a description; streaming a cover and a title; a short its title.
- The journal's not-ready warning and the dashboard's decisions widen with
  the list without code of their own: they read `ready` and the gaps.
