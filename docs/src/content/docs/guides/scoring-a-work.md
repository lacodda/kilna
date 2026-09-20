---
title: Scoring a work
description: A walkthrough of scoring a work, seeing the total and tier, and watching a revision move the score.
---

This walks through scoring a single work on the **Music** profile. The same
steps apply on any profile — only the axis names change.

## Score the current draft

Say you have a song called "Harbour lights" with a lyrics draft as its current
version. Open it and go to the **Score** tab. Each axis is a row: what it is
called, what it weighs, the question it asks, and a scale you click.

```jsonc
{ "hook": 6, "lyrics": 7, "emotion": 7 }
```

Click a mark to set it; click the same mark again to take the axis back to
unjudged. The scale is also a keyboard control: Tab moves between axes, the
arrow keys move a mark at a time, Home and End jump to either end, and
Backspace clears the axis. Judging six axes never needs the mouse.

A work that has been scored before opens with the recorded marks already on
the scales — the verdict that stands, on the scales rather than only in the
history underneath. **Save** stays off until you move a mark, so you cannot
record the same verdict twice by accident; move one and the scales are yours
again. After a save the scales settle on the score you just gave.

You don't need every axis filled. Leaving `production`, `originality` and
`visual` blank doesn't count against the work — they're excluded from the
total entirely rather than treated as zero. See
[Scoring](/kilna/concepts/scoring/#unfilled-axes-are-skipped-not-zeroed).

The total updates as you go, with the tier it currently reaches. Record it, and
kilna stores the total computed from the axes you gave and their weights, and
reports which tier that total falls into — `hold`, `audio`,
`picture` or `clip` on the Music profile's default thresholds.

## What the marks mean, and what the next tier costs

If your profile gives an axis a
[rubric](/kilna/reference/profile-document/#rubrics), the sentence for the mark
you are on appears under the scale as you move along it — so "is this a seven"
is answered by the craft rather than by mood. A mark with no sentence of its
own shows the nearest one below it.

Under the total, a ruler lays the tiers out to scale with the card standing
somewhere along it, and a line says how far the next tier is and where it is
cheapest to get there:

> **9.3 to Release it** · cheapest on Hook (weight 2): 4 more marks

Cheapest means *in marks*, not in weight. A heavy axis usually moves the total
furthest per mark, but not when it is already near its ceiling, and not when
the axes run on different scales — so kilna works out what each axis would
actually cost and names the one that costs least. If no single axis can close
the gap, it says the distance and offers no axis rather than pointing at one
that would not deliver.

The same arithmetic marks the scales themselves: on any axis that can carry the
card over on its own, the mark where it would cross is ringed. An axis with no
ring cannot get there alone.

## What each release makes of it

A premiere lives or dies on its dynamics; the same video as an ordinary
upload is carried by its fit to the track. If your profile gives a release
kind [weights of its own](/kilna/reference/profile-document/#release_kinds), the
panel reads one set of answers down every channel you ship to and shows what
each makes of it — one score, several honest verdicts. Kinds that weigh the
axes the way the profile does are shown greyed rather than left out, so the
row means "weighs them your way" and not "does not apply".

Nothing appears here until some kind actually reweighs something: otherwise
every row would repeat the total.

## Judging blind

The panel normally shows you where a work stood last time — the history, the
trend beside the total, a line per axis. That is useful when you are reading,
and a thumb on the scale when you are judging: it is hard to give a six to
something you called an eight in July.

**Score blind** puts the past out of sight. The history, the per-axis trends
and the assistant's opinion are held back until this card has a verdict of its
own, and the scales start empty rather than on the recorded marks; recording
a verdict brings everything straight back, which is the point — the
comparison is the payoff, not the temptation.

What stays visible is your own arithmetic: the total these marks add up to,
the tier it reaches, the ruler, the distance to the next tier. Those are a
mirror of the verdict you are giving, not a hint about the one you gave
before.

## Holding a tier by hand

Sometimes the number is right and the verdict is wrong: a song is the only one
of its kind, or it repeats a theme you have already published, and the axes
cannot see either. **Hold at a tier** fixes the verdict where you want it and
asks why — the reason is required, because a tier nobody can argue with later
is a tier with a sentence attached.

A held tier travels: the catalogue shows it instead of the scored one, marks
it with a pin, and stops offering a distance to the next tier, which would be
advice about a number nobody is chasing. Scores keep being recorded
underneath; the tier simply stops following them.

**Follow the facts** lets go, and the score decides again.

## Who judged

A score can carry a name. Leave **Who judged** empty and the score is yours —
that is what the field has meant since the model was laid down. Type a name
and the history labels that row, so a second opinion stands next to your own
rather than replacing it.

The name stays put after recording, because judging a batch as one person
means typing it once rather than once per work.

## The score is pinned to this draft

The score you just took is tied to the version of the lyrics that existed at
the moment you scored it — not to the work in general. If you look at the
work's score history, this snapshot stays exactly as it was even after you
keep editing.

**Judging** names which version is being scored. Left alone it means the
current one, which is what you usually want. Pick another and the score is
pinned to that draft instead — for going back and judging a version you had
skipped, without disturbing what the work currently points at.

**Why this score** is a line for yourself. A total tells you *what* you
decided; a note is the only place that keeps *why*, and three weeks later that
is the part you will want. It shows up beside the score in the history.

## Revise, then score again

Rewrite the second verse. Save it as a new version. Score the work again with
updated values:

```jsonc
{ "hook": 8, "lyrics": 8, "emotion": 8 }
```

Both scores now exist in the work's history — the first pinned to the
original draft, the second to the revision. The total moving from, say, 62 to
78 is the visible record that the rewrite helped. Neither score is
overwritten; a work's score history only grows.

From the second score onwards a small line appears beside the total, drawn
against the full scale rather than against its own range: a work that moved
61 → 63 looks like the small change it was. Green climbs, red falls. The exact
numbers, with what each score changed by, are in the list underneath.

Each axis gets its own line too, beside its scale. The line under the total
says the card moved; these say which axis moved it — a rewrite that lifted the
lyrics and left the hook alone reads that way at a glance. Every score records
what all the axes were worth at the time, so these lines come out of history
you already have.

## Reading staleness

If you edit the work again after scoring — even without adding a formal new
version — the catalogue marks the work **stale**: its most recent score now
describes an older draft than what's currently in front of you. Staleness is
a signal to re-score, not an error.

## Where scoring feeds into

A work's most recent score decides two things downstream:

- **Tier**, shown wherever the work appears in a catalogue or list.
- **Order in the queue**, strongest first — which is also the order the
  auto-layout places them in. See
  [Planning a release](/kilna/guides/planning-a-release/).

An unscored work is not treated as bad — it's treated as unjudged, and sorts
last in the catalogue rather than at the bottom.
