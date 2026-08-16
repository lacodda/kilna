---
title: The trash
description: How deleting works in kilna — the undo in the message, the Trash screen, what comes back with what, and the only two actions that are final.
---

Nothing in kilna is lost by pressing a button. Deleting a work, a version, a
score, a release, a note or a collection moves it to the **Trash**, where it
waits until you decide otherwise.

That is why kilna never asks *are you sure*. A confirmation dialog charges a
click every time to protect against the rare mistake; an undo charges a click
only when the mistake actually happened.

## Two ways back

**The message.** Every deletion announces itself in the bottom right with an
*Undo* button. Clicking it puts the thing back immediately — and reopens what
was closed, so undoing a deleted work returns you to its card.

**The Trash screen.** Once the message has faded, the entry is still in
**Trash** in the sidebar. Each row shows what it was, what kind of thing it
was, where it came from — a version's row names the work it belonged to — and
when it went. The restore button on the row is the same action as *Undo*,
available for as long as you want it.

## What comes back with what

Deleting a work takes everything hanging off it: its versions, scores,
releases, notes and assets. Restoring it brings all of them back, under the
same identifiers they had — including the pointer to whichever version was
current — so links and references still resolve. A round trip through the
trash is not an edit: timestamps are what they were, not what they would be
if the rows had been recreated.

Deleting a **collection** does not delete the works in it. They lose their
membership and stay where they are; restoring the collection puts them back
into it. Works you moved to another collection in the meantime keep the
collection you moved them to — restoring returns the collection, it does not
overrule decisions you made afterwards.

### Restoring a child needs its parent

If you delete a version and then delete the work it belonged to, that version
has nowhere to go back to. Its row in the trash is greyed out and its restore
button is disabled, with the reason on hover. Restore the work first and the
version can follow.

## The two final actions

Everything else in kilna is reversible. These two are not, and are the only
places still guarded by a question:

- **Delete for good** removes one entry from the trash permanently. Doing this
  to a work also clears the entries that only made sense underneath it — a
  version whose work will never return could never be restored either.
- **Empty the trash** clears every entry for the active profile at once. The
  dialog says how many entries that is before you commit.

Neither is done for you: kilna has no retention policy that quietly discards
old entries. The trash grows until you empty it, and that is the intended
behaviour — a tool that deletes your deletions on a timer is a tool you cannot
trust with the ones you have not looked at yet.

## The trash belongs to a profile

Like works and notes, trash entries are scoped to the profile they were
deleted in. Switching profiles shows that profile's trash, and emptying the
trash empties only the active one.
