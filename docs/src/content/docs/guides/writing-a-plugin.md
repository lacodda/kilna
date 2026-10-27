---
title: Writing a plugin
description: The plugin protocol — --manifest, run with JSON on stdin, what a work is handed, the outcome shape, progress on stderr, and how returned metadata and files are kept.
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
Overview, `"release"` on a release, `"cuts"` on a work's
[Cuts tab](/kilna/guides/shorts/#the-cut-list) - where a cutter belongs.

`timeout` is how many seconds the command may run - see
[How long a plugin may take](#how-long-a-plugin-may-take). Every field added
since the first protocol is optional on both sides, which is why the version
is still `1`.

## Running a command: `run`

When the user triggers a command, kilna runs the plugin again with a single
`run` argument and writes one JSON **invocation** to its stdin:

```jsonc
{
  "command": "count",
  "target": "work",
  "subject": { "id": "…", "title": "…", "kind": "song", "meta": {}, "bodies": { "lyrics": "…", "style": "…" } }
}
```

`subject` is the row the command was invoked on, as the frontend sees it. A
work - for a `work` or a `cuts` command - travels with everything a plugin
would otherwise have to ask for back:

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
- `error` — set this instead of the above to report failure without needing a
  non-zero exit code; kilna surfaces it as the command's error.

Printing nothing at all on stdout is treated as success with nothing to say,
not as a failure.

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
