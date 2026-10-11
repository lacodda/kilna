# 0062 — A closed day closes the layout, not the hand

Date: 2026-10-11
Status: Accepted

## Context

The calendar knew two things about a day: what is booked on it, and whether
a booked date was kept by hand (the pin, which since v0.44 says only "the
auto-layout does not put anything here"). It did not know that a person is
away. A fortnight off meant either laying the queue out and dragging every
release that landed in it back out, or pinning empty days that could not be
pinned - a pin belongs to a release, not to a day.

Two neighbouring asks of the same stage (v0.96 "Days and shifts") lean on the
same question:

- **A shift of the plan.** A week lost, and everything booked from a day on
  has to move by the same number of days. Which releases move, and what
  happens to one that lands on a closed day?
- **A period's dates kept at once.** A week or a month settled in one
  gesture instead of a click per chip.

And one ask from before: the calendar's batch gestures - a plan booked - were
not undoable, "a batch inverse is its own stage".

## Decision

**Time off is a span of days with a name** - its own table, `time_off`
(migration 0039): `starts_on`, `ends_on`, both inside, and a `label` that
cannot be empty, because the month shows it on every day the span closes and
a closed day that cannot say why reads as a fault. Spans may overlap; a day
two spans close is named by the one that starts first, the same answer in the
backend (`time_off::covering`) and the window (`lib/timeOff`). It belongs to
the profile, as the calendar and its rhythm do; it carries tombstones and
field clocks like every table a person's work lives in, is undone from a copy
the log keeps (no trash drawer - nothing hangs off a span), and goes out in
the markdown export as `time-off.md` and to agents with the MCP `calendar`.

**A closed day is closed to the auto-layout, not to a person.** `layout::plan`
steps over it, and `layout::apply` refuses a plan whose day was closed after
the preview, as it refuses a plan whose day was taken. A person dropping a
release on a closed day is told - `SlotPreview.closed` carries the span's
name - and the release lands. This is the footing the pin and the guard of
repeats already stand on: a rule that argued with a deliberate gesture is
what the calendar gave up in v0.44.

**A shift is a preview and a booking, as a layout is.** `layout::plan_shift`
returns the moves - release, from, to - and moves nothing; the window draws
them as ghosts; `apply_shift` books exactly those moves and refuses them whole
if any release is no longer planned on the day the preview saw, or was pinned
since. What moves is every planned release dated on or after the day; what
stays is what went out (history) and what is pinned (a date a person
settled) - the plan names how many stay. A shift moves by plain days: the gaps
between releases are the rhythm, and stepping a release over a closed day
would bunch the days after it, so a landing on a closed day is counted in the
preview instead. A shift never puts a release before today; one that would is
refused naming it.

**A period's dates are kept in one gesture** - `release.pinRange` pins, or
frees, every planned release dated in the span, touching only what changes, so
an older pin keeps its moment.

**The calendar's batches carry what they changed, and are one undo step each:**
`layout.shift` carries its moves, `release.pinRange` the pins it changed and
what each was, `layout.apply` its placements. The replay applies the carried
changes, not the gesture again: the same span read on a rebuilt workspace
could name releases the first run did not touch.

## Consequences

- A holiday is said once; the layout lands around it from then on, and
  closing days over a booked plan offers to shift what is inside out past it -
  closing moves nothing by itself.
- The month, the week and the agenda draw closed days the same way; the
  agenda folds a run of them into one line by name, apart from the gaps the
  plan should fill.
- Recurring closures - every Sunday, every 1 January - are not modelled: a
  span is a fact with two dates. A weekday the craft never publishes on is a
  question for the profile's rhythm, if it is ever asked.
- The MCP `calendar` answer is an object (`releases`, `closed`) where it was a
  list; agents read it by the tool's description.
