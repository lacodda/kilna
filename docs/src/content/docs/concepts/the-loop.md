---
title: The loop
description: Work, versions, score, calendar slot, shipped — why kilna models the cycle once instead of once per craft.
---

```text
work  →  versions  →  score  →  calendar slot  →  shipped
```

Every craft that kilna targets — music, prose, podcasting, blogging — repeats
the same cycle. You make something, you revise it, you decide whether it's
good enough, you decide when it goes out, and eventually it does. kilna models
that cycle once and lets the craft configure the vocabulary. See
[Profiles](/kilna/concepts/profiles/) for how the vocabulary part works.

## Work

A **work** is the thing you're making — a song, a chapter, an episode, a post.
It has a title, a kind (drawn from the active profile), a status, and whatever
craft-specific fields the profile defines (BPM for a song, point-of-view for a
chapter).

Its status follows the rest of this loop rather than being kept up by hand:
scoring makes a work scored, a calendar slot makes it scheduled, going out
makes it released. Setting one yourself pins it — see
[Statuses](/kilna/guides/statuses/).

## Versions

A work accumulates **versions**: whole bodies of text, not diffs. Each version
has a **role** — a song keeps `lyrics` and `style` as independent drafts; a
chapter keeps `text`, `outline` and `notes`. One of a work's versions is
marked current, but every prior one stays readable.

Bodies are stored whole because text is measured in kilobytes and
reconstructing a document from a diff chain is a well-known source of
corruption for no meaningful saving. See
[ADR 0002](https://github.com/lacodda/kilna/blob/main/docs/adr/0002-local-first-storage.md).

## Score

A **score** rates a work along the axes its profile defines — hook, lyrics,
emotion for a song; pull, prose, character for a chapter — and combines them
into a total and a tier. Crucially, a score is a **snapshot pinned to a
version**, not the current state of the work. Rewrite the second verse, score
again, and you can see the total move from 62 to 78 — the whole point of
scoring is making the effect of a revision visible. Details in
[Scoring](/kilna/concepts/scoring/).

## Calendar slot

A scored work becomes a **release**: a plan to ship it somewhere, as some
kind, on some date. Slots compete — a date can hold one release at a time, and
a stronger work can bump a weaker one out of it. The weaker release isn't
deleted; it drops back into the queue with its plan intact, waiting for
another date. Walkthrough in
[Planning a release](/kilna/guides/planning-a-release/).

## Shipped

Shipping is a state you set, not an integration kilna performs. You mark a
release **released**, optionally with the link it went out on, and that's it.
Plugins can automate the mechanical parts of getting something out the door
later, but the loop closes without them — kilna never needs to know how the
work actually reached an audience.

## Why this shape

The predecessor project split "work" into a separate concept from "release"
management in ways that scattered the same fact across multiple places. kilna
keeps one row per unit of release rather than three tables for the same idea.
The trade-off, and the alternatives considered, are in
[ADR 0001](https://github.com/lacodda/kilna/blob/main/docs/adr/0001-one-schema-with-profile-configuration.md).
