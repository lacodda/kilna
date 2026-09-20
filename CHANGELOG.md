# Changelog

All notable changes to this project are documented in this file.

## [0.74.3] - 2026-09-20

### Bug Fixes
- Give every screen the window's bottom edge
- Hold the row menu to the window, and take dowel's palette back
- One scale for the corners, and colours that exist

### Features
- Let a role say whether it is a draft of the work
- Fill a day by choosing a work, and sift the queue
- One button for the actions, and a journal that opens the work

## [0.74.2] - 2026-09-20

### Bug Fixes
- The dead compare button, the version count, and the stage dial
- Proposals in the bell, actions with glyphs, and the crowded rows
- Judge a song as a song, not as the video it might become

### Features
- Carry the craft's words in both languages

### Testing
- Hold every sentence's holes to what fills them
## [0.74.1] - 2026-09-18

### Bug Fixes
- Open the search, raise the popups, hold the table's edges
- The stage funnel, the star, the card's top and the calendar's plus

### Testing
- The stage covers the page and stays under what it opens
## [0.74.0] - 2026-09-18

### Documentation
- Make the readme a shopfront
- Cut the readme down to a shopfront
- Record that a door belongs to the work that goes through it
- Column order, dragged widths and the header funnels

### Features
- The window's own title bar, a splash and a remembered window
- A version beside the text, and the words it repeats
- A door belongs to the work that goes through it
- Column widths, a drawn order and header funnels in the model
- Drag column widths, reorder columns, filter from a heading

### Refactoring
- Take rendered markdown from dowel's prose stylesheet
- Take the eight new components from dowel's registry

### Testing
- The selectable gate reads the marked text, not the diff it replaced

### Breaking Changes
- The Studio profile no longer offers `clip` and `short`
as release kinds of a song; existing releases are moved on first open.
## [0.73.0] - 2026-09-17

### Bug Fixes
- Say what is being taken back, instead of printing the key

### Documentation
- Shorts, the cut list and the cover's prompt

### Features
- Keep which stretches of a video a short is made of
- Take, move, reorder and drop a stretch
- The prompt a work's cover picture is drawn from
- Two shorts of one video never land on neighbouring days
- The arithmetic of a splice, and the calls behind it
- A track of the donor, and the cover's prompt above its picture
- Words for the splice, the cover's prompt and five silent undos
## [0.72.0] - 2026-09-17

### Bug Fixes
- The tag suggestions close on Escape and on a click outside
- The craft's own stage stops reach a workspace that predates them
- The dial is on the week's rows, not only on the decisions

### Documentation
- The stage dial, full-text search and the card's field row

### Features
- Find a work by a word inside it, not only in its title
- How finished a work is, as its author judges it
- A dial beside the star, and a column of them in the catalogue
- The row of craft fields is a setting, not a fixture
- Words for the stage dial, the card switch and a new tag

### style
- Rustfmt the carried-forward stages
## [0.71.0] - 2026-09-16

### Bug Fixes
- A release field's hint and template say which is which

### Features
- What a release goes out as, written from the work
- The boxes a release goes out in, and a month of them at once
- A work packed into a folder, and a package that plans how it ships
- The boxes a release goes out in, edited on the screen
## [0.70.0] - 2026-09-16

### Bug Fixes
- The number column fits the field it now holds

### Documentation
- Numbering the board, and what it still owes

### Features
- A number you can type, and a board that closes up behind it
- What the board still owes, counted above the board
## [0.69.0] - 2026-09-16

### Bug Fixes
- A month that clears the dialog, and a queue that scrolls alone

### Documentation
- The video strip, the montage list, and a second attempt

### Features
- A scene's material has a kind, so a video is one of them
- Clips beside the stills, and the list a board is cut from
- Files in groups, and a second attempt at a video
## [0.68.0] - 2026-09-15

### Documentation
- Frames on the board, and the decision behind them

### Features
- A scene keeps the frames drawn for it, and says which one it is
- Frames on the board, three ways in, and a scene that is shot
## [0.67.0] - 2026-09-15

### Features
- A file copied into the workspace, with a row that knows whose it is
- The window may show the workspace files, and commands to attach them
- A work wears its cover
- A Files tab, a cover in the catalogue, and a backup that carries both
## [0.66.0] - 2026-09-15

### Features
- Regenerate one prompt block, and say how long a run took
## [0.65.0] - 2026-09-14

### Features
- Time a board from the work's duration
- Frame a board from the parts the text marks out
- The board as one table, with the long text folded away
- A scene points at who is in it and where it happens

### Breaking Changes
- Time a board from the work's duration
## [0.64.0] - 2026-09-13

### Features
- Actions on the board, previewed as sent

### Breaking Changes
- A scenes proposal carries `change` (add, replace, revise) instead of the `replace` flag, and `propose_scenes` takes `change` in place of `replace`; a stored `replace: true` still applies as a replacement.
## [0.63.0] - 2026-09-12

### Features
- Rename in place, copy, star, and badges for status and marks
## [0.62.0] - 2026-09-12

### Features
- Propose a storyboard, added to the board or replacing it
## [0.61.0] - 2026-09-12

### Features
- An action carries its method, and a task is about a version
## [0.60.1] - 2026-09-12

### Bug Fixes
- Render a note's body as markdown
## [0.60.0] - 2026-09-12

### Features
- A storyboard row per scene, owned by the work
## [0.59.0] - 2026-09-11

### Features
- A work made from another, with the version it was taken at
## [0.58.0] - 2026-09-11

### Features
- Narrow to a kind of work, and read each kind down its own columns
## [0.57.1] - 2026-09-11

### Bug Fixes
- Offer no single undo after a package

### Features
- Propose a whole work, and apply proposals with one click
## [0.57.0] - 2026-09-11

### Bug Fixes
- Register the server for the whole machine, not one project

### Documentation
- Register in the user scope
- Describe format 2 of the profile document

### Features
- Give each kind of work its own vocabulary
- Read the vocabulary of the work's kind on every screen

### Breaking Changes
- `config.axes`, `tiers`, `version_roles`,
`release_kinds` and `statuses` moved under each entry of
`work_kinds`; the flat keys are gone from the stored document and from
the frontend types. Old documents load and are rewritten automatically;
a profile JSON written by hand may stay flat, which reads as the same
vocabulary for every kind.
## [0.56.0] - 2026-09-11

### Documentation
- Describe the MCP server and how an agent outside the window proposes

### Features
- Serve the workspace to an agent, who proposes rather than writes
- Show and apply proposals from outside the window
## [0.55.0] - 2026-09-11

### Bug Fixes
- Run the sidebar the full height of the window
- Show the recorded marks on the scales and keep the rubric row still

### Documentation
- Describe the editing session, how roles read and the scales that show the score

### Features
- Let a version role say how its body reads
- Change a version's body in place until something judges it
- Click into the text and it is the next revision
## [0.54.0] - 2026-09-10

### Documentation
- Describe blind scoring, held tiers, the judge and per-kind verdicts

### Features
- Hold a tier by hand, name the judge, and judge blind
## [0.53.0] - 2026-09-10

### Features
- Say what the marks mean and what the next tier costs
- Show how far each work is from its next tier
## [0.52.0] - 2026-09-09

### Features
- Take back the last thing you changed
- Offer the undo where the edit happened
## [0.51.0] - 2026-09-09

### Features
- Record what was asked for, and rebuild the workspace from it
## [0.50.0] - 2026-09-08

### Features
- Lay the columns and keys of the scoring and release model
## [0.49.0] - 2026-09-07

### Features
- Leave a tombstone on delete and a clock on each edit
## [0.48.0] - 2026-09-06

### Documentation
- Say the catalogue is one list you can slice

### Features
- Narrow from the box, and keep a slice under a name
## [0.47.0] - 2026-09-05

### Bug Fixes
- Stop promising the contest in the showcase too
- Let the table scroll sideways, and gather the bulk actions

### Documentation
- Say how the bulk actions and the scrolling work now

### Features
- Act on a whole selection in one request
- Choose the columns and fold the table into blocks

### style
- Let rustfmt fold a signature back onto one line
## [0.46.0] - 2026-09-03

### Bug Fixes
- Name the version that actually brings each screen

### Features
- Answer to the keyboard from anywhere
- Open a row's actions with a right click too
- Hold the filter for as long as the app is open
- Offer the works you had open lately
## [0.45.0] - 2026-09-02

### Bug Fixes
- Stop promising a contest that was retired

### Documentation
- Say what the tab does and how a link behaves

### Features
- Name the day a release went out, and take the mark back
- Let the tab act on a release, and make its link open
## [0.44.0] - 2026-09-02

### Bug Fixes
- Say what a pin does now
- Turn the month, and put the dialog back in the window

### Features
- Pick a chip up and carry it
- Retire the contest for a date

### Breaking Changes
- A calendar day now holds any number of releases. Scheduling one onto a taken day no longer compares scores, refuses, or returns the weaker release to the queue. The `slotHeld` and `slotPinned` error kinds are gone, `Scheduling.displaced` is always null, and the `release.displaced` journal line is no longer written. A pinned date still keeps the auto-layout away, but no longer refuses a person.
## [0.43.0] - 2026-09-01

### Bug Fixes
- Make the window behave like a window
- Stamp every instant to the same width
- Break the tie in every ordering by a timestamp
## [0.42.0] - 2026-09-01

### Features
- Read a month at a glance
## [0.41.0] - 2026-09-01

### Features
- Move the overlays, the menus and the toasts onto dowel
## [0.40.0] - 2026-09-01

### Features
- Revise a version into the next one, and see what moved
## [0.39.0] - 2026-08-31

### Features
- Move onto dowel, the design system of the line
## [0.38.0] - 2026-08-30

### Features
- Open a review beside the draft it is written about
## [0.37.0] - 2026-08-30

### Features
- Let a work carry the author's own words and the flags they raise
## [0.36.0] - 2026-08-30

### Features
- Give a work the fields its craft actually describes it by
## [0.35.0] - 2026-08-29

### Bug Fixes
- Clear the five defects the pilot found on a real catalogue
## [0.34.0] - 2026-08-28

### Features
- Let a finding be heard, and put your own lines beside it
## [0.33.0] - 2026-08-28

### Bug Fixes
- Order a transcript by when it was written, not by uuid

### Features
- Notice what is worth doing, and offer the action for it
## [0.32.2] - 2026-08-28

### Bug Fixes
- Give each icon size the level of the mark that reads at it
- The home-screen icon is the full mark, not the small tile
## [0.32.1] - 2026-08-28

### Bug Fixes
- Ship the approved mark as the application icon
## [0.32.0] - 2026-08-28

### Features
- Open on what needs deciding
## [0.31.0] - 2026-08-27

### Documentation
- Record why a queued task is not persisted
- Mention batch actions on the landing page

### Features
- Run a profile action against many works at once

### Testing
- Check the batch path against real CLI processes
## [0.30.0] - 2026-08-27

### Features
- Notice a question, reach actions by typing, propose a score
## [0.29.0] - 2026-08-26

### Documentation
- Describe both ways a profile action runs
- Mention card-started actions on the landing page

### Features
- Start profile actions from the card without the panel

### Testing
- Retry a fake plugin that is still busy on unix
## [0.28.0] - 2026-08-26

### Bug Fixes
- Confirm the copy before showing its feedback

### Features
- Start runs in a dedicated empty directory
- Chat summaries, renaming, and messages tied to runs
- Make the chat a workplace
- Insert an answer as a new version of the work
- Reach the assistant from any screen

### Testing
- Pin answer pairing to run ids with parallel runs
## [0.27.0] - 2026-08-25

### Features
- Let a run outlive the screen that started it
## [0.26.0] - 2026-08-24

### Features
- State the release rhythm in the profile
- Lay the queue out to the rhythm, preview first
## [0.25.0] - 2026-08-22

### Bug Fixes
- Drop a mut the borrow checker never needed

### Features
- Let a release kind state the roles it needs
- Readiness marks on chips, the queue and the editor
- Show the outcome of a claim before it lands
- Warn ahead when an upcoming release is not ready
## [0.24.1] - 2026-08-22

### Bug Fixes
- Drag a release into the contest, not around it
- Say why a date was refused in the reader's language
## [0.24.0] - 2026-08-22

### Bug Fixes
- Make the journal gate see keys it cannot read

### Features
- A date that can be settled, judged by one score
- The pin behind the calendar
- Drag a release by its grip, and see what is settled
## [0.23.0] - 2026-08-21

### Documentation
- Point the version chips and comments at the right releases

### Features
- A month you can read, and releases you can edit
## [0.22.0] - 2026-08-21

### Bug Fixes
- Carry the lockfile version with the release commit

### Documentation
- Point at the catalogue for narrowing by kind or status

### Features
- A table you can work in
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
