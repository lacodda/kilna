---
title: Getting Started
description: Build kilna from source, run it, and see where your workspace lives.
---

## Install

Grab a build from the [Releases page](https://github.com/lacodda/kilna/releases):
an `.msi` or `.exe` for Windows, a `.dmg` for macOS (Apple silicon), and a
`.deb`, `.rpm` or `.AppImage` for Linux. Nothing is signed yet, so both Windows
and macOS will warn you about an unidentified developer.

## Build from source

You need Rust (1.85 or newer), Node 22+ and pnpm.

```sh
git clone https://github.com/lacodda/kilna.git
cd kilna
pnpm install
pnpm tauri dev
```

`pnpm tauri dev` compiles the Rust backend and opens the app window. The first
build takes a while; later ones are incremental.

Development builds show one extra sidebar entry, **Styleguide** — the living
inventory of the design system. Screens take their controls from that page and
only from there; it is not part of the released app.

To check the backend on its own, without the UI:

```sh
cd src-tauri
cargo test
cargo clippy -- -D warnings
```

## First run

On first launch kilna creates a fresh workspace and seeds it with the four
built-in profiles — **Music**, **Novel**, **Podcast** and **Blog** — with
**Music** active by default. Nothing else is pre-populated: no sample works,
no demo data.

Switch profiles from the picker at the bottom of the sidebar at any time.
Switching doesn't lose anything — works keep the kind and status they were
given, even if you later edit the vocabulary that named them.

kilna is a studio tool, so it is dark by default — more precisely, it follows
your system's theme until you say otherwise. The theme button in the sidebar
footer cycles through *system*, *light* and *dark*, and the choice is
remembered across launches. See [The interface](/kilna/concepts/the-interface/)
for the full tour of the frame.

## What a workspace is

A workspace is one SQLite database holding everything kilna knows: profiles,
works, versions, scores, releases, notes and chat history with the AI panel.
Media — audio, video, images — lives as plain files on disk; the database
holds only the path to each one. See
[Local-first storage](https://github.com/lacodda/kilna/blob/main/docs/adr/0002-local-first-storage.md)
for the reasoning.

### Where it lives

The workspace file is created under the platform's application data
directory the first time kilna runs — there is no way to point it elsewhere
yet. From inside the app, the **workspace path** command shows you exactly
where the file is, so you can find it or back it up directly.

Schema changes ship as versioned migrations under `src-tauri/migrations/`;
the schema is never edited in place, so an existing workspace upgrades safely
when you update kilna.

## Put your first work in

Create a work, give it a title and a kind (a **song**, if you're still on the
Music profile), and add a version — the body of a draft, kept whole rather
than as a diff. A song keeps lyrics and style as separate version roles; other
profiles use different roles for the same idea.

## Where next

- [The loop](/kilna/concepts/the-loop/) — work, versions, score, slot, shipped, and why each step exists.
- [Profiles](/kilna/concepts/profiles/) — how Music, Novel, Podcast and Blog share one schema.
- [Scoring a work](/kilna/guides/scoring-a-work/) — a walkthrough.
- [Data](/kilna/reference/data/) — export, backup, and where the workspace file lives.
