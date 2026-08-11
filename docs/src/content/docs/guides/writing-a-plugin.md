---
title: Writing a plugin
description: The plugin protocol — --manifest, run with JSON on stdin, the outcome shape, and how returned metadata is merged.
---

Integrations in kilna are ordinary executables, not a plugin API kilna's
process loads into itself. A plugin is any program named `kilna-plugin-*`
found on your `PATH` or in the workspace's `plugins` directory — written in
whatever language you like, run for the duration of a single call, and never
started as a background service.

[kilna-plugin-wordcount](https://github.com/lacodda/kilna-plugin-wordcount)
is the reference implementation — read it alongside this page if you want a
working example rather than just the shape of the protocol.

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
    }
  ]
}
```

`protocol_version` must match the version kilna speaks — currently `1`. A
mismatch doesn't crash anything; the plugin is listed but marked unusable,
with the mismatch shown rather than hidden. Each entry in `commands` becomes a
button wherever its `target` is drawn: `"work"` on a work's screen,
`"release"` on a release's.

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

`subject` is the row the command was invoked on, as the frontend sees it. For
a work, kilna adds a `bodies` object keyed by version role — the latest
revision of each — because a plugin acting on a work almost always wants its
text, and making every plugin ask for the body back separately would be
needless round-tripping.

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
