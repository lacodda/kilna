---
title: Profile document
description: Every field of a profile configuration, with its type, illustrated from the Music profile.
---

A profile is a JSON document with a `key`, a `name`, a `description`, and a
`config` object holding everything that shapes the craft. This page documents
every field of `config`, using the built-in **Music** profile
(`src-tauri/profiles/music.json`) for examples. See
[Profiles](/kilna/concepts/profiles/) for the reasoning behind the shape.

## Top level

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Stable identifier for the profile itself. |
| `name` | string | Display name — "Music", "Novel". |
| `description` | string | One sentence shown when choosing a profile. |
| `config` | object | Everything below. |

## `work_kinds`, `release_kinds`, `collection_kinds`

Each is an array of **kind** entries — the vocabulary a work, a release, or a
collection can take:

```jsonc
{ "key": "song", "label": "Song" }
```

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Stable value stored on the row. Never shown directly. |
| `label` | string | What the screen displays. Renamable at any time. |

The Music profile's `work_kinds` are `song` and `instrumental`; its
`collection_kinds` are `album`, `single` and `cycle` — a **collection** groups
works one level deep, without nesting.

A **release kind** carries one extra field:

```jsonc
{ "key": "clip", "label": "Video clip", "requires": ["lyrics", "style"] }
```

| Field | Type | Meaning |
| --- | --- | --- |
| `requires` | string[] | Version roles a release of this kind cannot ship without. Drives the [ready marks](/kilna/guides/planning-a-release/#ready-marks) and the not-ready warning. |

Every key in `requires` must name a role in `version_roles`. An empty or
absent list states no requirements: readiness is then judged on the score
alone, and every role mark reads as *not applicable* rather than *missing* —
which is how a profile written before this field existed loads. A workspace
whose stored copy states nothing gains the shipped requirements at the next
start; a list you narrowed yourself is left alone.

## `version_roles`

The independent bodies a work carries, same `{ key, label }` shape. Music
defines `lyrics` and `style`; Novel defines `text`, `outline` and `notes`. A
work can hold one current version per role, plus every prior revision of each.

## `statuses`

The states a work moves through, in order. Music: `draft`, `scored`,
`scheduled`, `released`, `shelved`.

```jsonc
{
  "key": "published",
  "label": "Published",
  "derive": "released"
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Stored on the work. |
| `label` | string | Display name; free to rename. |
| `derive` | string, optional | What this status means to the automation. Defaults to `manual`. |

`derive` is how the automation knows which of *your* words means "it went
out", without the app dictating the words. One status per meaning:

| `derive` | Set when |
| --- | --- |
| `released` | A release of this work has gone out. |
| `scheduled` | A release holds a slot in the calendar. |
| `scored` | The work has been judged at least once. |
| `draft` | Nothing has happened to it yet. |
| `manual` | Never set automatically — a decision only a person makes, like `shelved`. |

See [Statuses](/kilna/guides/statuses/) for how this plays out.

## `axes`

What a work is judged on when scored:

```jsonc
{
  "key": "hook",
  "label": "Hook",
  "weight": 2.0,
  "scale": 10.0,
  "description": "Does the chorus stay with you after one listen?"
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Stored in every score snapshot's `axes` object. Never renamed once scores exist against it — see [Scoring](/kilna/concepts/scoring/). |
| `label` | string | Display name. Free to rename; old scores stay readable under it. |
| `weight` | number | Relative importance when axes combine into a total. |
| `scale` | number | Highest value the axis accepts; values are normalized against it before weighting. |
| `description` | string, optional | Guidance shown next to the axis when scoring. |

## `tiers`

Score bands, evaluated highest-`min`-first:

```jsonc
{ "key": "clip", "label": "Clip", "min": 78.0 }
```

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Stored on a score snapshot as its computed tier. |
| `label` | string | Display name. |
| `min` | number | Minimum total (0–100) required to reach this tier. |

Music's tiers run `hold` (0), `audio` (55), `picture` (68), `clip` (78) — a
total of 80 lands in `clip`, the highest threshold it clears.

## `work_meta_fields`

Craft-specific fields stored in a work's `meta` JSON rather than as database
columns — columns per craft would produce a table that's mostly `NULL` for
any given work:

```jsonc
{ "key": "bpm", "label": "BPM", "type": "number" }
```

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Key inside `work.meta`. |
| `label` | string | Display name for the field's input. |
| `type` | `"text"` \| `"number"` \| `"date"` \| `"boolean"` | Validated in application code — SQLite doesn't type-check inside the JSON. |

Music defines `bpm` (number), `key` (text), `duration` (text) and `language`
(text). Podcast additionally uses `date` (`recorded_on`) and `boolean`
(`explicit`), showing all four types are in active use across the built-ins.

## `rhythm`

The pace releases go out at:

```jsonc
{ "every_days": 3, "default_time": "12:00" }
```

| Field | Type | Meaning |
| --- | --- | --- |
| `every_days` | number | Days the [auto-layout](/kilna/guides/planning-a-release/#the-rhythm-and-the-auto-layout) keeps between releases. `1` is daily. |
| `default_time` | string, optional | Time of day (`HH:MM`) a release usually ships, shown beside the date when editing a release. |

Calendar slots stay whole days — the contest for a date is per day, and a
time would split it — so the usual time lives here as a single fact about the
craft rather than on each release.

`rhythm` may be absent, which is how a profile written before the field
existed loads: the auto-layout then refuses with an explanation instead of
inventing a pace. A workspace whose stored copy has no rhythm gains the
shipped one at the next start; a pace you set yourself is left alone.

## `prompts`

AI panel actions scoped to this profile — see
[Writing a plugin](/kilna/guides/writing-a-plugin/) for the separate plugin
protocol; prompts are a simpler, built-in mechanism for the assistant panel
specifically.

```jsonc
{
  "key": "critique",
  "label": "Critique the lyrics",
  "description": "Weak lines, tired images, anything that does not sing.",
  "template": "Here are the lyrics of a song called \"{title}\".\n\n{role:lyrics}\n\nBe specific and be hard on it: which lines are weak, which images are worn out, what would you cut? Do not rewrite it — say what is wrong."
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Identifies the prompt. |
| `label` | string | Button text in the AI panel. |
| `description` | string, optional | Shown as a hint under the label. |
| `template` | string | The text sent to Claude, with placeholders filled per work. |

### Template placeholders

- `{title}`, `{kind}`, `{status}` — the work's own fields.
- `{body}` — the current version's body, whatever role that happens to be.
- `{role:lyrics}`, `{role:style}`, … — the latest revision of a specific
  version role, regardless of which one is current. This is what lets a
  prompt like Novel's "Check against the outline" pull in both `{role:text}`
  and `{role:outline}` at once.

An unrecognized placeholder — a typo — is left visible in the rendered prompt
rather than silently blanked, on the reasoning that a visible `{typo}` is
easier to diagnose than a quiet gap in the text sent to Claude.

`prompts` defaults to an empty list when absent, so a profile written before
the AI panel existed still loads without modification.
