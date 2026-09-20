# 31. A style is a brick of the workspace dictionary

Date: 2026-09-20

## Status

Accepted.

## Context

kilna knows two things called "style" and neither is a dictionary.

One is a **version in the role `style`**: a finished production prompt, written
on one work, kept beside its lyrics, marked `counts_as_version: false` because
it is written *about* the song rather than being a draft of it. The other is a
**prompt action** keyed `style` that asks the assistant for such a paragraph.
Both are about one work at a time.

What is missing is the vocabulary those prompts are built from. A picture
prompt is not one idea; it is a render recipe, plus a character, plus an
environment, plus typography, plus a camera angle — each of which outlives the
picture it was first written for. The same character stands in thirty videos.

The workspace this product grew out of learned that the hard way. It carried
three and a half parallel vocabularies for the same thing: six `render_styles`
rows read from vault files; twelve `characters` with no screen; free-text
fields on each song (`direction`, `mood`, `tempo`, `vocal`, `perspective`,
`instruments`); and, latest and richest, a typed brick table that reached
ninety-four rows. An API that read the oldest of them offered the wizard six
frozen styles against ninety-four live ones, and nobody noticed for months.

Six of those free-text fields are in kilna's shipped Studio profile today,
as `work_meta_fields`. They are that same flat vocabulary, one work at a time.

## Decision

**A style brick is its own row, owned by the workspace; its type is a word of
the craft, and so lives in the profile document.**

Three parts:

1. **`style_brick`** (migration 0026) — id, profile, `type_key`, name,
   `description`, `hint`, `status`, timestamps. It hangs on the profile, not on
   a work: a brick is picked by any prompt that wants it, and edited in one
   place. `status` is `draft` until a description exists, then `ready`, and
   `dropped` when the owner retires it without losing what it was. Only `ready`
   bricks are offered for building a prompt.

2. **`style_types` on the profile document** — `key`, `label`, `hint`, and an
   optional `icon`. It sits at profile level rather than on `WorkKind` beside
   `shot_types` and `scene_blocks`, because a brick is not judged, shipped or
   storyboarded: one character serves the videos and the shorts and the covers.
   A dictionary per kind would be the same character written twice.

   The `hint` is what makes one dictionary richer than several flat ones. It
   says what to describe when a brick is of that type — the render technique
   for `image-style`, the person for `character`, and never the other way — so
   the same photograph yields a different description under a different type.
   It is the craft's instruction, which is why it is a profile field and not a
   constant: `SceneBlock` already has this shape (`key`, `label`, `hint`).

3. **References are assets.** `asset.style_brick_id` joins the columns that
   already point at a work and a release. A picture arrives the one way
   pictures arrive here (ADR 0027) — copied into `media/`, named by its id, its
   arrival name kept.

`type_key` is a string, not a foreign key, for the reason scores keep their
axis keys as strings (migration 0012): the vocabulary is a document, and a
brick written under a type later dropped from that document still says what it
was.

The version role `style` stays, unchanged and undeduplicated. A brick is a
part; a version in role `style` is the finished prompt for one work. The
constructor turns the first into the second.

## Consequences

- The dictionary is one, and it is the rich one. A new type is a line in the
  profile document, edited where every other word of the craft is edited.
- `carry_forward` must name `style_types` explicitly — whole-list-if-empty, the
  form `shot_types` and `cover_blocks` already take — or the dictionary reaches
  no workspace that exists today.
- A brick's description is text a person vetted, and it enters a prompt
  verbatim. `hint` never does: it is an instruction *about* the description,
  and a prompt that carried it would be asking the generator to take notes.
- Bricks are shared across kinds, so nothing about them is per-kind. If a craft
  ever needs a type only one kind may use, that is a filter on the type, not a
  second dictionary.
