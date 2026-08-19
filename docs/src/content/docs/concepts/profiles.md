---
title: Profiles
description: One fixed schema, configured by profile — how Music, Novel, Podcast and Blog speak the same structure with different vocabulary.
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

## The four built-ins

kilna ships with four profiles, each a JSON document rather than code:

- **Music** — songs and instrumentals with lyrics and style kept as separate
  version roles, judged on hook, lyrics, emotion, production, originality and
  visual potential, shipped as clips, shorts or audio releases.
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

Switch the active profile from the workspace screen and the same screens
relabel themselves — "chapters" and "pull" instead of "songs" and "hook" —
without touching a line of application code.

## What a profile configures

A profile's `config` object is a single JSON document with these sections
(see the full field reference at
[Profile document](/kilna/reference/profile-document/)):

- `work_kinds`, `release_kinds`, `collection_kinds` — the kinds of thing a
  work, a release, and a collection can be.
- `version_roles` — the independent bodies a work carries (lyrics/style,
  text/outline, script/notes).
- `statuses` — what a work moves through, in order, each naming what it means
  to the automation that keeps it up to date.
- `axes` — what a work is judged on, each with a weight and a scale.
- `tiers` — score thresholds a total lands in.
- `work_meta_fields` — craft-specific fields (BPM, point-of-view, guest name)
  stored in the work's `meta`.
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

Profiles ship with the application and are editable — you pick and adjust
rather than design a schema from a blank page. Existing works keep the status
and kind they were given even if you later edit the vocabulary that named
them: an old value stays visible rather than being silently rewritten when the
profile changes underneath it.
