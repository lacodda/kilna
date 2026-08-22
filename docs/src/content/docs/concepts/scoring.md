---
title: Scoring
description: Snapshots pinned to a version, weighted axes, tiers, and why an unfilled axis is skipped rather than counted as zero.
---

Scoring is how kilna turns a subjective judgement — "is this good enough to
ship?" — into something that can be compared, tracked over time, and used to
decide what wins a calendar slot.

## Snapshots, not overwrites

A score is a **snapshot pinned to a version**, never an overwrite of the work.
Score a work, revise it, score it again, and both scores remain — the first
tied to the version it was taken against, the second to the new one. This is
the entire reason scoring exists as its own concept rather than a field on the
work: without a history of snapshots, there would be no way to see that
rewriting the second verse moved a song from 62 to 78.

A work also tracks whether it's **stale** — edited since its most recent
score — so a listing can show you when a score describes an older draft
rather than the one you're looking at now.

## Which score speaks for a work

A work accumulates snapshots, so something has to decide which of them *is* the
work's score — the number in the catalogue, the number the calendar shows, and
the number that decides which release keeps a contested date.

**Finalised is the work.** Once a version is named as the current one, that
version is the work, and its score is the work's score. Going back to judge an
old draft otherwise announces that the work got worse.

With no current version, or none that was ever judged, the **strongest** score
stands: the best it has been shown to be, rather than whatever happened last. A
half-finished experiment scored at 3 should not become the work's number just
by being the most recent thing you did.

One rule, in one place. Three parts of the app used to answer this question
differently, and a work could read 68.1 on one screen and 77.8 on the next
while a date was decided by a third number that nothing displayed.

## Axes and weights

A profile defines a set of **axes**: named dimensions a work is judged on,
each with a `weight` (relative importance) and a `scale` (the highest value
the axis accepts). Scoring a work means filling in a value per axis — `{"hook":
8, "lyrics": 6}` — and kilna combines them into a single 0–100 **total**:

```text
total = ( Σ (value / scale) × weight ) / (Σ weight of filled axes) × 100
```

Each axis is normalized against its own scale before being weighted, so an
axis scored out of 10 and one scored out of 5 combine fairly.

## Unfilled axes are skipped, not zeroed

If a score card only fills in `hook` and `lyrics` out of six axes, the total
is computed from just those two — the other four are **excluded from both the
sum and the weight total**, not treated as zero. A half-filled score card
should not read as a bad work; it should read as a partial judgement. This is
enforced in `ProfileConfig::total`, which skips any axis missing from the
submitted values entirely.

An empty score card — no axes filled at all — totals `0` rather than dividing
by zero.

## Tiers

A profile also defines **tiers**: named bands with a minimum total, evaluated
highest-threshold-first. A song profile might define `hold` at 0, `audio` at
55, `picture` at 68 and `clip` at 78 — a total of 80 lands in `clip`, because
it's the highest threshold the total reaches. The tier is computed and stored
alongside the total at the moment of scoring, using whatever tier list the
active profile holds *then* — later edits to the tier thresholds don't
retroactively reclassify old scores.

## Why the total is computed server-side, not accepted from the caller

The total and tier are always computed from the active profile's
configuration rather than trusted from whatever the frontend sends. Two
clients — or two moments of the same client, mid-edit — must not be able to
disagree about what a given set of axis values is worth.

## Reconfiguring a profile doesn't break history

Axis keys in a score are plain strings, not foreign keys into the profile's
axis list. Drop an axis from the profile, rename a label, adjust a weight —
every past snapshot keeps the values it was given and the total it computed
at the time, because that computation already happened and is stored, not
re-derived on read. See
[Profiles](/kilna/concepts/profiles/#keys-are-stable-labels-are-not) for the
same guarantee applied to kinds and statuses.
