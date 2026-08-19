---
title: Statuses
description: A work's status follows what has happened to it — and stays where you put it the moment you set one by hand.
---

A work has a status: draft, scored, scheduled, released. You should almost
never have to set it.

kilna works it out from what has actually happened. Score a work and it
becomes scored. Give a release a slot in the calendar and it becomes
scheduled. Mark that release as out and it becomes released. Delete the score
again and it goes back to being a draft.

The alternative — a field you update by hand every time something moves — is
where the predecessor of this app went wrong. Four different screens wrote to
it, none of them consistently, and after a few months the word on a song had
nothing to do with the song.

## When you set it yourself

Pick a status from the list on a work's **Overview** tab and it stays there.
The automation stops touching that work entirely — not for that status, for
any of them.

That is the point. A work you shelved is shelved even though it has a score
and two planned releases; nothing in the data says "abandoned", so nothing
can derive it. The same goes for any judgement the facts cannot see.

The field says which of the two is in charge:

- **Follows what has happened** — the automation owns it.
- **Set by hand — the automation leaves it alone** — you do.

**Follow the facts** hands it back and works the status out immediately.

## Which word means what

The statuses are yours, from the profile — a novel goes `draft`, `revised`,
`scored`, `ready`, `published`, and a podcast has `recorded` where music has
nothing. So each status carries a `derive` role naming what it means to the
automation, and the automation reads the role rather than the word.

A status whose role is `manual` — `shelved` in every built-in profile — is
never set automatically. Rename `Published` to *Out in the world* and it keeps
meaning "it went out". See the [profile document](/kilna/reference/profile-document/)
for the field itself.

## Catching up all at once

Statuses set before the automation existed, or left behind while you worked
elsewhere, can fall out of step. **Data → Statuses** shows exactly which works
disagree with the facts, and what each would become:

```
Harbour lights    scored → scheduled
Paper boats       draft  → scheduled
```

Checking changes nothing. **Restate** applies what the list showed, and writes
one line to [History](/kilna/guides/the-history/). Works whose status you set
by hand are left out of both.

The dry run is not politeness. Restating in bulk is the one thing here with no
undo behind it — the trash holds deleted things, not overwritten fields — so
you see the list first.
