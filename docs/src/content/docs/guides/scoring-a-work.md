---
title: Scoring a work
description: A walkthrough of scoring a work, seeing the total and tier, and watching a revision move the score.
---

This walks through scoring a single work on the **Music** profile. The same
steps apply on any profile — only the axis names change.

## Score the current draft

Say you have a song called "Harbour lights" with a lyrics draft as its current
version. Open it, go to the **Score** tab, and fill in values for whichever
axes you can judge honestly right now:

```jsonc
{ "hook": 6, "lyrics": 7, "emotion": 7 }
```

You don't need every axis filled. Leaving `production`, `originality` and
`visual` blank doesn't count against the work — they're excluded from the
total entirely rather than treated as zero. See
[Scoring](/kilna/concepts/scoring/#unfilled-axes-are-skipped-not-zeroed).

Submit it, and kilna computes a total from the axes you gave and their
weights, then reports which tier that total falls into — `hold`, `audio`,
`picture` or `clip` on the Music profile's default thresholds.

## The score is pinned to this draft

The score you just took is tied to the version of the lyrics that existed at
the moment you scored it — not to the work in general. If you look at the
work's score history, this snapshot stays exactly as it was even after you
keep editing.

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

## Reading staleness

If you edit the work again after scoring — even without adding a formal new
version — the catalogue marks the work **stale**: its most recent score now
describes an older draft than what's currently in front of you. Staleness is
a signal to re-score, not an error.

## Where scoring feeds into

A work's most recent score decides two things downstream:

- **Tier**, shown wherever the work appears in a catalogue or list.
- **Strength**, which is what a calendar slot compares when two releases
  contest the same date. See
  [Planning a release](/kilna/guides/planning-a-release/).

An unscored work is not treated as bad — it's treated as unjudged, and sorts
last in the catalogue rather than at the bottom.
