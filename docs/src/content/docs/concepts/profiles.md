---
title: Profiles
description: One fixed schema, configured by profile — how Studio, Novel, Podcast and Blog speak the same structure with different vocabulary, and how a profile is edited, kept as a type and moved in a file.
---

A song, a chapter, and a podcast episode differ in **vocabulary and evaluation
criteria** — not in structure. Both have a body, both go through revisions,
both get judged, both get scheduled and shipped. kilna keeps one fixed schema
and lets a **profile** describe the craft: what a work is called, what it's
judged on, what statuses it moves through, what extra fields it carries.

The alternatives — letting users define arbitrary fields, or a database table
per craft — were rejected. Untyped fields make weighted scoring impossible to
compute; a table per craft turns every new craft into a migration plus new
code. The reasoning is in full in
[ADR 0001](https://github.com/lacodda/kilna/blob/main/docs/adr/0001-one-schema-with-profile-configuration.md).

## The built-ins

kilna ships with four crafts and an empty profile, each a JSON document rather
than code:

- **Studio** — songs with lyrics and style kept as separate version roles,
  judged on hook, lyrics, emotion, production, originality and visual
  potential. A song never goes out itself: its clips, audio releases and
  shorts are works of their own made from it — the clips and shorts judged on
  the cut, the audio release a track under one picture, each with its
  releases, cover and comments.
- **Novel** — chapters, scenes and short stories with text, outline and
  working notes kept separately, judged on pull, prose, character, structure
  and tension, shipped to beta readers, as serial instalments, submissions or
  publications.
- **Podcast** — episodes, segments and trailers with script, show notes and a
  research outline, judged on hook, clarity, pacing, insight, delivery and
  shareability, shipped as feed releases, clips or video uploads.
- **Blog** — posts, newsletter issues and guides with a draft and an angle
  kept separately, judged on hook, usefulness, clarity, angle and
  shareability, shipped as site publishes, newsletter sends or syndication.
- **Empty** — one kind of work with a text, the statuses the automation keeps
  up to date and one way out. The starting point for a craft kilna does not
  ship: everything else is named in **Settings → Profile**.

All of them carry their words in English and Russian, and the window shows the
one it speaks.

Each [space](/kilna/guides/spaces/) is made of one of them, and in a space of
another kind the same screens relabel themselves — "chapters" and "pull"
instead of "songs" and "hook" — without touching a line of application code.

## What a profile configures

A profile's `config` object is a single JSON document with these sections
(see the full field reference at
[Profile document](/kilna/reference/profile-document/)):

- `work_kinds` — the kinds of thing a work can be, each carrying the
  vocabulary below (since v0.57: a song and a video are judged, versioned
  and shipped differently, so each kind says how).
  - `release_kinds` — the doors works of this kind go out through.
  - `version_roles` — the independent bodies a work carries (lyrics/style,
    text/outline, script/notes, plot).
  - `statuses` — what a work moves through, in order, each naming what it
    means to the automation that keeps it up to date.
  - `axes` — what a work is judged on, each with a weight and a scale.
  - `tiers` — score thresholds a total lands in.
- `collection_kinds` — the kinds of thing a collection can be.
- `work_meta_fields` — craft-specific fields (BPM, point-of-view, guest name)
  stored in the work's `meta`.
- `rhythm` — how many days the craft keeps between releases, and the usual
  time of day; what the calendar's auto-layout paces itself by.
- `prompts` — AI panel templates scoped to this craft.

## Keys are stable, labels are not

Every entry in a profile — a work kind, an axis, a status — carries a `key`
and a `label`. The **key is what the database stores**; the **label is what
the screen shows**, and it can be renamed freely without consequence.

This matters most for axes. A score snapshot stores `{"hook": 8}` using the
axis key, not a foreign key into the profile's axis list. Rename "Hook" to
"Chorus strength" and every past score is still readable under the new label.
Delete the axis entirely and old scores keep the value under the old key —
they simply stop being included when a *new* total is computed, because
`ProfileConfig::total` skips whatever isn't in the current axis list rather
than trying to reinterpret it.

## Editing a profile

Profiles ship with the application and are edited in **Settings → Profile** —
the whole document, laid out in parts:

- **The craft** — the kinds of work and the order they are offered in, the
  kinds of collection, the flags and the stops of the stage dial, the order of
  the style dictionary's types, the pace, the cover ideas and the guard of
  repeats.
- **One part per kind of work** — its statuses and what each means to the
  automation, its version roles, its axes (weight, scale, the kind of answer,
  what the axis asks, the answers of a choice, the rubric), its tiers, its
  kinds of release and how each weighs the axes, what a release needs before
  it goes out, and the words of its storyboard.
- **Fields** — what a work carries beside its text: each field's type, the
  kinds that have it, the answers of a choice.
- **Actions** — the assistant's actions: the message, the method, what the
  answer becomes and what the action is about.

Every list is added to under a new key, taken from, and put in order by
dragging a row or pressing **Alt** with an arrow. A key is typed once and then
stands — see above. A word is edited in the language the window speaks and
keeps its other language.

Edits are a draft until **Save**: the draft survives leaving the section and
restarting kilna, and the bar at the foot of the pane saves or discards it.
What would not save is said as you type, beside the row it is about — an axis
with no scale, a message reading a placeholder nothing fills — and the part it
is in carries a mark, so Save is never a surprise.

Existing works keep the status and kind they were given even if you later
edit the vocabulary that named them: an old value stays visible rather than
being silently rewritten when the profile changes underneath it.

## A profile as a file, and as a type

At the head of the section:

- **Export…** writes the profile to a file, `<name>.kilna-profile.json`: the
  shape kilna's own profiles ship in.
- **Import…** reads such a file and loads it as the draft — look it over, then
  save it or discard it. A file with anything wrong in it is refused, with
  every problem named.
- **Save as a type…** keeps the profile, under a name, as something a new
  [space](/kilna/guides/spaces/#types) can be made of.

All three take the profile as it is saved, not the changes still waiting in
the draft.
