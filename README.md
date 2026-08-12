<img src="https://raw.githubusercontent.com/lacodda/kilna/main/assets/banner.svg" alt="kilna" width="720">

# kilna

**A desktop workbench for content makers — from raw idea to shipped work.**

You write songs, chapters, episodes, or articles. The work lives in one place, the release plan in another, and the decision about what actually deserves to ship lives in your head. kilna closes that loop.

> **Status: early development.** The loop is closed end to end — a work gains versions, a version earns a score, a score wins a calendar slot, and the slot ends in a release you mark by hand — with four craft profiles, an AI panel over your own Claude Code CLI, and a plugin protocol. Builds for Windows, macOS and Linux are on the [Releases page](https://github.com/lacodda/kilna/releases); none of them are signed yet, so your OS will warn you about an unidentified developer.

<img src="https://raw.githubusercontent.com/lacodda/kilna/main/assets/screenshot.png" alt="A song open in kilna: its fields, its two lyric revisions, and the current draft" width="1200">

## The loop

```
work  →  versions  →  score  →  calendar slot  →  shipped
```

Every craft repeats the same cycle. kilna models it once and lets your craft configure the vocabulary.

- **Work with versions.** Every draft is kept whole — no diff chains, no lost revisions. A song keeps lyrics and style separately; a chapter keeps text and outline.
- **Scoring that means something.** Rate a work along axes you define, weighted into a tier. Scores are snapshots tied to a version, so you can see that rewriting the second verse moved it from 62 to 78.
- **A calendar that pushes back.** Slots compete. A stronger work bumps a weaker one out of Thursday.
- **Shipping is a state, not an integration.** Mark it out, paste the link. Plugins can automate it later; the loop closes without them.

## Profiles, not a form builder

A song, a chapter, and a podcast episode differ in vocabulary and in how they're judged — not in structure. So the schema is fixed and the craft is configuration:

```jsonc
{
  "work_kinds":    ["song"],
  "release_kinds": ["clip", "short", "audio-release"],
  "axes": [
    { "key": "hook", "label": "Hook", "weight": 2,   "scale": 10 },
    { "key": "text", "label": "Text", "weight": 1.5, "scale": 10 }
  ],
  "tiers": [{ "key": "clip", "min": 75 }, { "key": "hold", "min": 0 }]
}
```

kilna ships with **Music**, **Novel**, **Podcast** and **Blog**. Switch profile and the same screens speak about chapters and pull, or episodes and the cold open. No new code, no migration — the schema never moves.

## AI panel

kilna talks to Claude through your installed [Claude Code](https://claude.com/claude-code) CLI — your subscription, your session, your skills. Nothing is sent anywhere else, and there is no API key to configure.

Prompt templates come from the active profile, with placeholders filled from the work in front of you:

```jsonc
{
  "key": "critique",
  "label": "Critique the lyrics",
  "template": "Here are the lyrics of a song called \"{title}\".\n\n{role:lyrics}\n\nWhich lines are weak?"
}
```

So "Critique the lyrics" means the right thing in a music profile and something else entirely in a novel one. Conversations are kept in the workspace, and a follow-up question continues where the last one left off.

The core works without it. The panel is amplification, not a requirement — if the CLI isn't installed, the panel says so and everything else carries on.

## Plugins

Integrations are ordinary executables named `kilna-plugin-*`, found on your `PATH` or in the workspace's `plugins` directory. kilna runs one with `--manifest` to ask what it offers, then with `run` and a JSON invocation on stdin:

```jsonc
{ "command": "count", "target": "work", "subject": { "title": "…", "bodies": { "lyrics": "…" } } }
```

Whatever the plugin returns under `meta` is merged into that row — it can add and overwrite its own keys, never clear the rest. Plugins live for the duration of a call; nothing starts a service, and nothing is loaded into kilna's process.

[kilna-plugin-wordcount](https://github.com/lacodda/kilna-plugin-wordcount) is the reference implementation.

## Your data stays yours

Local SQLite, media as plain files on disk, and a markdown export of everything — front matter, full bodies, scores and releases, readable without kilna. Back the whole workspace up to a single file. No account, no server, no lock-in.

## Built with

[Tauri v2](https://v2.tauri.app/) · Rust · SQLite · React

## Development

Requires Rust (1.85 or newer), Node 22+ and pnpm.

```sh
pnpm install
pnpm tauri dev            # run the app
pnpm lint                 # eslint + tsc
cd src-tauri
cargo test                # backend tests
cargo clippy -- -D warnings
```

The workspace database is created under the platform's application data
directory on first run. Schema changes are versioned migrations in
`src-tauri/migrations/`; the schema is never edited in place.

Full documentation: [lacodda.github.io/kilna](https://lacodda.github.io/kilna). Architecture decisions live in [docs/adr](https://github.com/lacodda/kilna/tree/main/docs/adr).

## License

MIT
