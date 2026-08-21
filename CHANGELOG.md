# Changelog

All notable changes to this project are documented in this file.

## [0.21.0] - 2026-08-21

### Features
- One list of works, not two
## [0.20.0] - 2026-08-20

### Bug Fixes
- Judge a work by the version that currently is the work
- Land imported statuses in the profile's own vocabulary
- Let the journal gate see a sentence that counts things
- Stop the tab strip drawing a scrollbar it does not need
- Lay the fields out in columns that line up
- Keep the whole status readable while it is pinned

### Documentation
- Cross-link the statuses screen, and say so in the readme

### Features
- Derive a work's status, and let a person pin it
- Show who owns a work's status, and preview a mass restate
- Bring the card's header in line with the mockup
## [0.19.0] - 2026-08-19

### Bug Fixes
- Make the list filter find Russian titles

### Documentation
- Finding things, and why case works in Russian

### Features
- One query across works, drafts, notes and chats
- Ctrl+K opens the palette, from anywhere
## [0.18.0] - 2026-08-18

### Documentation
- Judging a work, and what the score remembers

### Features
- A scale you judge with, not a box you type in

### Testing
- Hold the rules the docs promise
## [0.17.0] - 2026-08-18

### Bug Fixes
- Address the versions tab as /versions, not /lyrics
- Hold the gate to keys the code actually asks for

### Documentation
- Writing a version, and what the editor keeps

### Features
- An editor worth writing in

### Testing
- A test runner, and a diff that has to be right
## [0.16.0] - 2026-08-17

### Bug Fixes
- Keep a draft with the role it was typed under
- Put plugin actions beside the fields they change

### Documentation
- The card, its tabs, and what stays on screen

### Features
- A work's card becomes a header and seven tabs
## [0.15.0] - 2026-08-17

### Bug Fixes
- Name a trashed score by what it said, not when

### Documentation
- The history, and what it keeps after the message fades
- Retake the screenshot on the current build

### Features
- Record what happened in words that can be translated
- A history screen, a bell, and each work's own history
## [0.14.0] - 2026-08-16

### Documentation
- The trash, and why deleting never asks

### Features
- Move deletions aside instead of destroying them
- A trash screen and an undo in place of confirmations
## [0.13.0] - 2026-08-14

### Documentation
- The language switch, and what it does not translate

### Features
- A Russian locale and a gate that holds locales to one shape
- Choose the interface language
## [0.12.0] - 2026-08-14

### Documentation
- Describe what the app says back

### Features
- Give failures a stable kind the frontend can branch on
- Put a data layer and real feedback under the screens

### Refactoring
- Drop the unused notFound predicate
## [0.11.0] - 2026-08-13

### Features
- Sidebar and topbar frame over real routes
## [0.10.0] - 2026-08-13

### Bug Fixes
- Tolerate a plugin that answers without reading stdin

### Documentation
- Point at the builds that now exist
- Changelog for v0.10.0

### Features
- Design system on brand tokens, radix and lucide
## [0.9.0] - 2026-08-12

### Bug Fixes
- Say why an unscored release cannot take a slot
- Keep the process helpers lint-clean off Windows

### CI
- Build and publish desktop bundles on a tag
- Let packageManager decide the pnpm version

### Documentation
- A documentation site built from the source, not from memory
- Changelog for v0.9.0
- A screenshot taken from the running build

### Features
- The approved mark — ki, the heat, magenta
## [0.8.0] - 2026-08-11

### Documentation
- Changelog for v0.8.0

### Features
- A plugin system, and the first plugin to use it
## [0.7.0] - 2026-08-11

### Documentation
- Changelog for v0.7.0

### Features
- Four crafts on one schema, switching and a profile editor
## [0.6.0] - 2026-08-11

### Documentation
- Changelog for v0.6.0

### Features
- Markdown export, backups, and importing a predecessor workspace
## [0.5.0] - 2026-08-11

### Documentation
- Changelog for v0.5.0

### Features
- The AI panel, over the user's own Claude Code CLI
## [0.4.0] - 2026-08-11

### Documentation
- Changelog for v0.4.0

### Features
- Calendar, releases and collections
## [0.3.0] - 2026-08-11

### Documentation
- Changelog for v0.3.0

### Features
- Scoring, score history and the catalogue
## [0.2.0] - 2026-08-11

### Documentation
- Changelog for v0.2.0

### Features
- Works, versions and notes behind Tauri commands
- Work list, work card and version history
## [0.1.0] - 2026-08-11

### CI
- Check the frontend, the backend and the desktop bundle

### Documentation
- Found the project
- Record the frontend decision as ADR 0003

### Features
- Add the provisional kilna mark and asset export
- Scaffold the application over a migrated SQLite workspace
