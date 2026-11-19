---
title: Writing a plugin
description: The plugin protocol — --manifest, run with JSON on stdin, what a work is handed, the outcome shape, settings kilna draws, a judge's score, words by language, progress on stderr, and how returned metadata and files are kept.
---

Integrations in kilna are ordinary executables, not a plugin API kilna's
process loads into itself. A plugin is any program named `kilna-plugin-*`
found on your `PATH` or in the workspace's `plugins` directory — written in
whatever language you like, run for the duration of a single call, and never
started as a background service.

[kilna-plugin-wordcount](https://github.com/lacodda/kilna-plugin-wordcount)
is the reference implementation — read it alongside this page if you want a
working example rather than just the shape of the protocol.
[kilna-plugin-cut](https://github.com/lacodda/kilna-plugin-cut) is the one
for everything a long command does: a `cuts` target, its own time allowance,
progress on stderr and the files it made in the reply - see
[Cutting a short](/kilna/guides/cutting-a-short/).

## Naming and discovery

kilna looks for executables prefixed `kilna-plugin-` in the workspace's
`plugins` directory first, then along the system `PATH`. The first copy found
under a given name wins, the same way any other command resolution works. A
plugin that fails to describe itself is still listed — with the reason it
failed — rather than silently absent, because "not appearing" is otherwise
indistinguishable from "not installed."

**Settings → Plugins** is where they stand: each with its version and what it
offers, why it cannot run if it cannot, a switch that turns it off without
uninstalling it, and [its settings](#settings). **Look again** reads the list
after a plugin was dropped in the folder. In the rest of the window, a plugin
that cannot run says only that it cannot, and links here. A plugin has five
seconds to say what it is, so that one that never answers cannot hold the rest.

## Describing yourself: `--manifest`

Run with a single `--manifest` argument, a plugin must print one JSON object
to stdout and exit successfully:

```jsonc
{
  "protocol_version": 1,
  "name": "wordcount",
  "version": "0.1.0",
  "description": "Counts words in a work's bodies.",
  "commands": [
    {
      "key": "count",
      "label": "Count words",
      "description": "Adds a word count to the work's metadata.",
      "target": "work"
    },
    {
      "key": "cut",
      "label": "Cut the short",
      "target": "cuts",
      "timeout": 900
    }
  ]
}
```

`protocol_version` must match the version kilna speaks — currently `1`. A
mismatch doesn't crash anything; the plugin is listed but marked unusable,
with the mismatch shown rather than hidden. Each entry in `commands` becomes a
button wherever its `target` is drawn: `"work"` in the head of a work's
Overview, `"release"` under a release's row in the
[Releases tab](/kilna/guides/planning-a-release/), `"cuts"` on a work's
[Cuts tab](/kilna/guides/shorts/#the-cut-list) - where a cutter belongs - and
`"score"` on a work's [Score tab](/kilna/guides/scoring-a-work/), where a judge
does, and since v0.158 `"cover"` on a work's [Cover tab](/kilna/guides/the-cover/),
where a picture generator does - see
[Generating pictures](#generating-pictures).

`timeout` is how many seconds the command may run - see
[How long a plugin may take](#how-long-a-plugin-may-take). `"batch": true`
(v0.148) says a command is meant for many works at once - a render by a
preset, with no editor - and Settings says so beside it. Since v0.155 such a
command on `"work"` is offered under **Plugins…** in the catalogue's bar of
ticked works and on a collection: see
[Running over many works](#running-over-many-works). Every field added
since the first protocol is optional on both sides, which is why the version
is still `1`.

## Words by language

Every word a manifest shows has a one-language field and, optionally, the same
word by language beside it: `labels` for a `label`, `descriptions` for a
`description`, on the manifest, a command, a setting and a choice's option.

```jsonc
{ "key": "cut", "label": "Cut the short", "labels": { "ru": "Нарезать шортс" }, "target": "cuts" }
```

The window takes its own language's, and falls back to the plain field: a
plugin that says nothing about Russian still has a button in a Russian window.

## Settings

A manifest may declare what can be set for it. kilna draws the form in
**Settings → Plugins**, keeps what is set, and hands every run the values, so a
plugin needs no configuration file and no screen of its own:

```jsonc
{
  "settings": [
    { "key": "channel", "type": "text", "label": "Channel", "default": "" },
    { "key": "crf", "type": "number", "label": "Quality", "min": 0, "max": 51, "step": 1, "default": 20 },
    { "key": "hand", "type": "boolean", "label": "The pointing hand", "default": true },
    { "key": "preset", "type": "choice", "label": "Preset", "default": "fast",
      "options": [{ "value": "fast", "label": "Fast" }, { "value": "slow", "label": "Slow" }] }
  ]
}
```

- `type` is `text`, `number` (with optional `min`, `max` and `step`), `boolean`
  or `choice` (with its `options`). An unknown type is a manifest that cannot be
  read, and the plugin is listed with the reason.
- `default` is what the plugin does when nothing is set. The form shows it, so
  what is on the screen is what a run does; a plugin may compute it - from a
  file of its own, say - each time it is asked for its manifest.
- Only what was changed is kept. A setting left alone keeps following `default`,
  so a default changed in your next release reaches everyone who left it alone.
  **Back to the plugin's own** takes a changed value back.
- A value is held to what the setting declares before it is kept. One kept
  earlier that no longer fits - an option you removed - is not handed over, and
  the default stands instead.

**A secret is marked so.** Settings are kept in the workspace, so they go into
every backup and export. A key, a token or a password is declared with
`"secret": true` (v0.155):

```jsonc
{ "key": "token", "type": "text", "label": "API token", "secret": true }
```

kilna keeps a secret on this machine only, in a file of its own beside the
plugins (`plugin-secrets/<executable>.json` in kilna's own folder), never in
the workspace's database or its exports. The form shows only that it is set;
every run is handed its value with the other settings. A plugin that would
rather ask the system's keychain still may.

A manifest at odds with itself - a command or a setting named twice, a choice
with nothing to choose, a default its setting does not take - leaves the plugin
listed with what is wrong, and unable to run.

## Running a command: `run`

When the user triggers a command, kilna runs the plugin again with a single
`run` argument and writes one JSON **invocation** to its stdin:

```jsonc
{
  "command": "count",
  "target": "work",
  "subject": { "id": "…", "title": "…", "kind": "song", "meta": {}, "bodies": { "lyrics": "…", "style": "…" } },
  "settings": { "channel": "Harbour Radio", "crf": 20 }
}
```

`settings` holds [its settings](#settings) as they stand: what was set in the
window, and the defaults of the rest. It is empty for a plugin that declares
none.

`subject` is the row the command was invoked on, as the frontend sees it. A
work - for a `work`, a `cuts` or a `score` command - travels with everything a
plugin would otherwise have to ask for back:

| Field | What it holds |
| --- | --- |
| `bodies` | The latest version of each role, keyed by role. |
| `ordinal` | The work's number among the works of its kind made from the same origin - the 2 of *Tide (short 2)*. `null` for a work made from nothing. |
| `folder` | The work's [folder on disk](/kilna/guides/files-and-covers/#the-works-folder-on-disk), whole, when its kind names one - whether or not it is there yet. |
| `file` | Its [finished file](/kilna/guides/files-and-covers/#the-finished-file): `state` (`found`, `chosen`, `chosenMissing`, `notYet`, `noRule`, `noFolder`), the kind's `rule`, the `relative` path inside the folder and the whole `path`. |
| `shots` | The stretches it is spliced from, in order: `starts_at`, `ends_at`, `source_id`, `source_title`, and `path` - the donor's finished file, `null` when it has none, with `file_state` saying why. |
| `donors` | What it was made from, each with `id`, `title`, `kind`, `folder` and `file`. |
| `chain` | The titles up the chain "made from", nearest first: a short's clip, then the song. |
| `origin` | What it is all made from - the song, for a short - as `id`, `title`, `kind`. |
| `cover` | The picture it goes out under, a whole path, or `null`. |
| `files` | The files attached to it, oldest first (v0.148): each `kind`, its whole `path` and the `name` it arrived under - a track kept on its Files tab rather than in a folder. |
| `axes` | For a `score` command only: the axes to judge along, as the work's kind names them - each with `key`, `label`, `weight`, `scale` and `kind`. |

A cutter has what it needs in one call: the stretches and the file each comes
out of, where to put what it makes, the song's name to letter the frame with,
and the cover to end on.

The plugin does its work and prints one JSON **outcome** to stdout before
exiting:

```jsonc
{
  "meta": { "word_count": 214 },
  "message": "214 words"
}
```

- `meta` — fields to merge into the subject's `meta`. Optional; omit or leave
  empty if the plugin has nothing to add.
- `message` — a line shown to the user. Optional.
- `files` — the files the command made, whole paths. Optional. The first one
  inside the work's folder becomes its finished file when the kind's rule
  would not find it by itself; a file the rule finds anyway - a new `v2.mp4`
  beside `v1.mp4` - changes nothing, so the next render made outside kilna is
  found too.
- `suggestions` — what a command on the Cuts tab heard in the videos the
  short is made from (v0.103). Optional. One object per video:
  `source_id` (its id from `donors`), `file` (the file heard, whole, as kilna
  handed it over), `boundaries` (seconds where its parts begin) and
  `stretches` (`starts_at`, `ends_at`, `reason`, `score`, best first). kilna
  keeps each only while that file is still the video's finished file, draws
  the beginnings on its track for edges to land on, and offers the
  stretches beside it - see
  [Where to cut from](/kilna/guides/shorts/#where-to-cut-from). The reasons
  kilna words are `chorus`, `loudest`, `repeats`, `lift` and `part`; any other
  is shown as it is.
- `score` — what a `score` command judged. Optional, and read only from one.
  `axes` is axis key to mark and `note` says why:
  `{ "axes": { "hook": 8, "craft": 6 }, "note": "The chorus carries it." }`.
  It is not written: it becomes a proposal on the
  [Score tab](/kilna/guides/scoring-a-work/), under the plugin's name, held to the
  kind's axes as any other proposal is - marks brought within their scales, an
  axis the kind does not have named, one left out named too. Kept, it is an
  ordinary score naming the plugin as its judge. Marks along none of the kind's
  axes are refused, and the refusal says which axes there are.
- `candidates` — several results to choose from (v0.155): the takes a
  generator made, say. Optional, and read only on a work. One object per
  result: `label`, `files` (whole paths), `meta` (fields as above), `seconds`
  (how long it plays) and `note` (how it strays from what was asked); all but
  `label` optional. Nothing of them is kept until the person picks one in the
  window; the one picked is kept as a run's answer is - its `meta` merged, its
  first file inside the work's folder the finished one.
- `quota` — what the run spent of its service's allowance (v0.155). Optional.
  `used` and `unit` (`{ "used": 10, "left": 490, "unit": "credits" }`), `left`
  when the service says. The window says it, and since v0.158 kilna sums
  `used` over the month on this machine and keeps the last `left`: **Settings
  → Plugins** shows *This month: 15 credits spent, 485 left* under the plugin.
  A new month, or another unit, starts the count again.
- `error` — set this instead of the above to report failure without needing a
  non-zero exit code; kilna surfaces it as the command's error.

Printing nothing at all on stdout is treated as success with nothing to say,
not as a failure.

## Running over many works

A command marked `"batch": true` on `"work"` can be run over many works at
once (v0.155): the works ticked in the catalogue, or every work of a
collection, under **Plugins…**. kilna asks how many runs go at a time - one to
four - and runs the command once per work, exactly as a single run would: the
same request on stdin, the same settings, the same allowance of time. What
each run returns is kept as it comes back, so a batch stopped halfway keeps
what was done. **Stop** stops the runs going on and starts no more; the report
at the end says work by work how it went, in the plugin's own words or its
error.

The catalogue's **Media** chips - *Ready for a video*, *No audio*, *No cover*,
*Ready to go out* - pick the works a batch is for: what holds a track and a
cover and no video yet is what a render makes, and what holds its video is
ready to go out. They read what is attached to
each work, its chosen cover and the files of its folder, and count only what
has not gone out.

A batch's run gets no progress bar of its own - kilna shows how many works are
done - so a plugin need not change anything to be run in one.

## Generating pictures

A picture generator - by convention named `kilna-plugin-image-<provider>` -
offers its command on `"cover"` (v0.158). Beside everything a work's command is
handed, it gets `picture`: what the Cover tab would copy, for the shape the
cover is written for.

```jsonc
"picture": {
  "prompt": "Paper lanterns over a river at night, warm light, …",
  "negative": "text, watermark, extra fingers",
  "typography": null,
  "aspect_ratio": "16:9"
}
```

How many pictures to make, which model and the provider's key are the
plugin's own [settings](#settings) - the key marked `"secret": true`. Every
picture it returns, as `files` or in `candidates`, is copied into the space
and set beside the work's cover to choose from on the Cover tab; nothing
becomes the cover until the person picks it. A file that is not a picture is
left out.

## How metadata is merged

Whatever a plugin returns under `meta` is merged into the subject's existing
`meta`: the plugin's keys are added or overwritten, but nothing else is ever
cleared. A plugin can only ever affect the keys it names — losing unrelated
metadata to a third-party integration is not a recoverable mistake, so kilna
doesn't allow it structurally.

The merge is made into the fields as they are when the plugin answers, not as
they were when it was called, so a field edited while it ran is kept. And it
is recorded like any other edit: `Ctrl+Z` takes back what the plugin wrote.

## How long a plugin may take

A command has **two minutes** to answer unless its manifest says otherwise:
`"timeout": 900` gives it fifteen, and an hour is the most a manifest can
claim. One that runs past its allowance is stopped, and kilna says so; its
output is discarded. kilna keeps working while a plugin runs.

## Saying how far along you are

While it runs, a plugin may write lines of JSON to **stderr**:

```jsonc
{"progress": 0.4, "message": "cutting 2 of 3"}
```

Both fields are optional. `progress` runs from 0 to 1 and is drawn as a bar
under the command; `message` is shown beside it. A bar with no `progress` yet
moves without filling, which is the honest picture of "working, nobody knows
how long". Any other line of stderr - what ffmpeg says, say - is kept, and
shown only if the plugin fails.

**Stop**, beside the bar, kills the plugin's process at once; kilna says the
run was stopped and keeps nothing it might have half-written to the
workspace. Files it already wrote to disk are yours to tidy - a cutter should
write to a new name and only then report it.

## What a plugin should not assume

- **No persistent process.** A plugin is spawned per call and expected to
  exit. There's no lifecycle hook, no daemon, nothing kept warm between
  invocations.
- **No dynamic loading.** Plugins are never loaded into kilna's own process —
  Rust has no stable ABI for that, and the integrations plugins exist for
  (talking to other services, running external tools) are exactly the kind a
  sandboxed runtime would forbid anyway.
- **Windows needs a runnable extension.** On Windows, only `.exe`, `.cmd` and
  `.bat` files under the `kilna-plugin-` prefix are treated as plugins; a
  stray `kilna-plugin-notes.txt` is correctly ignored.

## Checking your protocol version

If you're building against a specific kilna release, run your own executable
with `--manifest` and confirm `protocol_version` matches what that release of
kilna expects before shipping — a version bump on either side is meant to be
visible, not silently absorbed.
