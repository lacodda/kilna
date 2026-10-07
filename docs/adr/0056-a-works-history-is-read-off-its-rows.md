# 0056 — A work's history is read off its rows

Date: 2026-10-07
Status: Accepted

## Context

The card's History tab listed the journal's lines about the work. The plan
asked for more (#78): versions, scores, releases and the journal on one axis.
The journal looked like the natural source - it already has a line for a
version saved, a score given, a release gone out - but it cannot carry the
axis:

- it **forgets on purpose**: a line read is swept seven days after it was
  written, because the journal is the feed of notices, and a notice read has
  done its job;
- it **was never told** about most of the owner's history: a work brought in
  from atlas arrives with its versions and scores and no line about any of
  them (on the owner's workspace, 2026-10-07: 53 `version.created` lines for
  1 666 versions);
- and journalling more, for longer, would make the feed a second copy of the
  rows it describes.

## Decision

**The axis is read off the rows** (`work::timeline`, the `work_timeline`
command), each kept as long as the thing it describes:

- the work's own `created_at` - where the axis begins;
- every version, with the revision it was written from (ADR 0055);
- every score, with the role and revision it read;
- every work made from it, directly or through what was made from it, at its
  `created_at`;
- every release - the work's own, and those of what was made from it, since a
  song goes out only as its publications (ADR 0047) - on the day it went out,
  or the day it is booked for; a release with no day has no place on an axis
  of days.

**The journal adds only what no row holds**: a rename, a status that moved, a
scene put on the board, a proposal that arrived, a version or a score
deleted. Its lines for what a row already says at the same moment -
`work.created`, `version.created`, `score.added`, `release.released` - are
left out, so nothing is said twice.

The backend sorts latest first by the strings it holds; the window lays the
moments on days where the person is (`lib/timeline`), because only it knows
which day an evening in another zone falls on, and puts what is booked after
today above today, under *Ahead*. The overview's *Recent* widget reads the
same moments, so the two never tell different stories. The number beside the
tab counts the axis without its beginning, which every work has.

The `journal_for_work` command, read only by the tab and the widget, is
gone; the journal's part of the axis is read where the axis is built.

## Consequences

- A work's history no longer shrinks a week after it is read, and a work
  brought in from elsewhere has one from the first day.
- The journal stays what it is - notices, swept when read - and nothing new
  has to be written to it for the axis to grow.
- A thing the rows do not keep and the journal swept is gone from the axis:
  a rename read a fortnight ago. That is the journal's retention, unchanged,
  and the axis does not pretend otherwise.
