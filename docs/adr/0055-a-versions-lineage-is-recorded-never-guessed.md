# 0055 — A version's lineage is recorded, never guessed

Date: 2026-10-07
Status: Accepted

## Context

Since v0.50 a version can name the version it was written from
(`work_version.parent_version_id`, ADR 0013), and two ways of writing one
already filled it in: typing into the open text, which mints the next
revision from it (ADR 0015), and *new version from this one*, which copies a
version into the form. Nothing drew it, and the other ways a version is born
- an assistant's proposal, a package from an agent, an answer kept as a
version - left it empty. "The previous version" meant two things: the
Versions tab compared with the revision numbered just below, the overview
with the parent.

v0.91 draws the tree and compares by it. Two shortcuts offered themselves.

1. **Fill in the past.** Most versions have no parent (on the owner's
   workspace, 2026-10-07: 59 of 1 666 have one, all lyrics), and a migration
   could set each to the revision below it, so every role would draw as a
   line at once.
2. **Draw the line where no parent is known**, as if each version without
   one followed the revision below.

Both say something nobody recorded. The owner's 709 style prompts came over
from atlas as *alternatives* - "Alternative 2: post-punk" beside
"Alternative 1" - not as a sequence; a lyric written from nothing beside the
others has no parent either. A guessed line would be indistinguishable from
a recorded one, and the guess is wrong exactly where the tree has something
to say.

## Decision

**A parent is recorded where a version is born, and only there.**

- Every way of making a version from another names it: the text typed into
  (already), a copy in the form (already), a proposal kept from a chat that
  was started on a version - the version it was started on, when the text
  lands in that version's role - an agent's `propose_version` or package
  version with `from` (refused at once when `from` is not a version of that
  work in that role), an answer kept with *insert as version* from such a
  chat. A text the person moves into another role has no parent there:
  lineage does not cross roles (the backend has refused that since v0.50).
- A rewrite in the role of the version a chat is about is that version's
  child, and no longer also "about" it: `meta.about` stays what pairs a
  commentary with the revision it discusses.
- **Nothing is filled in after the fact.** A version without a parent is a
  root. A role in which no version names a parent draws no tree at all - the
  list is the list it was - rather than a column of lone dots that say
  "unknown" once a row.

**One rule says what a version is compared with by default**
(`lib/history`'s `predecessor`): the version it was written from, while that
is still a version of the role; otherwise the revision before it - which is
what a history with no recorded lineage has always meant, and is honest
about being a fallback (the comparison's menu says *written from* or
*previous*). The same rule draws the quiet marks in the margin of every
version, counts the `+12 −4` of every row, picks what *Compare with* puts
beside the text, and what the overview compares. Commentary is not compared
with the commentary before it.

**The tree is drawn beside the list, not instead of it.** The list stays
newest first, one row per revision, so the arrows, the numbering and the
comparison that follows a step keep working; a gutter draws a dot per row and
a line from each version down to its parent, a second column where two
versions share one (`lib/versionTree`). `←` and `→` walk up and down it.

## Consequences

- The owner's existing history draws as it was until new versions are
  written: lyrics that already know their parent draw a tree; style prompts,
  and lyrics brought over from atlas, do not.
- A version's parent can still be lost - deleting it sets the child's parent
  to nothing (`ON DELETE SET NULL`) - and the child becomes a root rather than
  attaching to whatever is numbered below.
- An agent that wants its rewrite drawn in the tree has to say which version
  it rewrote; one that does not is a version from nothing, which is true.
