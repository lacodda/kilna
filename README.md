<img src="https://raw.githubusercontent.com/lacodda/kilna/main/assets/banner.svg" alt="kilna" width="720">

# kilna

**A desktop workbench for content makers — from raw idea to shipped work.**

You write songs, chapters, episodes, or articles. The work lives in one place, the release plan in another, and the decision about what actually deserves to ship lives in your head. kilna closes that loop.

> **Status: early development.** The first half of the loop works — works, versions and notes, with the built-in Music profile supplying the vocabulary. Scoring and the calendar come next. Nothing to install yet.

<img src="https://raw.githubusercontent.com/lacodda/kilna/main/assets/screenshot.png" alt="kilna showing the loop and the active profile" width="1000">

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

Switch to the Novel profile and the same screens speak about chapters, books, and pacing. No new code.

## AI panel

kilna talks to Claude through your installed [Claude Code](https://claude.com/claude-code) CLI — your subscription, your session, your skills. Prompt templates come from the active profile, so the assistant knows whether it's helping with a chorus or a chapter.

The core works without it. The panel is amplification, not a requirement.

## Your data stays yours

Local SQLite, media as plain files on disk, and a markdown export of everything. No account, no server, no lock-in.

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

Architecture decisions live in [docs/adr](https://github.com/lacodda/kilna/tree/main/docs/adr).

## License

MIT
