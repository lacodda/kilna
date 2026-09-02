---
title: Planning a release
description: Queue, slots and the auto-layout — how a release gets a date, how a month is read at a glance, and how one click paces the whole queue to your rhythm.
---

A **release** is a plan to ship a specific work as some kind — a clip, a
newsletter send, a feed release — on some date, or with no date yet at all.
Creating a release doesn't schedule it; scheduling is a separate step, and
it's the one where kilna pushes back.

## The queue

A release with no `scheduled_at` sits in the **queue** — planned, but not yet
claiming a calendar date. The queue is sorted strongest first, by each
release's work's score, with unscored releases sorting last. This is where you
look when deciding what deserves the next open date.

**Which score** is the one the catalogue shows: the current version's, or the
strongest if no version is current — see
[Scoring](/kilna/concepts/scoring/). The queue, the catalogue and the calendar
all read that one number: until v0.24 this screen used the most recent snapshot
instead, and a work showed one figure here and another there.

## The rhythm and the auto-layout

The profile can state a [rhythm](/kilna/reference/profile-document/#rhythm) —
how many days the craft keeps between releases, and optionally the usual time
of day, which is shown beside the date when editing a release. With a rhythm
set, **Lay out the queue** plans a date for everything waiting, in one click;
without one, the button explains what to set first.

The plan is a **preview, not a booking**. Ghost chips — dashed, in each
work's own colour — show where every queued release would land, and a bar
above the grid says how many and between which dates. Nothing is written
until you approve it; cancelling leaves the calendar untouched. Approving
books **exactly the previewed plan**: if the calendar changed in between — a
slot claimed by hand, a release scheduled from its card — the whole plan is
refused rather than partially applied, and you preview again from what the
calendar holds now.

The layout follows four rules, in this order:

- **Nothing already on the calendar moves.** Booked days, pinned or not, are
  ground the layout builds around — it fills empty days only, and never puts
  anything beside what is already there.
- **Spacing.** A planned date keeps at least the rhythm's distance from every
  release that has a date — planned or already released. Something that went
  out yesterday sets the pace exactly as a booked slot would.
- **Scatter.** Two releases of the same work never land on neighbouring days.
  With a rhythm of two days or more, spacing already guarantees this; on a
  daily rhythm it is what keeps one work from occupying a whole week.
- **Order.** The queue is walked strongest first, each release taking the
  earliest day the rules allow.

The same inputs always produce the same plan, and placement starts tomorrow —
today is already underway. Applying writes one line to
[History](/kilna/guides/the-history/) for the whole batch, and the works'
[statuses](/kilna/guides/statuses/) catch up silently, the way the mass
resync does it.

## The month

The calendar draws one month at a time, weeks starting on Monday, with the
neighbouring days dimmed at either end so the grid stays rectangular. The
arrows walk it in both directions — work is planned ahead and reviewed
behind — and **This month** comes back to today, whose cell is outlined in the
accent colour.

Each booked release shows as a chip in its day, in the colour the work carries
everywhere else in kilna. The chip's top line holds the grip, the
[ready marks](#ready-marks), the lock if the date is pinned, and the glyph for
the kind of release; the title gets the day's full width underneath.
Already-released chips are dimmed: they are history sitting on a date, not a
plan competing for one.

Which glyph stands for which kind comes from the profile — kilna does not know
whether your craft ships clips or beta reads, so the profile says both what the
kinds are and what they look like. See
[kind glyphs](/kilna/reference/profile-document/#kind-glyphs). Hovering the
title opens a card with the work's score, its tier, what it still needs and the
link it went out on, if it has one — everything the release dialog shows, one
hover earlier.

**Filtering by kind.** Above the grid sits a row of chips: one per kind of
release the calendar holds, plus **All**, each with the count behind it.
Choosing one shows only those releases; choosing it again goes back to all of
them. The row doubles as the legend for the glyphs on the chips, and it stays
out of the way when the calendar holds only one kind of release. The queue is
not filtered with it: the queue is what still needs a date, and hiding part of
it would hide work waiting to be scheduled. A
[layout preview](#the-rhythm-and-the-auto-layout) on screen is narrowed to
match, so the plan never shows what the month is hiding — booking still books
every placement, filter or no filter.

**Picking a release in the queue turns the grid into a way to answer "when".**
Days become clickable; clicking one gives the release that date. Clicking a
chip instead opens the release itself.

## Ready marks

Every chip and every queue row carries a small readiness row, answering the
one question a calendar is read for: **can this actually go out?**

- **Two ticks** — it already went out.
- **One tick** — everything is there: the work has a version for every role
  this kind of release requires, and a score speaks for it.
- **Otherwise, one glyph per gap** — a page for a missing version role, a
  gauge for a missing score. Hover for the list in words; opening the release
  spells the same list out.

Which roles a release needs comes from the profile: each release kind lists
the version roles it *requires* — a clip needs lyrics and a style prompt, a
beta read needs the text. A role the kind does not require is neither shown
nor counted: *not applicable* and *missing* are different answers, and only
the second one blocks readiness. A kind that lists no requirements is judged
on the score alone.

**The colour belongs to the deadline, not to the gap.** The same missing
style prompt is a quiet grey note in the queue or a month out, amber inside a
week, and red two days before the slot. What changed is not the work — it's
how much time is left to do it.

At the same distance the amber starts, the journal starts talking: a release
due inside the coming week that is not ready writes a warning to
[History](/kilna/guides/the-history/) — once per release and date, checked at
startup and after every calendar change.

## Moving a release

**Pick a chip up anywhere on it** and carry it to another day. The chip itself
travels under the pointer — title, marks and all — and the day underneath
lights up as you cross it. A press only becomes a drag once the pointer has
moved a little way, so a click still opens the release; the grip on the left
stays as the sign that a chip can be moved at all.

**Carry it to either edge and the month turns.** Rest the pointer in the strip
down the left or right side of the grid and the calendar walks back or forward
a month at a time, so a date in April is reachable from January without putting
anything down. **Escape** puts the chip back where it was.

Dropping on another day writes that date. Nothing is refused and nothing is
evicted: a day holds as many releases as you put on it. Dropping on the
**bin** — which appears at the top only while something is in the air —
returns it to the queue, exactly as unscheduling does.

**A day already holding something says so** while a release hovers over it, and
the drop goes through anyway. It is a note about what is there, not a warning
about what will fail.

:::note[This changed in v0.44]
Until then a day held exactly one release, and dropping onto a taken one
started a contest: the stronger work kept the date and the weaker went back to
the queue. The only thing that ever asked for a date was a person pointing at a
day, and a rule that argues with a deliberate gesture reads as a fault rather
than as care — so the contest was retired. What remains of it is the pin, with
a narrower promise: see [Keeping a date](#keeping-a-date).
:::

A day with more than two releases shows the first two and folds the rest into
**+N more**; clicking it opens the day, and **Show fewer** closes it again.

## Keeping a date

Some dates are decided rather than proposed: an announced launch, a slot
booked around something else, a release someone is waiting on. Open the
release and tick **Keep this date**.

**A pin is a message to the auto-layout.** It never puts anything on a pinned
day, so the date stays yours while the queue is laid out around it. The lock on
the chip is how you see it from the month view.

It does not stop you. Dropping a second release onto a pinned day works like
any other drop — you can see the lock, and meaning it is the whole point of a
gesture. Until v0.44 the pin also refused other releases outright, because
scheduling was a contest; now that nothing contests a date, that half of the
promise has nothing to refuse.

The pin belongs to the date, so losing the date loses the pin: unscheduling or
clearing the date drops it, and pinning a release that holds no date is
refused.

## Claiming a slot

A calendar **slot** is a date, and a date holds as many releases as you put on
it. Scheduling a release for a day — from the queue, by dragging a chip, or by
typing the date into the release itself — writes that date. Nothing is
compared, nothing is refused, and nothing already there is moved.

Two exceptions to "nothing else happens", both about the queue rather than the
date: a release that had no date leaves the queue when it gains one, and a
release that loses its date returns to it.

:::note[This changed in v0.44]
A slot used to hold exactly one release. Claiming a taken one started a
contest — the stronger work kept the date, the weaker returned to the queue
with its plan intact, a tie went to whoever was already there, and an unscored
release could never take a date from a scored one. The rule was retired when
it became clear the only thing it ever argued with was a person choosing a
date deliberately. The score still orders the queue and still drives the
auto-layout; it just no longer decides who may have a day.
:::

## Rescheduling and released slots

Scheduling a release into the day it already holds writes the same date again
and changes nothing else. Once a release has been marked **released**, its
date is history rather than a plan — it stays on the calendar, dimmed, and
takes part in nothing.

## Marking released

When a release actually goes out, mark it **released**, optionally attaching
the link it shipped under. This is a state change kilna records, not an
action it performs — nothing is published, uploaded, or posted from here.
Marking a release without a link a second time keeps whatever link was
already recorded rather than clearing it.

## Editing a release

Clicking a chip opens the release: its kind, its date and the link. Changing
the date here **moves** the booking rather than bidding for a new one — a date
typed into a form is a correction, and the app pushing back mid-edit would be
answering a question nobody asked. Clearing the date returns it to the queue,
which is the same thing unscheduling does.

The move is written to [History](/kilna/guides/the-history/) and the work's
[status](/kilna/guides/statuses/) is worked out again, exactly as when a slot
is claimed. A release that moved without either would leave the work calling
itself scheduled after its date was cleared.

## Taking a release out of the calendar

**Unscheduling** a release clears its date without touching anything else: the
release stays exactly as planned, just without a slot, and returns to the
queue.
