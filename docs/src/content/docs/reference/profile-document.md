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
  "format": 3,
  "work_kinds": [
    { "key": "song",  "label": "Song",  "axes": [...], "tiers": [...], "version_roles": [...], "release_kinds": [...], "statuses": [...] },
    { "key": "video", "label": "Video", "axes": [...], "tiers": [...], "version_roles": [...], "release_kinds": [...], "statuses": [...] }
  ],
  "collection_kinds": [...],
  "work_meta_fields": [...],
  "marks": [...], "stages": [...], "prompts": [...], "rhythm": {...}
}
```

The lists a kind leaves out are empty for works of that kind: a kind with no
`axes` is scored empty, a kind with no `statuses` cannot hold a work and the
editor says so. What stays on the profile is what is genuinely about the
profile: collection kinds, meta fields, marks, stages, prompts, the rhythm, the
catalogue columns.

**A flat document still reads.** A profile written the old way — the five
lists beside `work_kinds` — is taken as *every kind gets these*: each kind
that declares nothing of its own receives the flat lists, all of them, and
the document comes out in format 2. A kind that names even one list is taken
to have named its vocabulary on purpose and receives nothing from the flat
ones. The shipped Novel, Blog and Podcast profiles are written flat for that
reason; Studio writes `song` flat and gives `video`, `audio` and `short` their
own. A stored profile still in the old shape is rewritten once,
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
[its storyboard](#shot_types-scene_blocks-and-cover); `release_kinds` therefore sit inside the work kind whose works go
out that way. A door belongs to the work that goes through it: the clip cut to
a song is a `video` with doors of its own, its audio release an `audio` work,
its shorts `short` works. `collection_kinds` stay on the profile.

**A kind may have no door at all.** Since v0.86 Studio's `song` lists no
`release_kinds`: a song is the thing - its text, its style, its score - and it
goes out as what is [made from it](/kilna/guides/made-from/). Its status is
read from those works' releases (see [Statuses](/kilna/guides/statuses/)), its
card has no Files tab and no release, its overview lists its publications, and its
comments are summed up from theirs. A release planned on a kind with no door is
refused with that reason.

A work kind may also say:

| Field | Type | Meaning |
| --- | --- | --- |
| `cover` | boolean, optional | Its works go out under a cover built in the [constructor](/kilna/guides/the-cover/): a picture, a negative and a title written from what is chosen. The card draws a **Cover** tab. Absent is no cover. See [below](#shot_types-scene_blocks-and-cover). |
| `frame` | boolean, optional | Its works play under one picture for their whole length - an audio release on a video platform - and so have a [frame](/kilna/guides/files-and-covers/#the-frame): a still, a loop of what moves in it, and a negative. The card draws a **Frame** tab. Absent is no frame. |
| `made_title` | string or map, optional | What a work of this kind is called when it is made from another: `{title}` is the title of what it is all made from - the song, even for a short cut from its clip - and `{n}` its number among the works of this kind made from it. Studio's are one string for every language: `"{title} (video)"`, `"{title} (audio)"`, `"{title} (short)"`. A template without `{n}` numbers only the second and later ones, inside a closing bracket when it ends with one: *Tide (short)*, *Tide (short 2)*. A title one of them already has is skipped. A map per language is still read, in the window's language. Absent means the source's title as it is. |
| `open_on` | string, optional | The tab a work of this kind opens on when the address names none - from the catalogue, the calendar, a search hit: `versions`, `score`, `scenes`, `cover`, `frame`, `files`, `links`, `notes`, `comments`, `assistant`, `history` or `overview`. Studio's song could open on `versions`, its short on `scenes`. kilna writes it when you choose in **Settings › The work card**. A tab the kind's works do not have, or a word this build does not know, opens the Overview; absent is the Overview. Added in v0.90.1 - before, one tab for every card was kept on the machine. |

The window's order of `work_kinds` is the order the **Make…** menu lists
them in.

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Stable value stored on the row. Never shown directly. |
| `label` | string or object | What the screen displays. Renamable at any time. See [Labels in more than one language](#labels-in-more-than-one-language). |

### Labels in more than one language

A label is either a plain string or a map from locale to string:

```jsonc
{ "key": "scored", "label": "Scored" }
{ "key": "scored", "label": { "en": "Scored", "ru": "Оценено" } }
```

Both shapes are read wherever a label is read — `label`, `description` and
`hint`, at every depth of the document, including an axis's `rubric` marks and
a release kind's `fields`. The window shows the entry for the language it is
in; it falls back to `en`, and then to whatever the map does hold, so a profile
carrying only one language still shows a word rather than a blank.

The profiles that ship with kilna carry English and Russian, because a window
set to Russian reading *Scored · Song* was the interface translated around a
hole in its own vocabulary. A label **you** write stays exactly as you write
it: a plain string is never rewritten into a map on your behalf, and renaming
a word in Settings replaces it with the one word you typed.

`template` and `method` are deliberately **not** translated. They are
instructions to a model rather than words on a screen, and translating one
changes what the assistant does rather than what the window says.

Studio's `work_kinds` are `song`, `video`, `audio` and `short` (the
instrumental of earlier versions is now a `variant` of an audio release); its
`collection_kinds` are `album`, `single` and `cycle` — a **collection** groups
works one level deep, without nesting.

A **release kind** carries two extra fields:

```jsonc
{ "key": "audio", "label": "Audio release", "requires": ["lyrics", "style"], "icon": "disc" }
```

| Field | Type | Meaning |
| --- | --- | --- |
| `requires` | string[] | Version roles a release of this kind cannot ship without. Drives the [ready marks](/kilna/guides/planning-a-release/#ready-marks) and the not-ready warning. |
| `icon` | string | Glyph the [calendar](/kilna/guides/planning-a-release/) draws this kind with, from the list below. |
| `axis_weights` | object, optional | Axis weights that apply when a work is judged *for this kind* of release, keyed by axis key: `{ "hook": 4.0, "visual": 3.0 }`. An axis not named keeps the weight the axis itself declares. Absent means the axes' own weights — one tier for every kind. |
| `cover_format` | string, optional | The shape of the picture a release of this kind goes out with, width to height: `"16:9"` for a video platform's preview, `"9:16"` for a vertical short, `"1:1"` for a streaming cover. The place decides the shape, so a work that goes out in two places needs two covers; the **Cover** tab lists the shapes of a work's releases. Refused at save when it is not two whole numbers with a colon. |

A premiere lives or dies on its dynamics — the room is watching it live;
the same video as an ordinary upload is carried by its fit to the track.
`axis_weights` lets one score answer both questions: the tier a work earns
*as a premiere* can differ from the tier it earns *as an upload*, from the
same axis values. Every key must name an axis in `axes`, and every weight
must be zero or above. The verdict per kind is computed by the same rule as
the plain total (see
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

### What a release goes out as

A release kind may name the fields a release of it ships with — the title it
goes out under, the text beneath it, the words it is found by:

```json
{
  "key": "youtube",
  "label": "YouTube",
  "requires": ["plot"],
  "icon": "film",
  "fields": [
    { "key": "title", "label": "Title", "type": "line", "template": "{title}", "limit": 205 },
    { "key": "description", "label": "Description", "type": "text", "template": "{role:plot}" },
    { "key": "tags", "label": "Tags", "type": "tags", "hint": "Comma separated." },
    { "key": "pinned", "label": "Pinned comment", "type": "text" }
  ]
}
```

| Key | What it is |
| --- | --- |
| `key` | What the value is stored under, in the release's `meta`. Renaming the label never loses what was written; renaming the key does. |
| `label` | The word above the box. |
| `type` | `line` (a title), `text` (paragraphs) or `tags` (a list, kept as text with commas between). Absent is `line`. |
| `template` | What the field is filled with when generated, in the placeholder language [below](#template-placeholders). Absent means the field is only ever typed by hand. |
| `hint` | A line under the box saying what goes in it. |
| `limit` | How many characters the destination accepts. Counted beside the box, never enforced — kilna is not the authority on what a platform takes this month. |
| `suffix` | A tail the value always ends with, whoever writes the rest: Studio's audio YouTube title keeps `" (audio)"`. The release keeps it at every write - typed, generated, proposed, replayed - and takes it off when the release moves to a place that keeps none. An empty value stays empty; within `limit`, the tail is what stays. |

A kind with no `fields` says nothing about itself, and its releases show no
boxes. That is the state of every profile written before the field existed; a
workspace made before it gains the shipped lists for the release kinds it
still shares by key, and a kind you have already given fields of your own is
left alone.

A field's template is checked against **one** work kind — the kind that owns
the release kind — which makes the check sharper than an action's:
`{role:plot}` in a song's audio release is refused at save even though the
video kind has a plot. `{scene}` is refused outright: it is one row of a
storyboard, filled from the row an action was started on, and a release is
about the whole work. Use `{scenes}` for the board.

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
{ "key": "clip", "label": "Strong", "min": 78.0 }
```

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Stored on a score snapshot as its computed tier. Never rename one: the snapshots under it would be orphaned, which is why Studio's song tiers were reworded in v0.74.2 and kept their keys. |
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
| `type` | `"text"` \| `"multiline"` \| `"number"` \| `"date"` \| `"boolean"` \| `"choice"` | Validated in application code — SQLite doesn't type-check inside the JSON. |
| `kinds` | string[], optional | The work kinds that have the field. Absent or empty is every kind. |
| `options` | kind entries, for `choice` | The answers a choice offers, `{ "key", "label" }`, stored by key. Required for a choice and refused on any other type. |
| `default` | value, optional | What a new work of a kind that has the field starts with: an option's key for a choice. |
| `own` | boolean, optional | A fact of the work's own media, never taken from the work it is made from: Studio's `duration` - a short is not as long as the clip it is cut from. Absent is `false`. |

Studio's `variant` is a choice for `audio` alone - the original, the
instrumental, a slowed or a sped-up version, a remix - starting at the
original:

```jsonc
{
  "key": "variant", "label": { "en": "Variant", "ru": "Вариант" }, "type": "choice",
  "kinds": ["audio"], "default": "original",
  "options": [
    { "key": "original", "label": { "en": "Original", "ru": "Оригинал" } },
    { "key": "instrumental", "label": { "en": "Instrumental", "ru": "Инструментал" } }
  ]
}
```

A work made from another takes the source's fields its own kind has - except
the ones marked `own` - and the defaults of the rest.

`multiline` is for a field that runs to paragraphs — a premise, a note on where
a piece came from. It gets a text area spanning the panel rather than a
single-line box, and it is left out of the card header: the header is the line
you glance at, and a paragraph printed there pushes the work off screen.

## `note_kinds`

The kinds a note can take, each a `key` and a `label`. A scene points at
notes of these kinds, which is what lets a board answer "every scene with
her in it" (see [Scenes](/kilna/guides/scenes/#who-is-in-it-where-it-happens)).

An optional key added in 0.65 — a document without it is the same
document, and a note still takes any kind you write. A profile naming no
kinds of note lets a scene point at any note at all; once it names some, a
scene may only point at those. A workspace made before them gains the
craft's kinds on the next launch, and one you renamed or added stays
yours, the way every vocabulary does.

Two flags, both optional, say what is different about a kind (v0.85,
[ADR 0045](https://github.com/lacodda/kilna/blob/main/docs/adr/0045-material-is-spent-not-moved.md)):

| Field | Type | Meaning |
| --- | --- | --- |
| `material` | boolean, optional | Notes of the kind are what works are made from, and are spent by them — an idea, a phrase. Such a note says where it stands: fresh, used, parked or dropped. *To a work* ties it to the work and marks it used in one gesture, and *Make it a work* leaves it in the bank, used, instead of moving it. |
| `line` | boolean, optional | A note of the kind is one line. The Notes screen keeps the kind in a bank of its own, a row each, and leaves it out of *All*. |

Every shipped profile names `idea` (material) and `phrase` (material, one
line). A kind with sections — a card of the canon — can be neither; saving
says so.

### Cards of the canon

A kind that names `sections` is a kind of **card**: its notes leave the
Notes screen for the [Canon](/kilna/guides/the-canon/), and what a card
knows is written as facts filed under those sections. A kind without
sections is a plain note. Studio ships a channel, characters, locations,
objects, groups, events, themes and lore as cards, and `note` as a plain
note; Novel ships characters, locations, objects, groups, events, themes
and lore; Podcast ships guests and segments; Blog ships only plain kinds.

```jsonc
{
  "key": "character",
  "label": "Character",
  "icon": "user",
  "describe_from": ["looks"],
  "sections": [
    { "key": "identity", "label": "Identity", "lenses": ["cover", "public"] },
    { "key": "looks", "label": "Looks", "hint": "What a picture needs.", "lenses": ["cover"] },
    { "key": "bio", "label": "Biography" },
    { "key": "circle", "label": "Around", "shape": "relations" },
    { "key": "where", "label": "Where it appears", "shape": "appearances" }
  ]
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `icon` | string, optional | The glyph the kind is drawn with in the card list: `radio`, `user`, `map-pin`, `box`, `users`, `calendar`, `sparkle`, `book`, `list`, `note`, `lightbulb`, `quote`. Anything else draws a plain note. |
| `sections` | list, optional | What a card of this kind knows, in the order the card reads. Empty or absent: a plain note. |
| `root` | boolean, optional | One card of this kind per workspace — the channel, the root of the world. Studio's `channel` is the root. A second card of a root kind is refused, and a profile with two root kinds is refused when it is saved. |
| `describe_from` | list of strings, optional | The sections the card's description for a picture generator is written from — a person's looks, not their biography. When a settled public fact in one of them changes, the description says it is stale. Absent: the description is written by hand and never goes stale. Each key must be a section of the kind. |

A section:

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Identifies the section. A fact is filed under it, so renaming a key leaves its facts under a section the card no longer names; they still read, at the end of the card, under *Other*. |
| `label` | string | The section's name on the card. |
| `hint` | string, optional | A line under the name: what goes here, and what reads it. |
| `shape` | string, optional | What an entry of the section is (below). Statements when absent. |
| `lenses` | list, optional | The outward tasks that read this section: `cover` (a picture — a cover, a frame) and `public` (what is said in public — a release's text, a reply). The work itself reads every section, so `work` need not be named. A section naming none is read by the work alone. |
| `kinds` | list of strings, optional | For a section of relations: the kinds of card it gathers — Studio's character keeps the events she was part of apart from the people around her. Empty gathers every relation no other section of the card claims. |

The shapes:

| Shape | An entry is |
| --- | --- |
| `facts` | A statement. The default. |
| `slots` | A named value: a caption's slot and its words, a template's parts. |
| `details` | A signature detail: its name, a template in English for a generator (`[[card:id]]` brings in a card's description), the pictures it belongs in — `cover`, `frame`, `scene` — and whether it is on by default. The [cover constructor](/kilna/guides/the-cover/) shows each as a switch. |
| `palette` | A colour with its name. |
| `marks` | A variant of a mark: its code, what it means, a description for a generator, and its files. |
| `styles` | A house style: a brick of the [style dictionary](/kilna/guides/styles/). |
| `bans` | A statement of what must not appear. Read through the `cover` lens into the negative of a picture (a cover, a frame, a scene), through the `public` lens by what is said in public. Studio keeps the two apart: `bans` for texts, `picture_bans` for pictures, on the channel and on a character. |
| `relations` | Holds no facts: the card's relations to other cards, drawn on the card. |
| `appearances` | Holds no facts: where the card appears, counted from the works it is the hero of, the scenes that point at it, the texts that name it, the covers that picture it and the works its facts cite. |

A fact is read by a lens when **one rule** says so: the work reads every
fact that is not retired; a cover and a public text read only a fact that is
settled (`canon`), `public`, on a card that is public itself, in a section
naming that lens. The Canon screen dims what a lens does not see, the MCP
tool answers through it, and every prompt reads through it.

Saving a profile checks the canon it names: kinds and sections are unique
by key, a section's shape and lenses are known words, `describe_from` names
sections of the kind, a section of relations names kinds of card the
profile has, and only one kind is the root. A workspace made before 0.84
gains the craft's sections, root and glyphs on the next launch; a kind whose
sections you already changed keeps yours.

## `relation_kinds`

The kinds a relation between two cards can take, each a `key` and a
`label` — Studio's `family`, `partner`, `friend`, `neighbour`, `colleague`,
`pet`, `member`, `place`, `owner`, `event` and `other`. A relation is drawn
once per pair, with a kind from this list, a word for each side
("neighbour" one way, "neighbour and first listener" the other) and a layer
of its own. A kind the profile does not have is refused, and a profile with
two relation kinds under one key is refused when it is saved. A profile
naming none has not decided, and any word goes — the leniency a kind of
note has.

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
| `counts_as_version` | boolean, optional | Whether a body in this role is a time the work was written. Defaults to "yes, unless it comments on something". |
| `sung` | boolean, optional | Whether a body in this role is sung: its words are checked for where the stress falls and how you sing them, the stress gesture and "show the stresses" work in it, and a public text made from it has the marks taken off. Studio's `lyrics` is. Absent is `false`. See [Singing a text](/kilna/guides/singing-a-text/). |

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

Music ships `review` (a read against the axes), `critique` (line-by-line)
and `neighbours` (where the lyric says a spent image in other words, and which
works it says the same thing as), all commenting on `lyrics`. A profile that names no commentary role keeps the
Versions tab exactly as it was.

`counts_as_version` answers a different question: how many times the *work*
has been written, which is the number the catalogue's Versions column shows.
Commentary is excluded by definition — a critique is not a draft of the song —
but so is anything else the craft says is written *about* the work rather than
being it. Music sets `counts_as_version: false` on `style`: a style prompt
stands on its own and comments on nothing, so the count read a song written
twice as having six versions. The role keeps its own lane, its own revisions
and its own history; it simply is not counted as the song.

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

## `stages`

The stops on the way from an idea to a finished work — what the dial beside the
star snaps to:

```jsonc
{ "key": "polish", "label": "Polishing", "percent": 80, "colour": "accent" }
```

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Names the stop. The work stores the percentage, not this, so renaming a stop never touches a work. |
| `label` | string | What the tooltip says, and what `stage:` accepts in the catalogue's box. |
| `percent` | integer 0–100 | Where the stop sits, and how much of the dial it fills. |
| `colour` | `"plain"` \| `"accent"` \| `"good"` \| `"warn"` \| `"bad"` \| `"info"` | A palette role rather than a colour. Defaults to `plain`. |

A stage is not a status. A status says where the work stands in the *process*
and is derived from facts — it was scored, a release was booked, it shipped. A
stage says how finished the *work itself* is, which no fact can answer: a song
can have a complete lyric and still be three verses of placeholder, and only
its author knows that. The two are independent, and a work is routinely
`Scored` and `Polishing` at once.

It is not a mark either: a mark is raised or not, and the question here is one
of degree.

**The work stores a percentage, not a key.** A dial is a fraction by nature, and
a second table mapping key to fraction would be a second truth about the same
thing. A percentage between two stops belongs to the lower one — 79 is still
*Polishing*, because rounding up would tell you a song is finished when you
said it was nearly.

**Unset is a third state.** A work nobody has judged draws an empty ring, and it
is not the same as a work judged to be a bare idea at 0 — that one draws a dot.
`Backspace` on the dial, or clicking the stop it already stands on, takes a work
back to unset.

Stages are optional: a profile that names none uses the line's seven — *Idea*,
*Rough draft*, *Half there*, *Nearly there*, *Polishing*, *Finished*, *Final*,
at 0, 17, 33, 50, 67, 83 and 100. A dial with nothing to snap to is not a dial,
so this is one of the few places the app answers for a craft that said nothing.

*Final* is the last stop rather than *Finished* because those are two different
claims: a work can be finished for a month before it goes out, and until v0.74.2
nothing on a row told them apart. It is set by hand like every other stop — the
stage stays a judgement, and whether a release actually shipped is what the
status already derives.

**Upgrading a profile renumbers its stops.** A stop that keeps a key the shipped
profile still has takes that profile's `percent`, while its *label* stays
whatever you renamed it to. This is the one field of a vocabulary entry that
cannot be left alone: a stage is not a word but a word at a position, and a
stored profile that gained *Final* at 100 while keeping *Finished* at 100 would
hold two stops on one number. A stop you added yourself is not in the shipped
list and is never touched.

## `cover_ideas`

How many ideas for its cover a publication is given when it is made -
**Make a clip**, **Make an audio**, **Make a short** start them beside the
release meta. A number from 0 to 5; 0 asks for none; absent is 3. Set in
**Settings → Profile**. Added in v0.89: a document without it is the same
document.

## `guard`

How the [guard of repeats](/kilna/guides/the-guard-of-repeats/) reads its two
words:

```jsonc
{ "window_days": 90, "rare_rank": 20000, "rare_in_works": 2 }
```

| Field | Default | Meaning |
| --- | --- | --- |
| `window_days` | 90 | How close another song's day is too close: a rare word it shares is red inside, orange outside. |
| `rare_rank` | 20000 | From which rank of the language's stems a Russian word is rare, by the language pack's frequency list. A stem the list does not hold is rarer than all of them. A word in another script is rare only when the bank keeps it. |
| `rare_in_works` | 2 | In how many of your works a word may stand and still be rare; past it the word is your own, and spent only if the register says so. |

Absent means all three defaults. Set in **Settings → Profile**. Added in
v0.90: a document without it is the same document.

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

## `shot_types`, `scene_blocks` and `cover`

Two optional lists and a flag on a **work kind**. The lists are for a kind whose
works are made in scenes — Studio's `video` and `short`. A kind that names
neither has no storyboard, and its cards draw no [Scenes](/kilna/guides/scenes/)
tab. A document without them is the same document: format 2 is not changed.

```jsonc
{
  "key": "video", "label": "Video",
  "shot_types": [
    { "key": "wide", "label": "Wide" },
    { "key": "close", "label": "Close-up" },
    { "key": "detail", "label": "Detail" }
  ],
  "scene_blocks": [
    { "key": "still", "label": "Still frame", "hint": "The frame as a picture: subject, light, lens, mood.", "picture": true },
    { "key": "motion", "label": "Animation", "hint": "What moves, and how the camera moves, from that frame." },
    { "key": "negative", "label": "Negative", "hint": "What must not appear." }
  ],
  "cover": true
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `shot_types[].key` | string | Stored on the scene as its kind of shot. A scene can only take a key this list names — the board is narrowed by it, and a kind of shot typed freely would never be found. |
| `shot_types[].label` | string | What the strip and the picker show. Renamable. |
| `scene_blocks[].key` | string | The key a scene stores that block's text under, and the key a template (v0.62) will read. A block under a key this list does not name is refused on write. |
| `scene_blocks[].label` | string | The caption over the box and on its copy button. |
| `scene_blocks[].hint` | string, optional | A line under the box saying what goes in it. |
| `scene_blocks[].picture` | boolean, optional | The block that says what the picture shows. A scene with a built frame has its still written around it, in the clip's style (see [the cover](/kilna/guides/the-cover/#a-scene-of-a-clip)). One per kind; a kind that marks none has no built frames. |
| `cover` | boolean, optional | Its works go out under a cover built in the constructor. |

Studio also gives both kinds a `context` [version role](#version_roles)
for what every scene shares — the hero, the palette, the lens. A workspace
that already has the video kinds gains the two lists at the next start,
where its stored copy names none; a list you narrowed is left alone.

A cover's parts are the application's, not the craft's (v0.88, ADR 0049):
the constructor writes a picture, a negative and - when the title goes apart -
a typography block from what is chosen, so a kind says only whether its works
have one. Studio's `video`, `audio` and `short` do; a song goes out as what is
made from it, and has none. **Format 2** named the parts of a cover's prompt
here as `cover_blocks`; a document that still does is read as `"cover": true`
and written back in format 3. A workspace that has the kinds gains the flag,
and the still's `picture` mark, at the next start.

The frame of a kind with `"frame": true` is **not** a list of blocks: its
parts are the application's - the still, what moves in the loop, the loop's
length, a still camera, a seamless join, the negative - because the loop's
prompt is written from its settings (ADR 0046). See
[Files and covers](/kilna/guides/files-and-covers/#the-frame).

## `style_types`

An optional list on the **profile**, not on a work kind: the types a
[style](/kilna/guides/styles/) can be. A style is a part a picture prompt is
built from, and the same character stands in the videos and in the shorts —
so the dictionary belongs to the workspace, not to one kind of work. A
profile that names none has no style dictionary, and the rail draws no
**Styles** entry. Added in v0.75 — a document without it is the same
document.

```jsonc
{
  "style_types": [
    {
      "key": "image-style",
      "label": { "en": "Image style", "ru": "Стиль изображения" },
      "hint": {
        "en": "Describe the render technique and the look: medium, film stock and grain, palette, light, processing. Not what is in the picture — only how it looks.",
        "ru": "Опиши технику рендера и вид: медиум, плёнка и зерно, палитра, свет, обработка. Не что на картинке — только как это выглядит."
      },
      "icon": "palette"
    },
    {
      "key": "character",
      "label": { "en": "Character", "ru": "Персонаж" },
      "hint": { "en": "Describe the person as such: age, build, face, hair, distinguishing marks. No clothing, no surroundings.", "ru": "Опиши человека как такового: возраст, телосложение, лицо, волосы, приметы. Без одежды и окружения." },
      "icon": "user"
    }
  ]
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Stored on the style as its type. A style can only take a key this list names — the dictionary is grouped and narrowed by it. A style written under a key later dropped from the document still reads, and shows the key. |
| `label` | string or map | What the chips, the groups and the picker show. Renamable, and bilingual like every other word of the craft. |
| `hint` | string or map, optional | **What to describe for a style of this type.** Reaches the assistant when it describes one, and never reaches a generator. |
| `icon` | string, optional | A glyph from the closed set: `palette`, `user`, `shirt`, `tree`, `type`, `camera`, `move`, `layers`, `grid`, `tag`, `square`, `droplet`. A name outside it draws the generic shape. |
| `form` | string, optional | What a style of this type is made of beside its description, and so how its card and editor are drawn: `picture` (the default - reference pictures, and a palette while there are none), `lettering` (a live sample of the typeface), `dressing` (a description with `{slots}`), `colour` (a ground: one colour, or a gradient of up to four), `accent` (a colour the cover leans on, one or a gradient - offered as the cover's accent, never as its ground). The application knows the forms, never the types. |
| `families` | list, optional | `{ "key", "label" }` pairs a style of this type is filed under - *Tattoo*, *Classic* for an image style. The dictionary narrows by them. |
| `retired` | string or map, optional | Set when styles of this type are no longer made: what does their work now, in a sentence the dictionary shows above them. They still read; a new one, or moving one into the type, is refused. |
| `canon_kind` | string, optional | A kind of card of the canon that stands in for this type: a hero with a card is described from its facts and needs no style. The dictionary says so above the type. Must name a kind with sections. |

The `hint` is what makes one dictionary richer than several flat ones.
Given the same photograph, `image-style` asks for the render technique and
`character` asks for the person, because the type says which question is
being answered. A type with no hint tells the assistant only its label.

The order of the list is the order the dictionary reads in — the groups on
the screen, and the chips above them — rather than the alphabet.

A workspace that already exists gains the shipped types at the next start;
one you renamed or added stays yours, matched by key, and a hint, a glyph, a
form, a retirement or a stand-in is filled in only where your stored copy
names none. Types and families arrive by key, each after the one it follows
in the shipped list: a family you relabelled keeps your word, and one you
added stays where you put it.

## `style_set`

Not part of the document: a top-level list of a **shipped** profile file,
beside `config` - the starter set of the style dictionary (ADR 0048). It is
seeded into the workspace's dictionary at every start; the bricks are then
yours. A profile you made yourself has none.

```jsonc
{
  "key": "music",
  "config": { /* the document */ },
  "style_set": [
    {
      "key": "woodcut",
      "type": "image-style",
      "label": { "en": "Woodcut", "ru": "Ксилография" },
      "family": "classic",
      "description": "STYLE: medieval woodcut and linocut relief print ... [ The one accent colour is {accent}.]",
      "when": "Raw, earthy or old-sounding songs; cream and kraft grounds.",
      "colours": ["#121114", "#E8DCC4", "#B8563A"]
    }
  ]
}
```

| Field | Meaning |
| --- | --- |
| `key` | Unique in the set. With the profile's key it makes the style's id in every workspace, so a style you deleted is known and stays deleted. |
| `type` | A key of `style_types`, not a retired one. |
| `label` | The name per language. The English one is the style's name. |
| `family` | A key of its type's `families`, where the type files any. |
| `description` | What goes into a prompt, in English. A dressing's slots are written inside `[brackets]`. |
| `when` | When to take it, in English - read by whoever picks styles for a picture (`{style_library}`). |
| `colours` | `#RRGGBB`: a background's or an accent's one, or the stops of its gradient in order (up to four); an image style's palette; the ground of a lettering sample. |
| `sample` | CSS declarations for the live sample of a lettering style. |

A style nobody touched since it was seeded takes a newer wording at the next
start; one you changed keeps yours and offers **Restore as in the set**.

Studio ships nine: `image-style`, `character`, `look`, `environment`,
`typography`, `angle`, `pose`, `layering` and `composition`.

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
  "icon": "spell-check",
  "template": "Here are the lyrics of a song called \"{title}\".\n\n{role:lyrics}\n\nBe specific and be hard on it: which lines are weak, which images are worn out, what would you cut? Do not rewrite it — say what is wrong."
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Identifies the prompt. Also what kilna recognises a running action by, so an action started from a card cannot be started twice at once. |
| `label` | string | Button text, in the AI panel and on a work's Overview tab. |
| `description` | string, optional | What the action does, in a sentence. Shown when the button is hovered, so the button itself can stay short. |
| `icon` | string, optional | The glyph on the button, from the list below. A name kilna does not know draws the generic spark. |
| `template` | string | The message sent to Claude, with placeholders filled per work. Keep it short: the method carries the how. |
| `method` | string, optional | How the action is done — the role the assistant takes, what it checks and in what order, the shape of the answer, what it must never say. Markdown; appended to the model's system prompt on every turn of the chat the action opened. See [ADR 0021](https://github.com/lacodda/kilna/blob/main/docs/adr/0021-an-action-carries-its-method.md). |
| `produces` | string, optional | What the action asks for beyond prose: `"score"`; `"version:<role>"` — the whole answer offered as a version in that role; `"scenes"` — a storyboard to replace the board, or `"scenes:add"` and `"scenes:revise"`; `"comment"` — a comment read off a screenshot; `"reply"` — the whole answer offered as the reply to a comment; `"canon"` — cards, facts and relations for the [canon](/kilna/guides/the-canon/); `"card-prompt"` — the whole answer offered as a card's description for a picture generator, only in an action with `"scope": "canon"`; `"release"` — what a release goes out under, field by field, only in an action with `"scope": "release"`; `"cover-ideas"` — ideas for a publication's cover, each a concept, only in an action with `"scope": "cover"`. Anything else loads as prose and is refused when the profile is saved. |
| `kinds` | list of strings, optional | The work kinds the action is offered on. Absent or empty is every kind. An action that reads `{role:lyrics}` is for the kinds that have lyrics — Studio's song actions say `["song"]` — because a button for it on a video would send a prompt with a hole in it. |
| `scope` | string, optional | `"scene"` for an action started from a row of the storyboard: it reads the row as `{scene}`, is offered on each scene rather than above the board, and must produce `scenes:revise`. `"style"` for one about a brick of the [style dictionary](/kilna/guides/styles/): it is offered on the dictionary and on neither bar of a card, and aimed at a work it is refused by name. `"comment"` for one about a [comment](/kilna/guides/comments/): it must produce `comment` (read a pasted screenshot) or `reply` (draft the answer), and is offered on the comments only. `"canon"` for one about a card of the [canon](/kilna/guides/the-canon/): it is given the card whole ahead of its template — the facts as the work reads them and the card's free note, or for `card-prompt` the settled public facts it is described from and its reference pictures — reads no placeholders, is offered on the card and nowhere else, and must produce `canon` or `card-prompt`. `"selection"` for one about lines selected in a work's text: it reads them as `{selection}`, is offered on a selection and nowhere else, and started without one it is refused. `"release"` for one about one release: it reads `{release}` and `{releases}`, is offered on the release (and started by **Make…** on the one it plans), must produce `release`, and may only name kinds that go out somewhere. `"cover"` for one about a publication's [board of ideas](/kilna/guides/the-cover/#the-board-of-ideas): it reads `{ideas}` and `{choices}`, is offered on the board (and started by **Make…**), must produce `cover-ideas`, and may only name kinds with a cover. Absent is the work. |

**Keep the label to a word or two.** The button carries a glyph and that label;
what the action does belongs in `description`, which is the tooltip. A row of
five actions spelled out in full — *Critique the lyrics*, *Suggest a revision*,
*Draft a style prompt* — is five sentences where the eye wants five marks, and
buttons like that are neither read nor remembered.

The names `icon` accepts: `sparkles`, `wand`, `pen`, `spell-check`, `scroll`,
`tags`, `gauge`, `music`, `film`, `clapperboard`, `image`, `list`, `lightbulb`,
`palette`, `eye`, `reply`, `book`, `quote`, `orbit`.

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

An action with `"scope": "comment"` and `"produces": "comment"` reads a pasted
screenshot. kilna attaches the picture, says which channel it was pasted under
and what day it is, and asks for a json block with the author, the text word for
word, the day it was written and the title it was written under. The channel is
never taken from the answer: the picture cannot say which channel it is. One with
`"produces": "reply"` drafts the answer to a comment: kilna gives the comment,
the work it is under with the opening of its text, and the replies already
posted on the same channel as the voice to write in; the whole answer is the
reply. Both come back as proposals kept on the comments screen, with every field
open to correction first. Every shipped profile carries `read-comment` and
`reply-to-comment`; see [Comments](/kilna/guides/comments/).

An action with `"produces": "canon"` asks for a package for the canon: kilna
appends every kind of card with its sections by key, the kinds of relation,
the layers and the states, and the shape of the json block — new cards,
facts on cards old or new, changes to facts already there, relations. The
answer comes back as a proposal with every item under a box to keep it or
leave it out, and with the facts each one would contradict shown beside it
before anything is written. A fact the assistant proposes is a draft until
you settle it. Started on a work, the version it read is every fact's
source; started on a card, the card is where its facts go. Studio ships
`gather-canon` (on a song: gather its facts), `to-canon` (on a selection:
the selected lines as facts) and, on a card, `gather-card` (facts out of
the card's own free note) and `describe-card` (`"produces": "card-prompt"`:
a description in English for a picture generator, written from the settled
public facts of the kind's `describe_from` sections — the answer is kept as
the card's description). See [The canon](/kilna/guides/the-canon/).

An action with `"produces": "words"` answers with a package of words for
the record: meanings and the works they are in, words for the bank, ways of
singing, terms for the register - the same shape an agent's
`propose_words` gives, kept whole or word by word. kilna appends the shape
of the json block. Studio ships `meanings` (on a song): the images and
scenes it shares with the songs that went out, named for the register's
entries where they are one. See
[The guard of repeats](/kilna/guides/the-guard-of-repeats/#meanings).

An action with `"scope": "release"` and `"produces": "release"` writes what a
release goes out under. kilna appends every field of the release's kind with
its word, its shape, its hint and its limit, and asks for a json block of
fields by key - the same shape an agent's `propose_release` gives. Started by
you, its answer **fills the fields still empty** at once; the ones you had
already started wait beside what you wrote as a proposal, taken a field at a
time or all together. Studio ships `release-meta` for the clip, the audio and
the short: it reads the release, what the work was made from, the channel and
the canon a public text may see, and what went out lately, so a new
description does not sign off or ask the way the last ones did.

An action with `"scope": "cover"` and `"produces": "cover-ideas"` proposes
ideas for a publication's cover. kilna appends the block to answer with - a
json list of concepts: `idea` and `scene`, the angle and the headline, the
hero by card id, a layout and its settings, the bricks by id, the accent, the
mark's variant, the captions - and reads it against the workspace: a name it
cannot find is left out of its idea and said. Asked for from the board, the
ideas **land on the board** as the answer comes; nothing about the cover
changes until one is taken into the constructor. Studio ships `cover-ideas`
for the clip, the audio and the short, with a method that holds the plot of a
cover: one moment through one hero, ideas that differ by angle, the mark's
status by the song's meaning, the house styles more often, the channel's bans
never.

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
- `{styles}` — the [styles](/kilna/guides/styles/) picked for this run, each
  under the label of its type and followed by its description, in the order
  they were picked. It is how a prompt is built out of parts rather than
  glued: the assistant is told which sentence is the place and which is the
  person. A style with nothing written yet goes in by name, marked as not
  described. The author's steer never does — it is an instruction about
  writing the description, not part of one. A template that reads `{styles}`
  in a profile naming no style types is refused on save. A description's
  `{slots}` are filled from the captions of the channel's card; a phrase in
  `[brackets]` whose caption is empty drops out whole, and a slot outside
  brackets takes its clause.
- `{style_library}` — every **ready** style of the dictionary under the label
  of its type, each with its family and *when to take it*, retired types left
  out. For an action that picks styles rather than writes with them - the
  cover's idea generator. Refused on save in a profile naming no style types.
- `{canon}` — the canon as a work reads it: every card by name and id, then
  in full — through the work's lens — the cards the work is about: its own
  heroes, the cards on its board and the cards its text names.
- `{canon:cover}` and `{canon:public}` — the same, as a picture and as a
  public text may read it: only public cards are named, only settled public
  facts of the sections given to that lens are read, and the channel's root
  card is read whole as well — its voice, marks and bans are about every
  work. A release field's template may read the canon only as
  `{canon:public}`: a release goes out in public, and `{canon}` would carry
  the internal layer into its description. That is refused when the profile
  is saved, as `{selection}` in a release field is.
- `{selection}` — the lines selected in the text, word for word. Only in an
  action with `"scope": "selection"`, which must read it.
- `{register}` — the [register of repeats](/kilna/guides/the-register/):
  wording grouped by strictness, then the spent images, scenes and devices,
  each with how many works carry it now and its note, and what the text the
  action reads already takes from it word for word.
- `{neighbours}` — the few works whose words stand closest to the text the
  action reads — shared words weighed by how rare each is across the works —
  each with its title, kind, the words it shares and its current text. Where
  to look for the same thing said twice, not yet a judgement of meaning.
- `{fields}` — the work's own filled overview fields, one per line under
  their words; a choice reads as its option's word.
- `{source}` — what the work was made from, whole: its title and kind, its
  fields, and the text of each role that is the work itself (the lyrics, not
  the critique of them). A work made from nothing says so rather than
  refusing, which is the difference from `{donor}`.
- `{release}` and `{releases}` — only in an action with `"scope": "release"`:
  the release it is about (where it goes out, when, and every field with what
  is written there), and what went out lately - the latest releases with their
  fields, newest first.
- `{ideas}` and `{choices}` — only in an action with `"scope": "cover"`: what
  the board asks for (how many ideas, the person's own to work out, more in
  the direction of the shortlist) with what already stands on it - the
  shortlist, the turned-down ideas as what is not wanted, the rest - the cover
  as it stands, the neighbouring publications' covers and, for a short, the
  song it comes from; and everything an idea is built from, each with the id
  to name it by - the layouts and settings of the frame, the bricks of the
  style dictionary by place with when to use them and the channel's house
  styles marked, the palette, the variants of the mark with what each means,
  the heroes of the canon, and what the channel bans in a picture.
- `{origin}` — the title of what the work is all made from: the song, for
  its clip, its audio and a short cut from the clip; the work's own title
  when it is made from nothing. What a release's title reads: a
  publication's own name (*Tide (audio)*) is kilna's word for it, not the
  audience's.
- `{released}` — the songs that went out, newest first, each with its day and
  its words, the song the action is about left out: what the `meanings`
  action reads a song against.
- `{words}` — the fresh words of your [bank of words](/kilna/guides/the-bank-of-words/),
  by block, each with how it is sung; `{words:space}` — one block's, by its
  name.
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
