---
title: MCP server
description: kilna --mcp serves your workspace to an agent over the Model Context Protocol — reading is open, writing is a proposal you apply with one click, a whole work at a time if you like.
---

`kilna --mcp` is the same build without a window: the workspace served to an
agent — Claude Code, or anything else that speaks the
[Model Context Protocol](https://modelcontextprotocol.io) — over standard
input and output. No port, no key, no network: the client starts the process
and talks to it through a pipe, and the process stops when the client hangs
up.

## Registering it

The **Settings** screen shows the exact command for the build you are
running, with a copy button. For Claude Code it is:

```
claude mcp add -s user kilna -- "C:\path\to\kilna.exe" --mcp
```

Quote the path; application directories have spaces in them. `-s user` makes
it every session on the machine rather than the one project you ran the
command in, which is Claude Code's default scope. After that a
Claude Code session anywhere on the machine can read your works and propose
to them — no need to have kilna open, though it is fine if it is.

Every tool answers from the [space](/kilna/guides/spaces/) kilna last opened -
looked up at every call, so the server follows the window - or from the one its
`space` argument names, as `workspace` lists them. A space another machine
holds open is served for reading only, and a proposal there is refused.

**The methods live in kilna.** `workspace` gives each kind's axes whole -
their rubrics, options and descriptions - and `guides`, the index of the
profile's skills without their methods; `guide` opens one. An agent is told
so in the server's instructions, and is meant to read them each time rather
than keep a copy: a method you rewrite in **Settings → Skills** is the one
the next agent reads.

**A trigger for your agent.** **Settings → Agents** writes a short file in
the agent's own place - Claude Code's `~/.claude/skills/kilna/SKILL.md`,
Codex CLI's `~/.codex/prompts/kilna.md`, Gemini CLI's
`~/.gemini/commands/kilna.toml`, or a Cursor rule saved into a project as
`.cursor/rules/kilna.mdc` - that sends the agent to this server whenever you
talk about your works. It holds no method, no axis and no rule of your world;
it is shown whole before it is written, and a file already there is replaced
only when you say so.

`--workspace <dir>` points the server at another application directory - a
test's, or a copy kept for a rehearsal. Without it, it opens the one the window
uses.

## What an agent can read

| Tool | What it answers |
| --- | --- |
| `workspace` | The space it answers from - `space`: its name, its kind, the other spaces `space` may name, and the machine that holds it when another does - and its profile's vocabulary: the overview fields (each with the kinds it belongs to, for a choice its options, and for a field kilna numbers itself `numbered_from` - leave it out of a proposed work, and the work is given the next code), and per kind of work its version roles and how each reads, axes with weights and scales, tiers, statuses, kinds of release (each with the fields a release of it goes out under and the shape of its cover), whether it goes out at all (`goes_out` — a song does not: its clip, audio and shorts do), for a kind with a storyboard its kinds of shot, its layers and the prompt blocks a scene carries, whether its works go out under a cover built in the constructor (`cover`), whether they play under a frame, whether they are experiments (`lab`: the role a kept trial goes into, and the field the anchors are read from), and whether they are prepared on a conception (`conception`: the role of the brief, the field of the idea, the roles sent with the order, the boards); `boards`, each with its rule and its kinds of card with their fields - each field's `shape` (`quote`, `proof`, `chips`) and, for a kind that links to the library, `library`; how many works; `note_kinds`, each plain kind of note with its `key`, `label`, `line` (one line - a phrase) and `material` (spent by works) - and the library's with `library` and the `media` an entry is in, a medium not `quoted` never quoted - while the cards of the canon stay with `canon`; the style types of the dictionary (`style_types`: key, label, `form`, families) and the compositions (`compose`) - a role written out of the dictionary, such as a song's style prompt, with its blocks, how many each takes and its rule, and the fields that close it. Read first — a work is judged in its own kind's keys, and every other tool speaks in them. |
| `catalogue` | Every work with its verdict: id, title, kind, status, total and tier, whether the score is stale, releases out and scheduled, when it was last touched, and `repeats` - the [guard's](/kilna/guides/the-guard-of-repeats/) mark: its level (orange or red), how many findings stand, and the loudest one's word, the song it is shared with and that song's day; a publication wears its song's. Filter by a substring of the title, a kind, a status. The answer is a page - `works` with `total`, `limit` (100 unless given, 500 at most), `offset` and `has_more`; `summary: true` gives id, title, kind, status and when it was last touched alone. |
| `work` | One card: fields and meta, tags, every version by role (id, revision, label, length, which is current, and `parent` — the id of the version it was written from, when that is known), the latest score with its axes, the releases, how many notes and scenes, the cover (`prompt` - the picture, the negative and, when the title goes apart, the typography, written from the cover's concept for its first shape - with the `format`, the lettered `title` and the `concept` itself), the frame (for a kind that plays under one: the still - built from the cover or written whole, `built_from_cover` says which - the loop as written from its settings, the negative), what it was made from (`sources`, each saying whether the source has moved on since) and what was made from it (`derived`); for a work that never goes out itself — a song — its `publications`: everything made from it, with where each stands, and the release its status stands on; for an experiment, `trials` - how many are on its board, kept, dropped and not judged. No bodies. Since v0.138 it says `work_plan` (`{id, scheduled_at, note}`) when the work is planned on a day. |
| `text` | The body of a version: the current one of a role, or a revision by id. Plain roles come back exactly as typed. |
| `scores` | The score history of a work, newest first. |
| `calendar` | `releases`: every release with a date, in calendar order, whether its date is kept; `closed`: the time off, spans of days nothing should go out on, with their names. `from` starts at a day. Since v0.138 also `work_plans`: the open planned slots, each `{id, work_id, title, kind, stage, scheduled_at, note, derivative_count}`. Since v0.151 `cross_spaces` - `true` for every other space, or a list of their names - adds `cross_spaces`: on the days this space puts something out or plans it, what the neighbours do, each `{space_name, date, title, cover_idea, genre, mood}` and nothing more, read from their databases for reading only. |
| `propose_batch_prompts` | A month's drafts in one pass (v0.153): `works`, each `{work, title?, fields?, versions?: [{role, body}], bank?: [item names]}`, and `collection` or `month` naming the month the checklist reads. One proposal into the chat named after the agent; the person sees what the space's quotas find over the month as it would stand and applies all of it at once - every work renamed and filled, its texts kept, the bank's items it takes marked used on its planned day - one undo. A field the profile does not have and a bank item it does not keep are named and left out; an unknown work or role refuses the proposal. |
| `quotas` | The space's [rules](/kilna/guides/the-canon/#quotas-and-banks) checked (v0.151): `work` for one work, `collection` (id or exact title) for a collection's works together, `month` (`YYYY-MM`) for the works going out or planned that month, nothing for the whole space. Returns `rules` (how many were asked), `works` (how many were measured) and `findings`, each `{rule_id, label, field, metric, severity, scope, work_id, title, measured, value, expected}` - `work_id` is null for a finding about the works together. |
| `notes` | Notes, all of them or one work's. Arguments: `work`, `query` (title, body or whose words), `kind`, `state` (fresh, used, parked or dropped, for a material kind) and `shelf` (only the phrases on one shelf, by id or name). A phrase comes with `attribution`, `shelves` (names), `sung_in` (how many works say it now) and `sung` (how it is sung, when the record says so). With no arguments it returns `{ shelves: [{id, name, phrases}], notes: [...] }`. A page of `notes` with `total`, `limit` (50 unless given), `offset` and `has_more`; `summary: true` leaves the bodies out - id, title or first words, kind, state, tags, the body's length, when it was last touched; `note` reads one note whole, by id or exact title. |
| `register` | The register of repeats: every word of the record with its forms, kind, strictness (null for a word that is not spent), how it is sung, topic, note and how many works carry it now; or, with `text` or `work`, that text checked — the terms it takes word for word and how often, and the words it leans on within itself — and, with `work`, `guard`: the songs out or booked its song shares a spent term or a rare word with, how loudly, and the words kept on purpose. The record is a page of `terms` with `total`, `limit` (100 unless given), `offset` and `has_more`; `summary: true` gives id, word, kind, strictness and uses alone; `term` reads one word whole, by id or the word itself, with the `works` that carry it. Since v0.151 a term says `scope` - where it is looked for: `lyrics`, `title`, `visual_motif`, `sound_combo` or `all` - and `max_per_month` and `cooldown_days`; a guard finding found outside the lyric says its `scope`. |
| `styles` | The [style dictionary](/kilna/guides/styles/): with `type`, `composition` (its types in its order) or `query` (a word of the name, the phrase or the explanation, in any language), the bricks - id, type, name, family, `phrase` (what the generator reads word for word), `explanation` per language, `when`, whether it is a `house` brick of the channel, where it came from and its status. Without any of them, each type with its form and how many bricks it has, and the compositions. See [The dictionary of sound](/kilna/guides/the-sound/). |
| `scenes` | The storyboard of a work, in order: each scene's number, section, seconds, kind of shot, layer, description and prompt blocks. The shared context is the `context` role — read it with `text`. |
| `canon` | The [canon](/kilna/guides/the-canon/): without arguments, every card — id, kind, name, aliases, layer, the work it lives at, how many facts, drafts and live zones — and the kinds of card with their sections. With `card` (an id, or an exact name or alias), that card whole: its facts by section, each with its layer, status, source, time in the world and the tasks that may read it; its relations; where it appears; its description for a picture generator and whether it is stale. `lens` — `cover`, `work` or `public` — narrows the card to what that task may read, by the same rule the window dims by; a card a lens cannot see answers that it does not exist. `query` searches the facts; `timeline` lists the facts dated in the world, earliest first. Since v0.144 the list also says whether the space has its passport (`has_root_card`, `root_card_id`) and `section_schemas` - for each shape of section (`facts`, `slots`, `details`, `palette`, `marks`, `styles`, `bans`, `branding`, and since v0.151 `quotas` and `bank`) what an entry's words are and the `data` keys it carries. A bank's items come to a prompt only while they are free - an item resting after a use, or locked, is left out. `card: "root"` (or `"project"`) is the passport. A long card is read in parts: `summary: true` answers its head, its summary and per section how many facts and drafts it holds, no facts; `section` and `status` narrow the facts; `limit` and `offset` page them, with `total_facts` and `has_more`. |
| `cover` | The board of ideas for a publication's cover - a clip, an audio release, a short: `cover`, the concept as it stands; `ideas`, each with where it came from (`own`, `refined`, `ai`, `sibling`), the person's verdict (`star`, `rejected` or none), its angle, headline and concept; `siblings`, the covers of the other publications of the same song fitted to this one's shape; and `choices`, the text an idea is built from - the layouts and the five settings of the frame, the bricks of the style dictionary with their ids and when to use them, the channel's palette, the variants of its mark with what each means, the heroes of the canon, what a picture of the channel never shows. A work without a cover is refused. |
| `trials` | The board of an [experiment](/kilna/guides/the-lab/): `anchors` - what every trial keeps; `harvest_role` and `harvest_kinds` - where a kept trial goes; `series` in the order each began; and `trials`, each with its id, series, `parent` (the trial it varies), `angle`, `body`, the `bricks` it was picked from, `reference`, `outcome`, `verdict` (`keep`, `drop` or null), `run_first`, `source` (the song's style it reworks), the anchors it has lost, how many takes it has, and `harvest` - the versions and bricks it went into. Plus `choices`: the phrases of the dictionary a trial is written from, block by block, with their ids, what each means and when to take it. A work that is no experiment is refused. The trials are a page (`total`, `limit` - 50 unless given - `offset`, `has_more`); `summary: true` leaves the texts and the dictionary's `choices` out; `trial` reads one trial whole by its id. |
| `brief` | The order a writer works from, for a work prepared on a [conception](/kilna/guides/the-conception/): `order`, the whole order in markdown - the same text **Copy the order** copies: the idea, the brief, the texts it keeps to (a song's style prompt), the fields, each board as its rule reads it, the notes and phrases attached with how each is sung, the channel's voice and the canon the work is about, the register - or null while nothing of the conception is written; `boards`, each card with its id, kind, topic, where it came from (`own`, `refined`, `ai`), the verdict (`star`, `rejected` or none), the `entry` of the library it links to and its fields, the directions the last run offered, and `elsewhere` - the other works each entry already worked in. In `order` a quote stands word for word only with the link it was checked at; `brief_role` and `idea_field`. Write from `order` and answer with `propose_version` or `propose_work`. A kind prepared on no conception is refused. |
| `search` | Works, versions, notes, facts and replies by text; every hit names its work or its card. |
| `guide` | The skills of the profile (v0.127): without `skill`, the catalogue - key, name, description, `when`, group, kinds, scope, what it answers with, whether it is a button - narrowed by `kind` and `scope`; with `skill`, the whole method, the message its button sends, which `propose_*` tool its answer goes through (`answer_with`), and `workspace_context` - each kind's axes with their rubrics, tiers, roles, statuses and doors, the boards and the style types, read live. |

A work is named by id, or by its exact title. A title two works share is
refused with the ids to choose from, rather than guessed.

**What a work feels like** (since v0.124): `workspace` lists the profile's
`vibe` - its moods, characters and complexities with their keys; `work`
answers its own `vibe` and its `shown_vibe` - its own, or the one of what it
was made from, with `from` naming that work; `catalogue` answers each row's
`vibe` and its `stage`, and narrows by `mood`, `character` and `complexity`.

## What an agent can propose

Nothing an agent does writes a work, a version, a score, a note or a
scene. It **proposes**, and the proposal lands as a message in a chat named after the
client — *Claude Code*, say — on the work, or on nothing for a work that
does not exist yet. Where kilna writes the message out itself - a package, a
board, trials, cover ideas, cards, facts for the canon - it is stored in
English, and a window reads it in its own language: the headings and the
profile's words are said again from the proposal, and what the agent wrote
stays as written. The same buttons that apply the assistant's own
proposals apply these:

| Tool | Where it lands |
| --- | --- |
| `propose_work` | A whole work — title, kind, overview fields, versions by role, a score, notes, for a kind with a storyboard its scenes, for an experiment its `trials` (in the shape `propose_trials` takes), cards for the boards of its conception (`boards`: each a `board` and its `cards` in the shape `propose_cards` takes - a new song with its research), and the releases it ships as with the text each goes out under — or, with `work`, a package of those for an existing one. A version of a package for an existing work may name `from`, as `propose_version` does. A note of the package may name its `kind` - a phrase or a note, as `workspace` lists `note_kinds` - and a phrase whose words it is, `attribution`. The message shows everything the package would write; **Create the work** or **Apply the package** writes all of it in one click. On a new work the version in the first role becomes current; on an existing one the package's versions wait beside the current, and its scenes go after the last on the board. A `releases` entry names its kind of release, optionally the day it goes out, and `fields` — what it goes out as, by the field keys that kind names. A kind of release the work does not ship refuses the whole package; a field key that kind does not have is named and left out. |
| `propose_scenes` | A storyboard for a video or a short: scenes with their number, section, seconds, kind of shot, layer, description and prompt blocks, in the kind's own words. The message shows the board as a table with every block under it. `change` says what happens to the board that is there: `add` (the default) — **Add to the board** numbers the scenes after the last; `replace` — **Replace the board** rewrites a scene with the same number in place — it keeps its id — sends the rest of the old board to the trash, and creates the numbers nobody held; `revise` — **Revise the scenes** changes only the numbered scenes, only in the fields given (a field left out is kept, `blocks` are set together), and sends nothing to the trash. A kind with no storyboard, an unknown kind of shot, layer or block key, a scene that ends before it starts, a revision without numbers refuse the whole package. |
| `propose_version` | The text of a new version in a role, with a note on what changed. **Insert as version** keeps it, verbatim, under that role and not current; **Choose role…** opens the dialog to change the role, name it or make it current on the way in. `from` names the version of the same role the text rewrites, by the id `work` lists: the version it becomes remembers it as the one it was written from, and the history draws it so. A version of another work or role is refused. Without `from`, a proposal in a chat started on a version, in that version's role, is written from it. For a role a composition writes - a song's style - give `bricks` instead of `body`: the phrases by id or exact name, in the order they are written; kilna writes the text from them as its own constructor does, the work's fields closing it, and says a block picked outside its bounds or a text past its limit. Both at once is refused. |
| `propose_score` | Marks along the kind's axes, checked the way the assistant's own are: unknown axes are named, marks are clamped to the scale. **Apply** writes the snapshot, judged by the agent — its name is the score's rater. |
| `propose_note` | A note, on a work or on nothing in particular. Optional `kind` (the profile's kind of note: a phrase or a note; the plain note when left out; a card kind is refused), `attribution` (whose words) and `shelf` (made when there is none). **Add as note** keeps it. Since v0.116: `tags`, a `layer` (public, internal, inWorks); `note` - the id or exact title of a note that is there - proposes a **change** to it rather than a second note (give only what changes; the person reads was against becomes); `notes: [...]` proposes several at once, kept whole or note by note. Since v0.145 `origin_work` (a work's id or exact title) says the work a new note or phrase arose at: it stays fresh and remembers it, apart from the work it is spent in. |
| `propose_release` | What one release goes out under — its title, description, tags, the comment pinned under it — by the field keys its kind of release names. A publication goes out once, so `work` names the release. A field key the kind does not have is named and left out; a lyric quoted with the singer's marks is cleaned. The fields wait in the chat on the work and in the **Release** block of its overview, each beside what is written there now; the person takes them one by one or all at once. Nothing is written until they do — unlike the window's own **Release meta** action, which fills the empty fields as its answer lands, an agent outside the window only proposes. A song has no release: propose for its clip, audio or short. |
| `propose_cover` | Ideas for a publication's cover, each a concept: `idea` (in the language of the work) and `scene` (in English) are required; `avoid` (what the scene keeps out beyond the channel's bans, in English - it goes into the negative with the scene), `angle`, `headline`, `hero` (a card id), `layout` with optional `column`, `row`, `size`, `crop` and `place`, the bricks `style`, `typography`, `dressing`, `background` (ids), `accent` (a colour or a name of the palette), `mark` (a variant id), `captions` by slot. Every name is looked up by id, then by name; one that is not in the workspace is left out of its idea and named in the answer, and the rest lands. The ideas wait in the chat on the work and above its board of ideas; **Put on the board** puts them there, or untick an idea to leave it out. Nothing is written until the person does - the window's own **Cover ideas** action puts its ideas on the board as they come. |
| `propose_trials` | Trials for an [experiment's](/kilna/guides/the-lab/) board: each its `body` (required), `angle`, `series`, `bricks` (ids or exact phrases of the dictionary), `reference` (who to listen to - never in the body), and optionally `parent` - the id of a trial on the board, or the number, from 1, of an earlier trial in the same list, which then lands as its child - `source` (the id of a song's style version it reworks), `outcome` and `verdict` (`keep` or `drop`) for a trial already heard, and `run_first`. A brick, parent or version not in the workspace is left out and named in the answer; the rest lands. The trials wait in the chat on the work and above its board; **Put on the board** puts them there. Nothing is written until the person does - the window's own **Propose trials** puts its trials on the board as they come. |
| `propose_cards` | Cards for a board of a work's [conception](/kilna/guides/the-conception/), by its key (`board`: `research`, `scenarios`): each names its `kind` - which may be left out on a board of one kind - its `fields` by the keys of that kind, and optionally a `topic`. A kind marked `library` names its work of culture under `entry` - `title` (required), `author`, `year`, `medium`, `original` - and links to the library's entry of that name, written into the library when there is none; a `proof` that is not a link, and every quote of a medium that is not quoted, are left out and named. `starred: true` carries over a choice the person already made; `directions` - two to four places the next run could dig. A kind or field the board does not have is left out and named in the answer; the rest lands. The cards wait in the chat on the work and above the board; **Put on the board** puts them there. Nothing is written until the person does - the window's own **Propose scenarios** puts its cards on the board as they come. |
| `propose_words` | Words for the record, kept in one click: to keep as a phrase (`phrase: true`, and a `shelf` by name, made when there is none), how a word is sung (`sung`: `[{written, sung}]`), a term for the register (`strictness`), the works a meaning is in (`works`: ids or exact titles). A plain string is a phrase to keep. A word kept only as a phrase writes no term; a word with `sung`, `strictness`, forms or `works` also writes or updates the term, and one the record already keeps gains what is proposed and loses nothing. A phrase already kept in the same words is not kept twice; a work that cannot be found is named and left out. The message lists every word under a box; keep all of them or some. Since v0.116 a word the record already keeps is **changed** by what is proposed - `strictness` replaces the one it has (`null` unspends it), `forms` join (or replace, with `replace_forms: true`), `kind`, `topic` and `note` are written over - and the proposal says each change, was and becomes; a field the register would not take when it is kept is said, never dropped in silence. `action: "delete"` takes a word out of the record, into the trash; `action: "split"` with `into: [...]` makes a word that is several into them, each with what the one held and the works named for it. A new `word` written with a comma or a slash is warned about: one word is one thing a text can say. |
| `propose_style` | Styles for the dictionary, kept in one click or one by one (v0.134): new bricks of any type (`type` and `name`) and rewrites of bricks it has (`id` from `styles`; a rewrite changes only the fields it names) - `description`, `family`, `when`, `explanation` per language, `colours`, and a `contract` of `needs`, `provides` and `conflicts`. An item of an unknown type, an unknown id or a new one with no name is left out and said. Each kept item is the dictionary's own new style or edit, undone like any other. |
| `propose_canon` | Cards, facts and relations for the [canon](/kilna/guides/the-canon/). A fact is an `add` to a card (by id, name, or the `handle` of a card proposed in the same call), a `refine` of a fact's id, or a `retire` with its reason; one that contradicts the canon says which fact in `contradicts`. `work` is where the facts were read from: every fact with no other source cites it, with the line in `line`, and a card with `on_work: true` lives at it. The message lists every item under a box, with the facts it contradicts beside it; **Keep all** writes the package, or untick what you do not want and keep the rest. A fact kept from a proposal is a draft — the works read it, a cover and a public text do not — until you settle it; one proposed as `open` stays a live zone. An item the canon has no place for — a kind or a section it does not have, a card it cannot find — is named in the answer and left out. `pictures` attach files on this machine (`path`, absolute; JPEG, PNG, WebP, GIF or AVIF) to a `card`, or to one `fact` of it such as an outfit, with a `role` — portrait, reference, outfit, mood, still or mark, reference when absent; a file is copied into the workspace only when the picture is kept. `descriptions` write what a generator is told about an existing `card`, in English, from its appearance facts; kept, one replaces the card's description and is fingerprinted against its facts as they stand. Since v0.144: `card_updates` change a card of the canon - `title`, `aliases` (the whole list), `layer`, `note` (its summary), `on_work` - named by id, name, alias, or `root` for the passport, and shown beside the card's name as it stands; a second passport proposed in `cards` becomes an update of the one standing, and the facts naming its handle land on it. A fact can be `move`d to another `section` of its card or `delete`d (to the trash, an undo away), and a `refine` with a `section` files it there. A new fact a card nearly holds already - the same words, or four in five of its longer words in any order - comes back marked `similar_to` (the fact's id, words and section), in the answer and beside the item. A relation with `action: "delete"` takes the relation between the two cards away. |

`propose_work` also takes a `vibe` - `{"mood", "character", "complexity"}`,
any of them left out - in the profile's keys; a key it does not have is
refused with the ones it has. A vibe alone is a package worth proposing.
Kept, it is the work's own only where the author has said none: a proposal
never writes over the author's word. A score the assistant writes may carry
a vibe the same way.

An applied proposal stays marked in the chat — *Inserted*, *Scored*,
*Created*, with a link to the work a package made — and cannot be applied
twice. When a chat holds more than one unapplied proposal, **Apply all**
takes them in order, and stops at the first that cannot be applied, saying
which. Every application is written as the same operations you would
write by hand, so undo takes each back on its own.

Each proposal leaves a line in the [history](/kilna/guides/the-history/) —
*Claude Code proposed a version for "Harbour lights"*, *Claude Code
proposed a new work, "Winter road"* — so the bell in the title bar counts
it, and the chat it went into is one click away from any screen through the
assistant's button beside it.

This is the rule the assistant panel has followed since v0.28, applied to an
assistant outside the window: it proposes, you apply. See
[ADR 0016](https://github.com/lacodda/kilna/blob/main/docs/adr/0016-an-agent-outside-the-window-proposes-too.md);
for packages and the mark,
[ADR 0018](https://github.com/lacodda/kilna/blob/main/docs/adr/0018-a-proposal-is-applied-by-the-application-and-marked.md);
for storyboards and what replacing one means,
[ADR 0022](https://github.com/lacodda/kilna/blob/main/docs/adr/0022-a-storyboard-is-proposed-whole-and-replaced-by-number.md).

## A session, end to end

```
> Use kilna: what is the weakest scored song, and what would you change?

  workspace  → seven axes, tiers HOLD / PIC / CLIP …
  catalogue  → 207 works; "Harbour lights" scored 54, tier HOLD
  text       → the current lyrics
  scores     → 54: hook 5, lyrics 6 …

  The chorus repeats its first line; here is a version with a turn in it.

  propose_version → "Proposed a `lyrics` version for “Harbour lights”.
                     It is in the chat on the work, waiting to be inserted — or not."
```

In kilna: the bell shows one new line, the work's chat *Claude Code* holds
the text with **Insert as version** under it. Insert, or don't.

A new song, whole:

```
> Use kilna: make a song from this idea — lyrics, a style prompt, the premise
  on the card, and your score.

  workspace     → kinds song / video / audio / short; fields bpm, key, …, premise
  propose_work  → "Proposed a new work — “Winter road”, 2 versions (lyrics, style),
                   fields premise, a score on 7 axes. It waits in the chat named
                   after you; one click creates it with everything in it."
```

In kilna: the assistant's button in the title bar opens the chat *Claude Code*
with the whole package rendered — the lyrics in a monospace block, the
style prompt, the premise, the marks — and **Create the work** under it.
One click, and the song is on the catalogue with its lyrics current, its
premise on the overview, its score in the history judged by *Claude Code*.

## A video from a song, end to end

The storyboard of a video is rows the agent can propose like anything
else. The whole road, from the song to a board ready for the generators:

```
> Use kilna: make a video for "Winter road" — plot, context, and a storyboard
  with prompts for every scene.

  workspace       → kind video: roles plot / context / style, shot types wide,
                    medium, close, detail, insert, title; blocks still, motion, negative
  work            → "Winter road", song, lyrics current
  text            → the lyrics

  propose_work    → "Proposed a new work — “Winter road — the clip”, 2 versions
                     (plot, context), 6 scenes. It waits in the chat named after you;
                     one click creates it with everything in it."
```

In kilna: **Create the work** — and the video is on the catalogue with its
plot and its context current, and six scenes on the **Scenes** tab, each
with its seconds, its kind of shot and a still-frame prompt to copy. Then
link it to the song on the **Links** tab as *made from*, or ask the agent
to do it in the same breath.

The same package can plan how it ships. An agent that has just written the
board is the one that knows what the description should say, so making it
propose that separately would be a second round trip for one half of one
thought:

```
> Use kilna: …and plan it as a YouTube release on the 2nd, with a title,
  a description and tags.

  propose_work    → "Proposed a new work — “Winter road — the clip”, 2 versions
                     (plot, context), 6 scenes, 1 release (youtube)."
```

Applying it plans the release with its boxes already filled — see [what a
release goes out as](/kilna/guides/planning-a-release/#what-a-release-goes-out-as).

The board is not final on the first pass. A second round works on the
existing work:

```
> Use kilna: the chorus scenes of "Winter road — the clip" are too static —
  redo the board with more close-ups, and time it to the lyrics.

  scenes          → the six scenes as they stand
  text (context)  → the hero, the palette, the lens
  text (plot)     → the plot

  propose_scenes  → "Proposed a storyboard of 8 scenes to replace the 6 on the
                     board of “Winter road — the clip”. It waits in the chat on
                     the work; one click replaces the board."
```

In kilna: the chat on the video shows the new board as a table, every
prompt block under it, and **Replace the board** with a line saying what
that does. One click: scenes 1–6 are rewritten in place, 7 and 8 are
created. Had the new board been shorter, the scenes beyond it would be in
the trash, and undo walks the whole thing back a scene at a time. To grow
the board instead of redoing it, the agent leaves `change` out and the
scenes go after the last; to rewrite the prompts of scenes 3 and 4 and
nothing else, it sends those two with their numbers and `change: revise`.

The plot and the context are versions, so they take the usual road:
`propose_version` in the `plot` or `context` role, **Insert as version**
on the Versions tab. There is no separate tool for them — a video's plot
is a body with revisions like a lyric.

## Protocol

JSON-RPC 2.0, one message per line, protocol version `2024-11-05`. The
server answers `initialize`, `ping`, `tools/list`, `tools/call`,
`prompts/list` and `prompts/get`; a
notification gets no answer, an unknown method gets a `-32601` error, and a
tool that fails answers *inside* the result with `isError` so the agent
reads the reason. Nothing but protocol goes to stdout; diagnostics go to
stderr.

**Prompts** (v0.127): every skill of the profile is a prompt by its key - a
slash command in the clients that offer them - with one optional argument,
`work`. Got with a work, it is the skill's method followed by its message
filled from that work, as a button would send it; without one, or for a
skill that is a method only, the method and how to apply it - which work to
read and which `propose_*` tool to answer through.
