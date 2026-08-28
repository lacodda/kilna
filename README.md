<img src="https://raw.githubusercontent.com/lacodda/kilna/main/assets/banner.svg" alt="kilna" width="720">

# kilna

**A desktop workbench for content makers — from raw idea to shipped work.**

You write songs, chapters, episodes, or articles. The work lives in one place, the release plan in another, and the decision about what actually deserves to ship lives in your head. kilna closes that loop.

> **Status: early development.** The loop is closed end to end — a work gains versions, a version earns a score, a score wins a calendar slot, and the slot ends in a release you mark by hand — with four craft profiles, an AI panel over your own Claude Code CLI, and a plugin protocol. Nothing is deleted outright and nothing happens unnoticed: there is a trash behind every deletion and a history behind every change. The interface speaks English and Russian. Builds for Windows, macOS and Linux are on the [Releases page](https://github.com/lacodda/kilna/releases); none of them are signed yet, so your OS will warn you about an unidentified developer.

<img src="https://raw.githubusercontent.com/lacodda/kilna/main/assets/screenshot.png" alt="The kilna catalogue with the search palette open over it: one box finding a work by title and two drafts by a line inside them, with the assistant's floating button in the corner" width="1200">

## The loop

```
work  →  versions  →  score  →  calendar slot  →  shipped
```

Every craft repeats the same cycle. kilna models it once and lets your craft configure the vocabulary.

- **Work with versions.** Every draft is kept whole — no diff chains, no lost revisions. A song keeps lyrics and style separately; a chapter keeps text and outline. Name them, compare any two side by side, and write full screen; an unsaved draft survives a closed window.
- **Scoring that means something.** Rate a work along axes you define, weighted into a tier — a scale you click or drive from the keyboard, with each axis showing the question it asks. Scores are snapshots tied to a version, with a note on why, so you can see that rewriting the second verse moved it from 62 to 78.
- **A status you never have to keep up.** Scoring makes a work scored, a calendar slot makes it scheduled, going out makes it released — worked out from what happened, in your profile's own words. Set one by hand and it stays put; the automation steps over that work entirely until you hand it back.
- **A calendar that pushes back.** Slots compete. A stronger work bumps a weaker one out of Thursday — and one click lays the whole queue out to your profile's rhythm, as a preview you approve before anything is booked.
- **Shipping is a state, not an integration.** Mark it out, paste the link. Plugins can automate it later; the loop closes without them.
- **A first screen that answers "what now".** kilna opens on what needs a decision — a slot this week the work is not ready for, overdue first — then the week as it stands, then the shortlist of scored work going nowhere yet. Already-released work never appears: the automation shows what can still be done.
- **One box finds anything.** `Ctrl+K` searches titles, every draft's full text, notes and assistant replies at once — and folds case in full Unicode, so a Russian workspace is as searchable as an English one.

## Profiles, not a form builder

A song, a chapter, and a podcast episode differ in vocabulary and in how they're judged — not in structure. So the schema is fixed and the craft is configuration:

```jsonc
{
  "work_kinds":    ["song"],
  "release_kinds": ["clip", "short", "audio-release"],
  "statuses": [
    { "key": "draft",    "label": "Draft",    "derive": "draft" },
    { "key": "released", "label": "Released", "derive": "released" },
    { "key": "shelved",  "label": "Shelved",  "derive": "manual" }
  ],
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

Asking something starts a run that belongs to the chat, not to the screen you were on: leave the card, open another work, come back — it is still going, and the panel replays what it said while you were away. A floating button carries a badge while runs are in flight and opens every chat from any screen — a work can hold several chats, and a chat does not need a work to be about. Answers render as markdown with one-click copy, and an answer worth keeping becomes a version of the work with one more. A profile action means two things by design: in the panel it fills the composer so you read what is about to be sent (typing `/` reaches the same list from the keyboard), and on a work's Overview tab it runs at once in a chat of its own — you are told when it lands, wherever you are, and the same action cannot be started twice on the same work. An action that stops to ask you something says so with a banner instead of leaving the question in a chat you are not watching, and an action can come back with a score along your axes for you to apply with one button. **The assistant never writes to your workspace**: it proposes, you apply. Runs start in an empty directory, so the model sees what the prompt carries — not whatever files kilna was launched next to. You can watch the tools it uses as it works, stop it, and have up to three going at once — tick a batch of works in the catalogue and the rest queue for a free slot, counted down by a bar you can see from any screen. See [The assistant](https://lacodda.github.io/kilna/guides/the-assistant/).

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

Nothing is lost by pressing a button either: deleting moves things to a [trash](https://lacodda.github.io/kilna/guides/the-trash/) you can restore from, so kilna never asks you to confirm — the undo is in the message.

And nothing happens unnoticed. Every change leaves a line in [History](https://lacodda.github.io/kilna/guides/the-history/) — what happened, to what, and when — so a question you only think to ask three weeks later still has an answer. The bell in the top bar lights only for the entries that need a look, such as a release pushed out of its slot by a stronger work.

## Built with

[Tauri v2](https://v2.tauri.app/) · Rust · SQLite · React

## Development

Requires Rust (1.85 or newer), Node 22+ and pnpm.

```sh
pnpm install
pnpm tauri dev            # run the app
pnpm lint                 # eslint + tsc + locales + tests
pnpm test                 # frontend unit tests
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
