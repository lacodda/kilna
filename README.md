<p align="center"><img src="https://raw.githubusercontent.com/lacodda/kilna/main/assets/banner.svg" alt="kilna - from raw idea to shipped work" width="720"></p>

> You write songs, chapters, episodes or articles. The work lives in one place, the release plan in another, and the decision about what deserves to ship lives in your head. kilna closes that loop.

<p align="center">
  <a href="https://github.com/lacodda/kilna/releases/latest"><img src="https://img.shields.io/github/v/release/lacodda/kilna?style=flat-square" alt="Release"></a>
  <a href="https://github.com/lacodda/kilna/actions"><img src="https://img.shields.io/github/actions/workflow/status/lacodda/kilna/ci.yml?style=flat-square" alt="CI"></a>
  <a href="https://github.com/lacodda/kilna/blob/main/LICENSE"><img src="https://img.shields.io/github/license/lacodda/kilna?style=flat-square" alt="License"></a>
</p>

<p align="center"><img src="https://raw.githubusercontent.com/lacodda/kilna/main/assets/screenshot.png" alt="The kilna catalogue with the search palette open over it: one box finding a work by title and two drafts by a line inside them, with the assistant's floating button in the corner" width="1200"></p>

## Why

Every craft repeats the same cycle, and every tool covers one arc of it. Drafts
end up in an editor, the plan in a spreadsheet, and the judgement of what is
actually ready stays in your head - where it quietly becomes "whatever I
touched last".

kilna models the whole cycle once:

```
work  →  versions  →  score  →  calendar slot  →  shipped
```

The structure is fixed; the vocabulary is yours. A song is judged on hook and
lyrics and ships as a clip; a chapter carries text and an outline and ships as
a release. Same screens, same loop, different words - configured in a profile
rather than coded.

## What you get

- **Drafts kept whole.** Every version is stored complete - no diff chains, no
  lost revisions - and any two compare side by side.
- **Scoring that means something.** Rate along axes you define, weighted into a
  tier, tied to the version that earned it, with a note on why.
- **A status you never keep up by hand.** Scoring, scheduling and shipping are
  worked out from what happened; set one yourself and the automation leaves
  that work alone.
- **A first screen that answers "what now".** Overdue slots first, then the
  week, then scored work going nowhere - each with the action that would
  answer it.
- **A storyboard for what is made in scenes.** A video or a short carries a
  table of scenes, prompts per generator, the pictures they came back with, and
  a montage list any editor reads.
- **An assistant that proposes, never writes.** kilna talks to Claude through
  your own [Claude Code](https://claude.com/claude-code) CLI - your
  subscription, no API key - and every answer lands as a proposal you apply
  with one button. `kilna --mcp` gives an agent outside the window the same
  door.
- **Your data stays yours.** Local SQLite, media as plain files, a markdown
  export readable without kilna. A trash behind every deletion and a history
  behind every change. No account, no server.

## Install

Download the installer for your platform from the
[latest release](https://github.com/lacodda/kilna/releases/latest): `.msi` or
`.exe` for Windows, `.dmg` for macOS, `.AppImage`, `.deb` or `.rpm` for Linux.

The builds are not signed yet, so your OS will warn about an unidentified
developer.

The first run asks for a workspace directory and a craft profile - kilna ships
with **Studio**, **Novel**, **Podcast** and **Blog**, and either can be changed
later. See [Getting started](https://lacodda.github.io/kilna/getting-started/).

## Status

v0.73.0, in daily use. The loop is closed end to end - a work gains versions, a
version earns a score, a score wins a calendar slot, and the slot ends in a
release you mark by hand - with four craft profiles, the assistant panel, an
MCP server and a plugin protocol. The interface speaks English and Russian.
What landed in each version:
[CHANGELOG](https://github.com/lacodda/kilna/blob/main/CHANGELOG.md).

## Documentation

**[lacodda.github.io/kilna](https://lacodda.github.io/kilna)** - the loop,
profiles, scenes, the assistant, the keyboard and the plugin protocol.

Writing a plugin: integrations are ordinary executables named `kilna-plugin-*`
that answer on stdin - see
[the guide](https://lacodda.github.io/kilna/guides/writing-a-plugin/) and
[kilna-plugin-wordcount](https://github.com/lacodda/kilna-plugin-wordcount),
the reference implementation.

Building it yourself:
[CONTRIBUTING.md](https://github.com/lacodda/kilna/blob/main/CONTRIBUTING.md).

## License

MIT (c) [Kirill Lakhtachev](https://lacodda.com)
