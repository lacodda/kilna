---
title: Profile document
description: Every field of a profile configuration, with its type, illustrated from the Music profile.
---

A profile is a JSON document with a `key`, a `name`, a `description`, and a
`config` object holding everything that shapes the craft. This page documents
every field of `config`, using the built-in **Studio** profile
(`src-tauri/profiles/music.json`, key `music`) for examples. See
[Profiles](/kilna/concepts/profiles/) for the reasoning behind the shape.

## Where the vocabulary lives

Since v0.57 the vocabulary a work is judged and shipped by — `axes`,
`tiers`, `version_roles`, `release_kinds`, `statuses` — belongs to the
**kind** of work, not to the profile. A studio makes songs and the videos cut
to them, and a video judged on *hook* and *lyrics* is nonsense: each kind
names its own.

```jsonc
{
  "format": 2,
  "work_kinds": [
    { "key": "song",  "label": "Song",  "axes": [...], "tiers": [...], "version_roles": [...], "release_kinds": [...], "statuses": [...] },
    { "key": "video", "label": "Video", "axes": [...], "tiers": [...], "version_roles": [...], "release_kinds": [...], "statuses": [...] }
  ],
  "collection_kinds": [...],
  "work_meta_fields": [...],
  "marks": [...], "prompts": [...], "rhythm": {...}
}
```

The lists a kind leaves out are empty for works of that kind: a kind with no
`axes` is scored empty, a kind with no `statuses` cannot hold a work and the
editor says so. What stays on the profile is what is genuinely about the
profile: collection kinds, meta fields, marks, prompts, the rhythm, the
catalogue columns.

**A flat document still reads.** A profile written the old way — the five
lists beside `work_kinds` — is taken as *every kind gets these*: each kind
that declares nothing of its own receives the flat lists, all of them, and
the document comes out in format 2. A kind that names even one list is taken
to have named its vocabulary on purpose and receives nothing from the flat
ones. The shipped Novel, Blog and Podcast profiles are written flat for that
reason; Studio writes `song` and `instrumental` flat and gives `video` and
`short` their own. A stored profile still in the old shape is rewritten once,
at the next start.

**A kind that arrives later arrives without its judgement.** When a kilna
update ships a new kind into a profile your workspace already has — the way
v0.57 shipped `video` and `short` into Studio — the kind arrives with its
statuses, roles and kinds of release, and *without* its axes and tiers. Your
axes are your own words; a stranger's would not appear silently beside them.
Works of the new kind are scored empty until you write its axes in the
profile editor. A fresh workspace gets the whole kind.

The sections below describe each list; every one of them sits under a kind.

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

A **work kind** carries the five lists described on this page — its `axes`,
`tiers`, `version_roles`, `release_kinds` and `statuses` — beside its key
and label, and, for a kind whose works are made in scenes, the two lists of
[its storyboard](#shot_types-and-scene_blocks); `release_kinds` therefore sit inside the work kind whose works go
out that way, and a video's *YouTube* and a song's *clip* are different
doors. `collection_kinds` stay on the profile.

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Stable value stored on the row. Never shown directly. |
| `label` | string | What the screen displays. Renamable at any time. |

Studio's `work_kinds` are `song`, `instrumental`, `video` and `short`; its
`collection_kinds` are `album`, `single` and `cycle` — a **collection** groups
works one level deep, without nesting.

A **release kind** carries two extra fields:

```jsonc
{ "key": "clip", "label": "Video clip", "requires": ["lyrics", "style"], "icon": "film" }
```

| Field | Type | Meaning |
| --- | --- | --- |
| `requires` | string[] | Version roles a release of this kind cannot ship without. Drives the [ready marks](/kilna/guides/planning-a-release/#ready-marks) and the not-ready warning. |
| `icon` | string | Glyph the [calendar](/kilna/guides/planning-a-release/) draws this kind with, from the list below. |
| `axis_weights` | object, optional | Axis weights that apply when a work is judged *for this kind* of release, keyed by axis key: `{ "hook": 4.0, "visual": 3.0 }`. An axis not named keeps the weight the axis itself declares. Absent means the axes' own weights — one tier for every kind. |

A clip lives or dies on its hook and its visuals; the same song as an audio
release is carried by its lyrics. `axis_weights` lets one score answer both
questions: the tier a work earns *as a clip* can differ from the tier it earns
*as an audio release*, from the same axis values. Every key must name an axis
in `axes`, and every weight must be zero or above. The verdict per kind is
computed by the same rule as the plain total (see
[Scoring](/kilna/concepts/scoring/)) and shown on the score panel, under the
total, once at least one kind names weights of its own — see
[What each release makes of it](/kilna/guides/scoring-a-work/#what-each-release-makes-of-it).
A profile where no kind reweighs anything shows nothing there: every row would
carry the number the total already gives.

Every key in `requires` must name a role in `version_roles`. An empty or
absent list states no requirements: readiness is then judged on the score
alone, and every role mark reads as *not applicable* rather than *missing* —
which is how a profile written before this field existed loads. A workspace
whose stored copy states nothing gains the shipped requirements at the next
start; a list you narrowed yourself is left alone.

### Kind glyphs

kilna does not know what kinds of release your craft ships, so the profile also
says what each one looks like. `icon` names a glyph from this list:

`audio-lines` · `book` · `book-open` · `disc` · `film` · `globe` · `image` ·
`mail` · `mic` · `music` · `newspaper` · `radio` · `rss` · `send` · `share-2` ·
`smartphone` · `users` · `video`

A kind with no `icon`, or one naming a glyph outside the list, is drawn with a
neutral calendar mark. Nothing breaks and nothing warns: the label is still
there in the chip's tooltip and in the filter above the grid. A workspace
written before the field gains the shipped glyphs at the next start, for the
kinds it still shares by key; a kind you added yourself keeps whatever you gave
it.

## `version_roles`

The independent bodies a work carries, same `{ key, label }` shape plus how
each one reads (`body`, see below). Music defines `lyrics` and `style`; Novel
defines `text`, `outline` and `notes`. A work can hold one current version per
role, plus every prior revision of each.

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
| `colour` | string, optional | The badge's emphasis on the card and down the catalogue — `plain`, `accent`, `good`, `warn`, `bad` or `info`, the same roles a mark takes. Absent draws the badge in outline; the word is always there. Studio ships *Scored* accent, *Scheduled* warn, *Released* good, *Shelved* plain, and a status without a colour gains the shipped one at the next start. |

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
| `weight` | number | Relative importance when axes combine into a total. Zero or above. |
| `scale` | number | Highest value the axis accepts; values are normalized against it before weighting. Above zero. |
| `description` | string, optional | Guidance shown next to the axis when scoring. |
| `kind` | `"scale"` \| `"flag"` \| `"choice"`, optional | What kind of answer the axis takes. Defaults to `scale`. |
| `options` | array, optional | The answers a `choice` axis offers. Required for a choice, refused on any other kind. |
| `rubric` | array, optional | What the landmark marks on this axis mean. Absent means the axis says nothing about its marks. |

### Rubrics

An axis's `description` asks the question; a rubric answers what the marks mean:

```jsonc
{
  "key": "hook",
  "label": "Hook",
  "weight": 2.0,
  "scale": 10.0,
  "description": "Does the chorus stay with you after one listen?",
  "rubric": [
    { "at": 2, "label": "you could not hum it back straight after" },
    { "at": 5, "label": "the chorus lands, but you have heard it land before" },
    { "at": 8, "label": "you catch yourself singing it hours later" }
  ]
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `at` | number | A mark on this axis's own scale, from 0 to `scale`. Not on the 0–100 total. |
| `label` | string | What that mark means, in your craft's words. |

Name only the landmarks — three or so on a scale of ten. A mark with nothing of
its own reads the nearest named mark **below** it, so scoring a 6 against the
rubric above shows the sentence written for 5. Below the lowest landmark
nothing is shown.

The point is that "is this a seven" stops being a feeling and becomes a
question with an answer: the craft says what a seven is once, and every scoring
after that is measured against the same sentence. Two landmarks naming the same
mark, or a mark outside `0`–`scale`, is refused when the profile is saved.

### Axis kinds

A **scale** takes a number up to `scale` — the axis every profile had until
v0.50. A **flag** takes yes or no: "has a chorus", "explicit". Yes is worth the
whole scale and no is worth nothing, so a flag with weight 1 and scale 10
counts exactly like a scale axis scored 10 or 0. A **choice** takes one option
from a short list, each worth a value on the scale:

```jsonc
{
  "key": "length",
  "label": "Length",
  "weight": 1.0,
  "scale": 10.0,
  "kind": "choice",
  "options": [
    { "key": "short", "label": "Too short", "value": 4.0 },
    { "key": "right", "label": "About right", "value": 10.0 },
    { "key": "long", "label": "Runs long", "value": 6.0 }
  ]
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Stored in the score snapshot. Never renamed once scores hold it. |
| `label` | string | What the option is called when scoring. |
| `value` | number | What the answer is worth, from 0 to the axis's `scale`. |

All three kinds land in the same 0–100 total, and a snapshot never records
which kind an axis was: a number reads on any kind, a boolean on a flag, an
option key on a choice. So an axis can become a flag or a choice after scores
exist against it, and the old numbers still count. An option key the profile
no longer offers is skipped like a missing axis rather than counted as zero.

The scoring interface for flags and choices arrives with the versions that
build on the model package; the profile document accepts them now.

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

A song's tiers in Studio run `hold` (0), `audio` (55), `picture` (68),
`clip` (78) — a total of 80 lands in `clip`, the highest threshold it clears.
A video's run `shelve`, `rework`, `post`, `lead`: a kind's tiers are its own.

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
| `type` | `"text"` \| `"multiline"` \| `"number"` \| `"date"` \| `"boolean"` | Validated in application code — SQLite doesn't type-check inside the JSON. |

`multiline` is for a field that runs to paragraphs — a premise, a note on where
a piece came from. It gets a text area spanning the panel rather than a
single-line box, and it is left out of the card header: the header is the line
you glance at, and a paragraph printed there pushes the work off screen.

## `note_kinds`

The kinds a note can take, each a `key` and a `label`: Studio and Novel
ship `character`, `location`, `lore` and `note`; Podcast ships `guest`,
`segment` and `note`; Blog ships `source` and `note`. A scene points at
notes of these kinds, which is what lets a board answer "every scene with
her in it" (see [Scenes](/kilna/guides/scenes/#who-is-in-it-where-it-happens)).

An optional key added in 0.65 — a document without it is the same
document, and a note still takes any kind you write. A profile naming no
kinds of note lets a scene point at any note at all; once it names some, a
scene may only point at those. A workspace made before them gains the
craft's kinds on the next launch, and one you renamed or added stays
yours, the way every vocabulary does.

`duration` is a **number of seconds**, and the Scenes tab divides it between
the scenes of a board (see [Scenes](/kilna/guides/scenes/#timing-the-board)).
It shipped as `text` holding `3:45` until 0.65; the migration retyped it and
converted what was already written. A workspace that had retyped the field
itself keeps its own version, the way every renamed field does — the board
still reads `3:45` there, but the Overview box will not hold you to a number.

Music defines the reference numbers (`bpm`, `key`, `duration`, `language`) plus
what a song is about: `tagline`, `direction`, `mood`, `tempo`, `vocal`,
`perspective`, `instruments` and a `multiline` `premise`. Podcast additionally
uses `date` (`recorded_on`) and `boolean` (`explicit`), so every type is in
active use across the built-ins.

A field added to a built-in profile after your workspace was created arrives on
the next launch, appended after your own. Fields are matched by `key`: one you
renamed or retyped keeps your version, and one you deleted comes back.

## `version_roles`

The independent bodies a work carries:

```jsonc
{ "key": "review", "label": "Review", "comments_on": "lyrics", "body": "markdown" }
```

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Stored on each version. |
| `label` | string | What the lane is called. |
| `comments_on` | string, optional | The role this one discusses. |
| `body` | `plain` or `markdown`, optional | How a body in this role is read. Defaults to `plain`. |

`body` says how the text is *shown*, never how it is stored: a `plain` role is
a monospace column exactly as typed — lyrics, a style prompt — and a `markdown`
role draws its headings, quotes and tables — a review, a chapter. The craft
says which, because the application cannot tell a lyric sheet from an essay by
looking at it. A value outside the two reads as plain, and the profile editor
says so. A workspace written before the field gains the shipped reading for
the roles it still shares by key; a choice you made yourself stays.

Most roles stand alone: lyrics and style advance separately, and the Versions
tab shows one lane at a time. A role that names `comments_on` is different —
it is written *about* another role, so it opens **beside** what it discusses
rather than in its own lane, matched revision for revision. A review of
revision 2 says nothing about revision 5, so it is not shown there.

Music ships `review` (a read against the axes) and `critique` (line-by-line),
both commenting on `lyrics`. A profile that names no commentary role keeps the
Versions tab exactly as it was.

## `marks`

Flags a work can be given by hand, beside the status the app derives:

```jsonc
{ "key": "working", "label": "Working on it", "colour": "warn", "icon": "wrench" }
```

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Stored on the work; renaming the label never touches a work. |
| `label` | string | What the chip says. |
| `colour` | `"plain"` \| `"accent"` \| `"good"` \| `"warn"` \| `"bad"` \| `"info"` | A palette role rather than a colour, so it reads in both themes. Defaults to `plain`. |
| `icon` | string | A glyph beside the word, from this list: `tag`, `wrench`, `clock`, `circle-help`, `flame`, `star`, `bookmark`, `eye`, `check`, `heart`, `lightbulb`, `thumbs-up`, `gauge`, `flag`, `pin`. A name outside it draws `tag`; the word is always there. Studio ships `wrench`, `circle-help` and `thumbs-up` for its three marks, and a mark without a glyph gains the shipped one at the next start. |

A mark is not a status: the status says where a work stands in the process and
is worked out from what happened, while a mark says something the data cannot
know — that you are fighting with this one, or that it is the good one. It
derives nothing and blocks nothing.

It is not a tag either. Tags are free text — the author's own words for what a
work *is*, completed from what the workspace already holds — and they stay with
the work. A mark comes from this short list and comes off again. Keeping them
apart means clearing the flags does not clear the vocabulary.

Marks are optional: a profile written before they existed loads with none, and
the built-in ones arrive in an existing workspace on the next launch.

## `rhythm`

The pace releases go out at:

```jsonc
{ "every_days": 3, "default_time": "12:00" }
```

| Field | Type | Meaning |
| --- | --- | --- |
| `every_days` | number | Days the [auto-layout](/kilna/guides/planning-a-release/#the-rhythm-and-the-auto-layout) keeps between releases. `1` is daily. |
| `default_time` | string, optional | Time of day (`HH:MM`) a release usually ships, shown beside the date when editing a release. |

Calendar slots stay whole days, and the usual time lives here as a single fact
about the craft. The original reason was that a date was contested per day and
a time would have split the contest; the contest went in v0.44 and the shape
stayed, because the calendar is read a month at a time and a column of clock
times is not what makes a month legible. Since v0.50 a release can also carry
its own time of day and time zone, for the platforms that ask for one; this
default is what a release starts from.

## `catalogue_columns`

Which columns the [catalogue](/kilna/guides/the-catalogue/#choosing-the-columns)
shows, by column id, in order:

```jsonc
"catalogue_columns": ["title", "marks", "tier", "total", "scored", "updated"]
```

A novel and a record are read down different columns, which is why the list
belongs to the profile rather than to the machine. kilna writes it when you
pick columns in the catalogue; you can also edit it here. An id this build
does not know is dropped on read rather than refused, and the title is always
shown. Absent means the catalogue's own default — and a workspace from before
the field opens on what the machine remembered, writing that list here once,
so the move costs nobody their layout.

## `catalogue_columns_by_kind`

The columns the catalogue shows while it is narrowed to one kind of work, by
kind key:

```jsonc
"catalogue_columns_by_kind": {
  "video": ["title", "marks", "versions", "updated"]
}
```

A video is read down other columns than a song. kilna writes an entry when you
choose columns with the catalogue narrowed to that kind; a kind without an
entry reads down `catalogue_columns`. Optional — a document without the key is
the same document, and a build that does not know an id drops it on read.

`rhythm` may be absent, which is how a profile written before the field
existed loads: the auto-layout then refuses with an explanation instead of
inventing a pace. A workspace whose stored copy has no rhythm gains the
shipped one at the next start; a pace you set yourself is left alone.

## `shot_types` and `scene_blocks`

Two optional lists on a **work kind**, for a kind whose works are made in
scenes — Studio's `video` and `short`. A kind that names neither has no
storyboard, and its cards draw no [Scenes](/kilna/guides/scenes/) tab. A
document without them is the same document: format 2 is not changed.

```jsonc
{
  "key": "video", "label": "Video",
  "shot_types": [
    { "key": "wide", "label": "Wide" },
    { "key": "close", "label": "Close-up" },
    { "key": "detail", "label": "Detail" }
  ],
  "scene_blocks": [
    { "key": "still", "label": "Still frame", "hint": "The frame as a picture: subject, light, lens, mood." },
    { "key": "motion", "label": "Animation", "hint": "What moves, and how the camera moves, from that frame." },
    { "key": "negative", "label": "Negative", "hint": "What must not appear." }
  ]
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `shot_types[].key` | string | Stored on the scene as its kind of shot. A scene can only take a key this list names — the board is narrowed by it, and a kind of shot typed freely would never be found. |
| `shot_types[].label` | string | What the strip and the picker show. Renamable. |
| `scene_blocks[].key` | string | The key a scene stores that block's text under, and the key a template (v0.62) will read. A block under a key this list does not name is refused on write. |
| `scene_blocks[].label` | string | The caption over the box and on its copy button. |
| `scene_blocks[].hint` | string, optional | A line under the box saying what goes in it. |

Studio also gives both kinds a `context` [version role](#version_roles)
for what every scene shares — the hero, the palette, the lens. A workspace
that already has the video kinds gains the two lists at the next start,
where its stored copy names none; a list you narrowed is left alone.

## `prompts`

Assistant actions scoped to this profile — see
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
| `key` | string | Identifies the prompt. Also what kilna recognises a running action by, so an action started from a card cannot be started twice at once. |
| `label` | string | Button text, in the AI panel and on a work's Overview tab. |
| `description` | string, optional | Shown as a hint under the label. |
| `template` | string | The message sent to Claude, with placeholders filled per work. Keep it short: the method carries the how. |
| `method` | string, optional | How the action is done — the role the assistant takes, what it checks and in what order, the shape of the answer, what it must never say. Markdown; appended to the model's system prompt on every turn of the chat the action opened. See [ADR 0021](https://github.com/lacodda/kilna/blob/main/docs/adr/0021-an-action-carries-its-method.md). |
| `produces` | string, optional | What the action asks for beyond prose: `"score"`; `"version:<role>"` — the whole answer offered as a version in that role; `"scenes"` — a storyboard to replace the board, or `"scenes:add"` and `"scenes:revise"`. Anything else loads as prose and is refused when the profile is saved. |
| `kinds` | list of strings, optional | The work kinds the action is offered on. Absent or empty is every kind. An action that reads `{role:lyrics}` is for the kinds that have lyrics — Studio's song actions say `["song"]` — because a button for it on a video would send a prompt with a hole in it. |
| `scope` | string, optional | `"scene"` for an action started from a row of the storyboard: it reads the row as `{scene}`, is offered on each scene rather than above the board, and must produce `scenes:revise`. Absent is the work. |

The same prompt is offered in three places: in the panel it fills the composer
for you to read and send, typing `/` reaches the same list from the keyboard,
and on **Overview** — and on **Scenes**, for a kind with a storyboard — a click
starts it at once in a chat of its own, with an eye beside the button that
shows exactly what the click sends. See
[The assistant](/kilna/guides/the-assistant/).

### `produces`

An action with `"produces": "score"` asks the assistant to end its reply with a
json block holding a value for every axis of the work's kind. kilna appends
that instruction itself — the axes with their labels, descriptions, rubric
marks and weights, and the tiers with what the total means — so the template
only has to say what to judge and how honestly. A template that names axes of
its own contradicts the instruction; the shipped `score` templates no longer
do, and a stored copy still reading exactly as it shipped before v0.61 follows.

An action with `"produces": "version:<role>"` offers its whole answer as a
version in that role — Studio's `critique` produces `version:critique`. The
instruction kilna appends tells the model to write only the text, with no
preamble, since the answer is kept word for word.

The answer comes back with the numbers laid out and a button that applies them
as an ordinary score. **kilna never lets the assistant write to the workspace**:
a proposal is read before it becomes a fact, always. A value past an axis's
scale is clamped rather than refused, an axis the profile does not have is shown
and not applied, and an answer that ignored the instruction and replied in prose
is simply an answer.

An action with `"produces": "scenes"` asks for a storyboard: kilna appends the
shape of the json block with the kind's kinds of shot and prompt blocks spelled
out, and the answer comes back as the same proposal an agent's `propose_scenes`
makes — the board as a table, every block under it, and one button. `scenes`
is the whole board, replacing what is there by number; `scenes:add` puts the
scenes after the last; `scenes:revise` changes only the scenes it numbers, and
only in the fields it gives — the shape a scene action needs. A block in the
wrong words — a kind of shot the profile does not have, a revision that
numbers another scene — is not silently nothing: the chat says why there is
no button. See [Scenes](/kilna/guides/scenes/#actions-on-the-board).

Every shipped profile carries a `score` action. Anything else declaring
`produces` gets the same treatment; an unrecognised value is ignored when the
profile loads, so a profile written for a future kilna still opens — and named
when the profile is saved, so a typo does not become an action that never
proposes.

### Template placeholders

- `{title}`, `{kind}`, `{status}` — the work's own fields.
- `{body}` — the current version's body, whatever role that happens to be; or,
  for an action started from the versions tab, the revision open there.
- `{role:lyrics}`, `{role:style}`, … — the latest revision of a specific
  version role, regardless of which one is current. This is what lets a
  prompt like Novel's "Check against the outline" pull in both `{role:text}`
  and `{role:outline}` at once. An action started on a revision reads that
  revision for its own role. A role the work has no version in yet is a
  refusal — *“Harbour lights” has no Plot yet: write it first* — not an
  empty gap.
- `{scenes}` — the storyboard as a table: number, section, seconds, kind of
  shot, description. The blocks stay out. An empty board says so.
- `{scene}` — the scene a scene action was started on, whole: its fields and
  every block it holds. Only in an action with `"scope": "scene"`.
- `{donor}` — the first work this one was [made from](/kilna/guides/made-from/),
  as *“Harbour lights” (song)*; `{donor:lyrics}`, `{donor:style}`, … — the
  latest revision of that role on the donor. A work made from nothing refuses
  the action and says to link the source first.

Saving a profile checks every template against the vocabulary it reads: a
placeholder nothing fills, a `{role:x}` not every kind of the action has,
`{scenes}` on a kind without a storyboard, a scene action that never reads
`{scene}` — each is named and the save is refused, the way a duplicate axis
key is. This is the check the predecessor lacked: a template edited to lose
the placeholder carrying the text sent critiques of nothing for a month. At
render time an unrecognized placeholder is still left visible rather than
silently blanked, for a stored profile that predates the check.

`prompts` defaults to an empty list when absent, so a profile written before
the AI panel existed still loads without modification.
