---
title: Planning a release
description: Queue, slots, displacement — how a stronger work takes a contested date and a weaker one keeps its plan.
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
[Scoring](/kilna/concepts/scoring/). Until v0.24 the contest used the most
recent snapshot instead, so a work could read one number in the catalogue and
be judged by another here.

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
  ground the layout builds around — it fills empty days only, and never
  displaces anything.
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
behind — and **This month** comes back to today, which is marked in the
accent colour.

Each booked release shows as a chip in its day, in the colour the work carries
everywhere else in kilna. Already-released chips are dimmed: they are history
sitting on a date, not a plan competing for one.

**Picking a release in the queue turns the grid into a way to answer "when".**
Days become clickable; clicking one claims that slot, with the same contest
described below. Clicking a chip instead opens the release itself.

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

Each chip has a grip on its left. **Drag the grip**, not the chip: a chip that
is draggable everywhere puts every click in a race with a drag, which is how
the predecessor lost the click entirely.

Dropping on another day claims that date, with the same contest as any other
way of claiming one: an empty day is simply taken, a held day goes to whichever
work scores higher, and a pinned one refuses. Dropping on the **bin** — which
appears at the top only while something is in the air — returns it to the
queue, exactly as unscheduling does.

**The outcome shows before the drop.** While a release hangs over a day — or
while a picked queue entry hovers over one — the day answers: amber names the
release that would lose the date, red says the claim would be refused and why
(the holder scores at least as well, or the date is pinned). The message is
the same verdict the drop would act on, computed by the same rule, so the
preview cannot promise one thing and the contest do another. An empty day
shows nothing extra: there is nothing to warn about.

Editing the date in the release itself is the exception: that moves a booking
you already hold rather than bidding for a new one, and does not contest.

## Keeping a date

Some dates are decided rather than proposed: an announced launch, a slot
booked around something else, a release someone is waiting on. Open the
release and tick **Keep this date**.

A pinned slot is not contested. The rule below never runs against it — a
stronger work is refused rather than let through, and the refusal says the
date is pinned rather than that something outscored you. The lock on the chip
is how you see it from the month view.

The pin belongs to the date, so losing the date loses the pin: unscheduling or
clearing the date drops it, and pinning a release that holds no date is
refused.

## Claiming a slot

A calendar **slot** is a date. Scheduling a release for a slot that's empty
just claims it — nothing else happens.

Scheduling a release for a slot that's **already held** by another release
triggers a contest:

- If the new release's work scores **higher** than the slot's current
  occupant, the new one takes the slot and the occupant is **displaced**: its
  `scheduled_at` is cleared and it drops back into the queue. It is never
  deleted — losing a slot must never lose the plan for the work, only the
  date.
- If the slot is **pinned**, scheduling is refused outright and nothing is
  compared — see above.
- If the new release's work scores **equal or lower**, scheduling is
  **refused** with an error naming what's already holding the date. Ties go
  to whoever is already in the slot — a plan shouldn't move without a reason
  to move it.
- An **unscored** release cannot displace a **scored** one, on the reasoning
  that "unjudged" is not a claim of strength. A scored release, even a weak
  one, can displace an unscored one.

```text
schedule(release, "2026-09-01")
  → slot empty:              claimed, nothing displaced
  → slot held, you're stronger:  claimed, holder returned to the queue
  → slot held, you're weaker/tied: refused
```

## Rescheduling and released slots

Scheduling a release into the slot it already holds is a no-op that succeeds
without displacing itself. And once a release has been marked **released**,
its old date no longer competes for anything — a new release can claim that
same calendar date freely, because history occupying a date is not the same
as a plan contesting it.

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
