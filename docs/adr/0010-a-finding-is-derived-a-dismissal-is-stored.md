# 0010 — A finding is derived, a dismissal is stored

Date: 2026-08-28
Status: Accepted

## Context

v0.33 gave the dashboard a row of findings: standing complaints the workspace
makes about itself — a score describing a draft that has since been rewritten,
the weakest work holding a slot while something stronger waits, a draft nobody
has opened in a month. They are computed on the front end from the catalogue
and the calendar, and stored nowhere.

v0.34 asks the obvious next thing of them: let the person put one away. That
raises a question the previous stage could dodge — what exactly is stored?

The plan for this stage said "pin, drag-and-drop, hide with a remembered
reason, TTL refresh". Read literally, that describes a table of findings: rows
with an order, a pin flag, a hidden flag and a staleness timestamp, refreshed
from the derivation on some schedule.

## Decision

**A finding is never stored. What the person decides about one is.**

Two tables, and neither holds a finding:

- `focus_dismissal` — a complaint that has been heard. It holds the kind, the
  work, and the *complaint string* verbatim, and the back end has no opinion
  about what is inside that string.
- `focus_note` — a line the person wrote themselves, with a position and an
  optional pin.

Findings keep being recomputed on every read, and `visible()` subtracts the
dismissals. A finding therefore appears when its complaint becomes true and
leaves the moment it stops being true, with nothing to sweep and nothing to
reconcile.

Three consequences follow, and each of them is a piece of the plan that this
decision deliberately drops:

**No pin on a finding.** Pinning promises to keep something in place. A finding
cannot be kept: the day its complaint stops being true it is gone, and a pin
that silently fails to hold is worse than no pin. Notes pin, because notes stay.

**No hand-made order for findings.** The derivation already sorts them, and a
second order set by hand would be a second answer to "what matters most". Two
answers to one question drift apart — the same reason v0.32 refused the
dashboard a query of its own. Notes reorder, because nothing else claims to
know where a note belongs.

**No TTL refresh.** There is nothing to expire. The queries the derivation
reads already carry `staleTime` and are invalidated by every mutation that
could change them, so a finding is at most thirty seconds stale and refreshes
by the same path as everything else on the screen.

**Hiding remembers the complaint, not the work.** This is the part that makes
dismissal safe. Hiding by work would silence it for good; hiding by kind would
silence a whole class. The complaint string carries what was actually said —
`stale-draft:1` is not `stale-draft:4` — so the same complaint stays quiet and
a changed one is news again. v0.33 already rounded the unstable complaints to
months for exactly this reason, one stage before there was anything to hide
with.

## Consequences

The back end can never render a finding, list them, or count them. Anything
that needs the list computes it, which today means the front end. A future
consumer — a plugin, a report — would have to compute it too, from the same
two queries. That is the price of not having a second place where "is this
worth doing" is decided.

Dismissals accumulate for works that are later deleted. The table holds no
foreign key on purpose, since a dismissal is a decision rather than a fact
about a work, so `focus::sweep` clears the orphans at startup alongside the
journal and run sweeps.

Adding a finding kind needs no migration. The kind is a string on both sides,
and a complaint the back end has never seen stores and matches like any other.

A complaint whose string is unstable is now a bug with a visible symptom: it
reappears every time it changes. That is a good failure — it is loud, and it
shows up the first morning after a dismissal rather than silently never hiding.
