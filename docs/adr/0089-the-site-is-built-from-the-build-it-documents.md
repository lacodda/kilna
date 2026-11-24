# 0089 — The site is built from the build it documents

Date: 2026-11-24
Status: Accepted

## Context

The documentation had forty pages and no picture of the window: the one
screenshot in the repository was made by hand for the README many versions
ago, and the transcripts on the MCP page were sketches written from memory.
The first criterion a reader holds documentation to - that it shows the
program they have - was kept by nobody, because keeping it meant opening the
window and a paint program after every release.

A picture can be kept true in three ways: taken by a person and committed,
taken by a workflow and committed, or taken when the site is built and never
committed.

## Decision

**The Docs workflow builds the application and takes the pictures and the
transcripts from it, then builds the site.** Nothing it takes is kept in git.

- `docs/screens.json` lists every picture: its name, the words that describe
  it, and the steps that bring the window to it - a route opened, something
  pressed, something waited for. `tools/screens.mjs` makes a demo stand with
  the build's own `--demo-all`, opens the build's window on it through
  [tauri-driver](https://v2.tauri.app/develop/tests/webdriver/) and saves
  each picture under `docs/public/screens/`, which git ignores. The driver is
  spoken to in plain WebDriver over HTTP: a session, a script, a press, a
  screenshot. That is some two hundred lines, and a client library would be a
  dependency kept for them.
- A page marks a block `<!-- transcript: ... -->`, and `tools/transcripts.mjs`
  writes the block again from `kilna --mcp` on the same kind of stand. The
  blocks stay in git as the last taking, so a page reads whole in the
  repository; the site never shows them, it shows the build's.
- The pages and the list are held to each other without a build, in `pnpm
  lint`: a picture a page shows is one the list takes, a picture the list
  takes is shown, and the page describes it in the list's words.
- The README's picture is the site's, by its address.
- `kilna --workspace=<dir>` says in one word what `--workspace <dir>` says in
  two: a WebDriver hands a window its arguments as a set of switches, sorted,
  and a flag parted from its value names nothing.

## Consequences

- A picture cannot fall behind the program: there is no older one to show.
  One that can no longer be taken - a screen renamed, a button gone - stops
  the Docs run, and the site stays as it was until the list is put right.
- The Docs run starts on every push and builds the application, some minutes
  where it took one. It is not part of the gate a release waits for.
- The pictures are taken on Linux, by the engine the window has there, with
  Inter standing in for the system font: a reader on Windows sees their own
  font in the window and Inter on the site. The layout is the same.
- No workflow commits to `main`, so the release run never finds the branch
  moved under it, and every commit has its one author.
- The demo is now what the documentation looks like. A screen the demo
  leaves empty - the canon, the lab - has no picture until the demo fills it.
