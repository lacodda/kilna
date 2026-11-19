# Changelog

All notable changes to this project are documented in this file.

## [0.151.0] - 2026-11-19

Quotas and a bank. The rules a channel counts by hand - a Suno prompt of 400
to 500 characters, no more than four tracks of a month in one playlist, at
least five angles of a cover, no artist's name in a title - are written on
the passport as rules a machine measures, and one check reads them over a
work, a collection, a month or the whole space, in the window and for the
assistant; no number of any channel is written in kilna. A bank keeps the
themes a channel takes in turn, each resting its months after a use, and
marks one used when a release of a work naming it goes out. The register of
repeats now looks for a term in titles, pictures and sound as well as in the
lyric, with a limit a month and days apart, so a channel of instrumentals is
guarded too. And the assistant's calendar can see what the other spaces put
out on the same days.

### Bug Fixes
- The quota subject keeps its name across the wire
- A bank's filter wraps in a narrow panel, the quotas take a row
- The quota's works break a tie of their timestamps by row

### Features
- Quotas and banks on the passport
- Look for a term in titles, pictures and sound
- The calendar reads the neighbours on the same days

## [0.150.1] - 2026-11-19

Songs in the calendar's pane. A song goes out only as what is made from it,
so the pane's list of works with nothing planned left songs out, and a space
whose every clip, audio and short already waited in the queue showed no
works at all. A song is listed now until something whole made from it is
out, booked or waiting; carried onto a day, it is planned there.

### Bug Fixes
- List songs among the works with nothing planned

## [0.150.0] - 2026-11-18

A music channel. kilna now ships a profile for a channel that publishes
tracks made with a generator: each track one card from its sketch to its
release - its Suno prompt, its fields, its cover and frame, its YouTube and
Audius releases, judged on its sound, its tradition, its mood, its cover and
how unlike the month it is - with shorts and clips made from it, months
gathered into packs, a passport for the channel, an action that writes the
Suno prompt and an orchestrator that walks a track through. Its demo shows a
month of it. And a rhythm can now keep to days of the week: Tuesday,
Thursday and Saturday, thirteen a month, leaving out the weakest day of a
crowded month or adding a floating peak to a short one, at the hour of the
channel's own time zone with its summer time. The layout and the dashboard
follow the grid.

### Bug Fixes
- The rhythm's spacing has room for its number beside a long unit

### Features
- The Music channel profile, and a rhythm of weekdays with a monthly target and a time zone

### Testing
- A finding's complaint and params may be absent

## [0.149.0] - 2026-11-18

The simple video, on a timeline. A clip, an audio release or a short now has
a Build tab while the simple-video plugin is installed: the picture on one
track and the sound on another, with the track's waveform trimmed by its
edges, fades in and out of black and silence, the picture held before and
after the sound, the gain with a warning when the peaks would clip, and an
optimal loudness measured and set in one press. A preview plays it all before
anything is rendered, and Render the video makes the file with its progress
and a way to stop it. What is set is kept with the work and undone like any
edit.

### Bug Fixes
- The tab opens by its address before the plugins are read, says the track's own name, and shows what applies in an empty box
- A work refetched after the measure may be gone

### Features
- The Build tab - a simple video on two tracks, previewed, and rendered by its plugin

## [0.148.0] - 2026-11-18

A simple video, rendered. The new plugin kilna-plugin-simple-video renders
a work's cover - a picture, or a loop played over and over - over its track
into a video in the work's folder: at a stream's 1080p, quick to make with a
fade-out at the end, or as a 4K master with the sound at 320k and brought to
-14 LUFS. It follows the work's own timeline when it has one, and measures
how loud a track is and how far from what a platform wants. In kilna, a
plugin's command can now say it is meant for many works at once, and a work
hands a plugin the files attached to it, so a track kept on the Files tab is
found.

### Features
- A command can be for many works at once, and the simple-video plugin's guide
- A work's attached files travel with it to a plugin

## [0.147.0] - 2026-11-18

Places, the channel's dressing, and the vault's notes put in their place.
A place of the canon has a passport, words for a picture generator,
angles - wide, medium, detail - with their references, and picture bans; a variant
of a place, its epoch or its state, is read over its base. The channel card
keeps its dressing: avatar, banner and watermark kits with their prompts,
the picture in use, and the look before a rebranding in the archive. A
competing channel is a card that is never drawn. A work shows the reviews
and documents attached to it on its Overview, and a cover can be marked to
redraw from its own tab. `kilna --convert-notes` turns notes brought in from
a vault into cards, kits, reviews on their works and marks, by a file of
rules in the owner's words: a plan first, one undo when applied.

### Bug Fixes
- Kits in use before the archive, a kit's texts clamped, and a converted document retitled

### Features
- Places in detail, a place's variants, the channel's dressing, competitors, reviews of a video, and notes converted by rules

### Testing
- The reviews widget in the rule of what applies

## [0.146.0] - 2026-11-18

A song's material. A song now has an emotional arc on its Overview: a row
a section - what it feels and how strongly - with the curve the
intensities draw, read section by section by the assistant. A field can be
marked as kept by every action: Studio's anchors - the hooks and devices a
song keeps - go with every critique, polish and rewrite with the rule not
to touch them unless asked, and stand in the order and in an agent's
brief. The research board gains a reference track - what to take from a
song and where in it. `kilna --import-songs` brings the folders of songs a
previous system kept into their works, a plan and a report first. And the
cover constructor makes room: the shortlist folds to one line, and the
constructor takes the whole window the way the text editor does.

### Bug Fixes
- The anchors stand once in the order, under their own heading
- The emotional arc stands with the paragraphs and writes a box when it is left
- The arc's curve takes whole coordinates, not a formatted number

### Documentation
- The shortlist folds, the constructor takes the whole window

### Features
- A song's material - the emotional arc, kept anchors, reference tracks, and the old song folders brought in
- The shortlist folds to one line and the constructor takes the whole window

## [0.145.0] - 2026-11-18

Documents and links. Every shipped profile now has a Document kind beside
its notes: a long page read with its headings as an outline, set at a
reading size. A tag of a kind can be pinned as a selection at the top of the
list. A note can link to another note, a work or a card of the canon, and
the window follows the link; Obsidian's [[links]] in notes brought from a
vault turn into these in one press, with a list of the ones not found. A
phrase remembers the work it arose at apart from the one it was spent in -
said on its row, a filter on the shelf, and readable by agents. And
`kilna --import-ideas` brings a bank of phrases kept as markdown into a
space, marking notes as documents by their tags: a plan and a report first,
`--apply` to write, a copy of the space kept and one undo.

### Bug Fixes
- The document reader's missing import

### Features
- Documents, kilna links, the work a phrase arose at, and a bank brought in

## [0.144.0] - 2026-11-18

The canon over MCP. An agent reading the canon now learns whether the space
has its passport and what each shape of section carries, reads a long card
in parts - its summary and counts, one section, one state, a page of facts -
and names the passport `root`. Proposing, it can change a card's name, other
names, layer or summary, and a second passport becomes an update of the one
standing; move a fact to another section, refile it as it refines it, or
delete it; take a relation away; and a new fact the card nearly holds comes
back marked as similar. The proposal shows every new kind of item, each kept
or left out on its own. Two playbooks ship for agents: setting up a canon
from an interview or a description, and tidying one card by card.

### Features
- The canon over MCP - read in parts, changed, tidied

## [0.143.0] - 2026-11-18

The canon as a graph. An open card's side now draws it among the cards it is
related to - itself in the middle, the others on a ring in their kinds'
glyphs and colours, each line with the first words of its relation - and a
press on one opens it; every relation is listed under the drawing, with Add
a relation. The graph button over the cards maps the whole canon: the wheel
or the buttons zoom, dragging the ground moves the map, chips hide kinds of
card, a press opens a card, and the cards land in the same places every time
the map is opened.

### Features
- The canon as a graph - around a card, and the whole map

## [0.142.0] - 2026-11-18

The long card. A card's summary - its free note - now comes first, under
the name, and Summarize from the facts has the assistant propose one: a new
kind of action answer, kept with one press as the summary, shipped in every
profile with a canon, beside Gather facts from the summary. Sections fold by
the arrow beside their names, which also counts what they hold, with Fold
all and Unfold all; the folds are remembered for each kind of card, and the
list's groups fold too. A section of more than eight facts shows five and
Show all, a card of ten facts or more has a search and the state chips of
its own, and the fact editor takes a pasted list - one fact a line, bullets
and numbers taken off - added as one change an undo takes back.

### Features
- The long card - its summary on top, sections that fold, a search inside, facts by the list

## [0.141.0] - 2026-11-18

A canon for every craft. Novels, blogs and podcasts have a canon now, and
every canon a passport - World / Book, Publication / Blog with its personas,
topics and cases, Show / Podcast, and Studio's channel worded Project /
Brand - made with each new space and named after it. While the canon holds
nothing else, a quick start offers to fill the passport, make the first card
or set it up with the assistant. A card shows only the sections that hold
something, the rest under + Add section; the letters of the layers and the
words of the states appear only where they say something, and a fact's
layer, state, time and source wait under More options. The passport folds
its publication templates away, the lens moved into How the AI reads this,
and a line over a card counts the assistant's proposals with Review and
Accept all. Kinds of card and their sections can be turned off in the
profile editor.

### Bug Fixes
- The passport kilna made does not keep the first space from going

### Features
- A passport in every profile, made with the space, and a card that shows only what it holds

## [0.140.0] - 2026-11-18

Catalogue columns. The total now carries the tier it earned - its letter by
rank beside the score, the word, the day it was scored and the distance to
the next tier on hover, a turned arrow for a stale score - and its funnel
ticks the profile's tiers as ranges of the score. The next release carries
whether it can go out, as a dot before the day, and the planned slot when
nothing is booked; its funnel asks planned, overdue or no date. A press on
the id copies it whole. New columns: the collection, the tags, and what was
made from a song - a chip a kind that opens the work. A list of columns that
still names the retired tier, scored or ready columns reads without them.

### Features
- One total, one next release, and the collection, tags and made-from columns

## [0.139.0] - 2026-11-18

Marks and filters. A mark is raised where it is shown: the catalogue's cell
opens the card's menu over itself, and the bulk bar raises a mark on the
chosen works or takes it off them in one undoable change, with a dash where
only some carry it. Every shipped profile but the blank one now carries the
same eleven marks in both languages - in focus, next to go out, needs polish,
experiment, redo, on hold, calling card, for shorts among them - with
fourteen more glyphs, and a space made before gains them. Each column funnel
reads its ticks With or Without and Or or And; And is closed, and says why,
where a work holds one value; marks and tiers can be filtered by (empty).

### Features
- Marks raised from the cell and in a batch, eleven shipped marks
- Funnels read their ticks with or without, any or all

## [0.138.0] - 2026-11-18

Planned slots. A song goes out nowhere itself, so putting it on a day used to
be refused; now it is planned there - a dashed line on the calendar, warmer
once its time comes. Opening the slot makes it real in one press: an audio, a
video or a short made from the song with its release booked on the slot's
day, or the work's own release - one change an undo takes back. A slot is no
release: it counts toward no rhythm and asks nobody whether it went out, but
the guard of repeats reads its day. The Dashboard says a slot from the
profile's lead days, the bell warns once on its way and once when it is late,
and the slot closes by itself once something made from the work is booked
near its day. The assistant reads the slots through MCP.

### Bug Fixes
- The planned slot's types - the release queue's key and one day count
- A planned slot's lines belong to the calendar

### Features
- A planned slot of a work - one per work, moved, noted, made real, closed by one rule
- Planned slots on the calendar, made real in one press, reminded once

### Testing
- The planned slot is the space's forty-third table

## [0.137.0] - 2026-11-18

Starter packs. The snapshot of the style registry that ships with kilna now
holds thirty packs: sixteen sound packs in the spirit of an artist - the name
in the pack's title only, every phrase plain production tags - and picture
and motion packs written to the catalogue's detail with their contracts:
black and white, a photo studio, Soviet print, cinematic light, the city at
night, film stocks, stage looks, rain and snow, small movements of a hero.
Every pack installs into Studio as it is, and a registry keeper's tool writes
the digests.

### Features
- Thirty starter packs in the snapshot, sixteen in the spirit of an artist

## [0.136.0] - 2026-11-18

Frame and motion. A frame's loop is picked from the dictionary's Motion
branch - up to five movements, the starred first, a camera move marked while
the camera holds still - and built of the ones its picture can take. The
frame can have its own style and layers, starting from a copy of the cover's,
and uses the cover's layout list and scales with "No subject". A dock under
its prompts copies all three blocks and keeps what came back, a still or an
mp4 loop.

### Features
- Movements from the Motion branch, the frame's own bricks, scales and a dock

## [0.135.0] - 2026-11-18

The cover constructor, reworked. The starred ideas stand at the top of the
column as a list held there while it scrolls; every section folds to one line
of what is chosen in it, and the app remembers which are folded. Layouts are
searched and starred, and "No frame" is one press; how big the subject is and
how much of it shows are scales of named stops, the second starting at "No
subject". A section of pickers fills the layers - look, environment, angle,
pose, light, lens, grade - each with what its contract says beside it. The
lettering is one section, and a dock under the made column stands whatever
the scroll: Copy all, the second prompt, how fresh the prompt is, a check of
the prompt for contradictions by the assistant, and the results as a strip.

### Bug Fixes
- A compact result strip in the dock, stops of a scale on one line, one way to clear the frame

### Features
- A sticky shortlist, folding sections remembered across the app, and the layer pickers
- Size and shot on scales of named stops, the shot starting at no subject
- Layouts are searched, starred first and cleared in one press
- One lettering section - the title, where it goes, the second prompt and the typeface
- A standing dock with the prompt check, the result strip and freshness

## [0.134.0] - 2026-11-18

Brick contracts. A style can say when it can go into a picture and what it
brings to it - a pose that needs the hero to the waist, a grade that cannot
stand a black-and-white style, a camera move that cannot go into a frame
whose camera holds still - and one rule reads every chosen brick against
the picture. What the picture cannot take is left out of the prompt but
kept in the cover, and the constructor says why with one button that lets
it in. Covers gain nine layers - composition, environment, look, pose,
angle, layering, lighting, optics, grading - and Studio ships the light, the
lens and the grade. A frame's loop is built from up to five movements of the
Motion branch, a scene gets its own bricks and movements, and an agent can
propose styles of any type, with their contracts, over MCP.

### Bug Fixes
- The window's types for frames, problems and new bricks

### Features
- Brick contracts read by one resolver, nine layers, movements in loops and scenes

## [0.133.0] - 2026-11-18

Motion and heroes. The dictionary has a third branch beside the picture and
the sound: Motion, the movements of an animated frame - the atmosphere, the
subject, the world around and the camera - twenty of them shipped, each
saying what moves and what stays still. The heroes of the canon are shown
where the Character type is, with their portraits and layers, without a copy
in the dictionary; the cover's hero picker can show every layer, and a hero
picked by hand brings its bans to the cover whatever its layer.

### Features
- A Motion branch of the dictionary, and heroes mirrored from the canon

## [0.132.0] - 2026-11-18

Sets, stars and the switch. Every style kilna ships is now filed under a set
- lettering, dressings, colours and every phrase of sound included - and the
sets stand in a panel beside the dictionary with their counts and a switch
that takes a whole set out of every picker without touching its styles. A
card has a star that lifts a favorite to the top of the pickers and a switch
that takes one style out of them; "Retired" is "Off" now. In the sound, the
buttons sit together on the right, From your texts ticks every phrase at
once, and phrases that say nothing of what they mean can be explained in one
go - an answer about a style already in the dictionary fills it in rather
than making a second one. A pack's preview picture becomes the first picture
of the style it installs.

### Bug Fixes
- The sets are compact rows, not tiles
- The card's switch says its hint around it

### Features
- Every shipped style in a set, sets switched whole, stars and a switch on the card

## [0.131.0] - 2026-11-18

Style packs from a registry. The dictionary grows from themed packs - film
photography, light and angle, lo-fi textures, the sound of an era, camera
moves - read from a registry: a snapshot ships with kilna and works offline,
and folders and sites (GitHub Pages among them) are added in Settings. Get
styles on the Styles screen lists the packs with where each stands in the
space and installs a pack whole or one style of it. A pack's style is never
installed twice, takes a newer pack's words when nobody touched it, keeps
yours when you reworded it, and stays deleted when you deleted it - and
every install is in the history and undone like anything else.

### Bug Fixes
- A pack's source is a PackSource in the window, and the canon's Source is its own again
- A brick's source is optional in the window's types

### Features
- Packs of styles from a registry, installed whole or one at a time

## [0.130.0] - 2026-11-18

A demo and a second copy. A portable copy, or one started on another folder
with `--workspace`, is now an instance of its own and runs beside the
installed kilna; starting kilna again on the same folder still brings its
window forward. `kilna --demo-all` makes a demo stand - a space of every
craft, each with its demo - and makes it again on the next call, never in a
folder of real spaces; `kilna --seed-demo` fills one empty space. The demo
speaks Russian when the window does, and Settings -> Spaces can fill the
open space with it.

### Features
- One instance per folder of data, a demo stand, and the demo in Russian

### Testing
- The Russian demo's first title is read with its tie broken

## [0.129.0] - 2026-11-18

Onboarding. The first start of a new machine welcomes you in: whether
Claude Code is here, what your space is made of, its name and look, a demo
to look around in, then your first work, opened on its card. The demo - a
handful of made-up works at every stage with a score, notes and releases
on the coming days - is there for every craft and for any new space. Each
screen says what it is for on its first visit. Two new crafts ship:
Poetry and Newsletter, each with its own skills and playbook. And the
profile editor gains the canon's structure and the vibe's lines.

### Bug Fixes
- The list of coached screens stays inside the hint
- The first work reads the kind of a space that may have no profile yet
- A reading goes out under a glyph the window draws
- A demo books the piece itself where its kind goes out, and the welcome answers at its foot

### Features
- A welcome on the first start, and a demo to look around in
- A hint on each screen's first visit
- Poetry and Newsletter, two crafts of their own
- The canon's structure and the vibe's lines edited in the profile

## [0.128.0] - 2026-11-18

The box without an author. Every skill of Studio, Novel, Podcast and Blog now
says when to reach for it, in both languages, and has a method - sixteen got
theirs written. Each craft ships the playbook of its whole cycle as a method
an agent follows: song-workflow and ai-video-pipeline in Studio,
novel-workflow, podcast-workflow and blog-workflow. Studio's methods no longer
carry one author's examples or conventions: the critique judges against the
channel's card in the canon, the singer's stresses follow the register, and
the audio's titles ship with no tail. Spaces that changed any of it keep
their words; untouched skills take the new ones.

### Features
- The box without an author - a method and a trigger for every skill

## [0.127.0] - 2026-11-18

Methods over MCP. An agent outside the window now reads the craft from kilna
rather than from its own files: `workspace` gives each kind's axes whole,
rubrics included, and the index of the profile's skills; the new `guide`
tool opens a skill - its method, its message, the tool its answer goes
through and the live context of the space. Every skill is also an MCP
prompt, a slash command in the clients that offer them, filled from a work
when one is named. Settings -> Agents writes a short trigger for Claude
Code, Codex CLI, Gemini CLI or a Cursor project that sends the agent to
kilna, with no method inside it.

### Bug Fixes
- Prompts are answered from the space, as tools are

### Features
- Methods over MCP - guide, prompts and a thin trigger for agents

## [0.126.0] - 2026-11-18

Skills. The profile's actions get a section of their own, Settings -> Skills:
each is a button the window offers with the message it sends, or a method
only - a playbook the assistant and agents read, never a button on a card.
Find one by a word of its key, name, trigger or method, narrow by kind, what
it is about, button or method and where it stands against what kilna ships,
and edit it whole: its trigger, its group, every scope and answer, the
button switch, the method with a preview of what the assistant is handed.
Reset one to the standard, duplicate it, export and import the set. A count
beside Profile and Skills says how many newer standards wait to be compared.

### Bug Fixes
- The newer standards fold into one line in Skills

### Features
- Skills - a button or a method an agent reads, in a section of their own

### Testing
- The test backend answers what kilna ships with nothing waiting

## [0.125.0] - 2026-11-18

One rule for what kilna ships. The actions, statuses, stages, marks, fields,
kinds of note and their sections, and the vibe's words a profile was given
now follow a single rule at every start: an entry you never changed takes the
newer version, one you changed keeps your words with the newer version
offered beside it, one you deleted stays deleted, and one shipped since
arrives in its place. Settings -> Profile -> What kilna ships now shows what
waits, field by field, to keep or take; taking is an edit an undo brings
back. The first start meets every entry once and writes over nothing of
yours. The style dictionary's starter set follows the same rule, and the
profile module is split by what each part is for.

### Features
- One rule for what ships - given once, offered after

### Refactoring
- Split the profile module by what each part is for

## [0.124.0] - 2026-11-17

A work's vibe. What a work feels like is said along three lines - its mood,
its character, how complex it is - and drawn as one mark: the mood's colour,
the character's shape, as many strokes as it is complex. Say one, two or all
three, in the card's header beside the stage or in a new catalogue column;
a clip or a short with none of its own shows its song's. The catalogue
filters by it and the search box reads `mood:`, `character:` and
`complexity:`; a calendar's slot carries the mark. The words are the
profile's, and every space gets them. The assistant reads a vibe when it
scores a text and an agent may propose one - either is kept only where you
have said nothing. A free word in an older Mood field is left as it is.

### Features
- A vibe - mood, character and complexity - kept with a work
- A mark built of three lines, set on the card and in the catalogue
- An agent and a score may propose a vibe, never over the author's

## [0.123.0] - 2026-11-17

Scoring by itself. With "Score texts by themselves" on in Settings, a song's
text is scored when it has been left alone for a while, so the catalogue's
numbers describe the texts as they stand. Off by default: a score is a run,
and a run costs money. Only a written text of a work far enough along is
scored, a typo mended in a scored text is not a reason, and every edit
starts the silence over. It waits behind everything you ask for yourself and
never takes the last free slot. What it says waits as a proposal, or - if
you ask - is kept as the assistant's opinion. Your own score of a text now
always speaks over anyone else's, however much later theirs came. Turning it
on scores nothing that is already there; a button scores the unscored and
the stale.

### Features
- Score a text by itself once it has been left alone

## [0.122.0] - 2026-11-17

A viewer, and a menu on every file. A click on a file opens it inside kilna:
a picture fitted to the window and enlarged from there, a clip with its sound
and speed, a text to read and copy, a PDF, and for anything else what is
known of it; the arrows page through the files without leaving the window. A
sound still plays in the card's player, and the one playing is marked. A
right click on a file opens its menu: look at it, open it in its program,
show where it lies, save a copy, copy the path, make a picture the cover. A
file of the folder on disk can also be renamed - never onto a file that is
there - and moved to the system's recycle bin, which asks first.

### Features
- Look at a file inside kilna, and do things to it from a menu

## [0.121.0] - 2026-11-17

Any files. The Files tab takes any file, and as many as you pick or drop at
once: a sound, a clip, a text, subtitles, an archive, where it took pictures.
A tab with nothing attached draws the place to drop the first ones. Files are
filed as images, audio, video and other, and one bar arranges the attached
files and the folder on disk together: which categories show, gathered by
type or by folder, sorted by name, date, size or type, drawn as large icons,
tiles, a list or a table with the size and the date. A file says with a tag
that it is the cover, a candidate or the finished file.

### Features
- Any files, several at once, and four ways to look through them

## [0.120.0] - 2026-11-17

A folder by id. Every shipped profile names a work's folder on disk by the
work's id, one per work and flat, so renaming a work never loses sight of its
files. A folder is made when it is first needed: "Open the folder", on the
Files tab and in a catalogue row's menu, makes it and opens it, and "Make the
folder" is gone. A work that goes to the trash takes a folder with nothing in
it; a folder holding a file is never removed by anything. A space made before
this release keeps its templates and its folders as they are. To move them to
the new layout there is `kilna --rehome-media`: a plan and a report by
default, and with `--apply` a copy of the database, the files moved without
writing over any, and the templates switched last.

### Features
- A folder named by id, made when needed, and a planned move

### Testing
- Look for a work's folder where the shipped template puts it

## [0.119.0] - 2026-11-17

Five stages. A work stands at one of five stops - 0, 25, 50, 75, 100 - in
every craft, where there were seven and a work nobody had judged held none.
A new work starts at the first. When this release opens a workspace, every
work moves to the nearest stop (17 and 33 to 25, 67 and 83 to 75) and one
with no stage to the first; a stored profile takes the five stops, each with
its own colour. The dial is an empty ring, an arc of a quarter, a half and
three quarters, and the whole ring; no word stands beside one anywhere, and
what a stop is called is said when it is pointed at. The picker, the
overview's widget and the stage filter are the five dials in a row, without
"Clear". `stage:` finds a stop by its key, its words or its number, in
English and Russian.

### Features
- Five stops for every craft, and no work without one

### Breaking Changes
- `Work.stage`, `ScoredWork.stage`, `Publication.stage`
and `ScheduledRelease.work_stage` are numbers, never null; a profile's
`stages` must stand at 0, 25, 50, 75 or 100.

## [0.118.0] - 2026-11-17

Focus, spacing, scroll. A field's focus ring is drawn inside its box and a
region that scrolls leaves room for one, so a ring is no longer cut by the
edge of a widget, a tab or a dialog. Every dialog stands its parts one step
apart. Where a comment was written is a field with a list of the channels
already used, in place of a row of buttons under it, and a new work of a
trial picks its kind in a select. The assistant's drawer lets go of the
pointer: the bar of a long conversation is dragged and text is selected,
where a press anywhere in the panel used to start a swipe to close it. A
task's chat says its action in the window's language, in chats made before
this release too, and two dialogs that read "[object Object]" say the word.

### Features
- Keep focus rings whole, one step in dialogs, a drawer that lets go of the pointer

## [0.117.0] - 2026-11-17

A card without leftovers. A work's card shows the tabs its kind has something
for. The Conception tab stands for a kind prepared on a conception, and for a
work that already holds a note; a clip, an audio release and a short open
without it. Comments are a tab of what goes out: a song and an experiment
have none, and a song no longer sums up its publications' comments - they are
on each publication and on the Comments screen. An experiment's Links appear
with its first link. A song's own facts - tempo, key, mood, vocal, language,
premise - stand on the song's card alone, not on its clip, its audio release
and every short, and an audio release names no version roles; a stored Studio
profile still as it shipped follows, and what you changed stays yours. A slot
that went out says so in its dialog and always leads to the work, link or no
link. The Release and Repeats widgets stand at their natural height, and the
task preview scrolls once.

### Features
- Drop the tabs and fields a kind has nothing for

### Testing
- Give the checklist's audio kind a role of its own
- An audio release names no roles and none of its song's fields

## [0.116.0] - 2026-11-17

An agent that mends, not only adds. Through `kilna --mcp` a note that is
there can be changed: `propose_note` names it by id or exact title, and the
person reads was against becomes before agreeing; several notes come as one
proposal, kept whole or note by note, with tags and a layer. A word the
record already keeps is changed by `propose_words` instead of being added
to in silence - strictness, forms, kind, topic, note - with each change
said, and a field the register would not take said too; a word entered
wrongly is taken out into the trash, and two entered as one are split into
their parts, each with what the one held. A new word written with a comma
or a slash is warned about. The long readings - notes, the register, an
experiment's trials, the catalogue - come by the page, with a total and
whether more follow, in a short form without the texts, and one entry whole
by its id (ADR 0085).

### Bug Fixes
- Let a proposed note's words be selected

### Features
- Read notes, the register, trials and the catalogue a page at a time, short or one whole
- Change a note and a word that are there, take a word out, split one

### Testing
- Hand a stored proposal to the chat as the JSON it is

## [0.115.0] - 2026-11-17

A prompt without ghosts. A cover's frame places a subject, not a hero: the
picture is of a person, a thing or nobody, and a cover with no hero never
says hero, head, chest, face, figure, eyes or skin - not in the framing, not
in where the title is set (ADR 0084). The frame is said once, as one FRAMING
paragraph of geometry, without the motifs every layout used to add. The
accent is a colour unless a disc is switched on; a background is the
picture's palette unless taken as a flat ground with no scenery. A detail of
the channel with no English for a generator and a lettering on a cover with
no title are left out and said in the tab; an empty slot in brackets is no
longer reported; a title sent to a second prompt keeps no room in the first
unless asked. The frame under a track and a scene of a clip follow the same
rules, a frame's own words are said with the scene, and a close look is
drawn inside the scheme. Ten prompts are kept whole in the repository, so a
reworded sentence shows as a difference before it ships. Covers built before
this read differently; the prompt last copied out stays on the cover.

### Features
- Place a subject, not a hero, and say geometry alone

## [0.114.0] - 2026-11-17

A safety net. kilna keeps a copy of a space's database by itself: once a day,
before a newer version changes its shape, and before a backup is put back.
A backup goes back from Settings, without a restart and without the four
steps by hand: it is checked, what stands is kept first, and SQLite restores
it into the open database (ADR 0083). One gesture is one undo: a package
kept, a clip made from a song, a section of the canon settled are taken back
whole by one press, and a rebuild from the log now replays an undo as it was
taken (ADR 0082). A file taken off a work, a style or a card goes to the
trash with its bytes instead of being removed for good. Settings, Data says
what a space takes on disk and compacts the database, examines the space -
damage, lost and stray files, plugins, the CLI - without changing anything,
and sets how long read history is kept; what leaves the feed goes to a page
of its month in the space's history folder first.

### Bug Fixes
- Name the area of a file taken off its owner

### Features
- Take a gesture back whole, and replay an undo
- Send a file taken off its owner to the trash with its bytes
- Keep copies of a space, restore one in place, measure and examine it

## [0.113.0] - 2026-11-17

Out and in. A work leaves by itself: its card's menu saves or copies its
page, the one the export writes, and packs it into a `.kilna.zip` with its
files. And works arrive: New, Settings and the palette open one dialog that
takes files, a folder or a pasted text, lists what it found and where each
stands, and writes nothing until it is told to. A page kilna wrote and a
parcel come back as the works they were - versions with their numbers and
lineage, scores, release, notes, the board with its frames - under the ids
and dates they were made with (ADR 0081); any other `.md` or `.txt` becomes a
work of the kind named, with the text as its first version. The export is
format 8: a version's body stands between two marks and the page ends with
the work's parcel, so a page exported, brought into an empty space and
exported again is the same page to the byte. Bringing in only adds - a work
already here, or deleted here, is passed over - and is one step for Undo.

### Bug Fixes
- Start a text from outside with its kind's fields and code

### Features
- Take a work out as a page or a parcel and bring works in

## [0.112.0] - 2026-11-16

Profiles, whole. Settings → Profile edits the whole document, laid out in
parts - the craft, each kind of work, the fields, the actions. Kinds of work,
axes, tiers, statuses, roles, kinds of release, fields, flags and the stops of
the dial are added under a key, taken out and dragged into order; an axis has
its scale, its kind of answer, what it asks, the answers of a choice and its
rubric; a kind of release weighs the axes its own way; an action can be about,
and become, everything the backend reads. A word is edited in the window's
language and keeps its other one. What would not save is said as it is typed,
beside the row it is about, and Save waits for it: the validator's hundred and
fifty sentences travel as codes with their place (ADR 0080), so a Russian
window reads them in Russian - as it now reads why a plugin cannot run, and
the body of a proposal an agent sent. A profile is a document that travels
(ADR 0079): written to a file, read back as a draft, and kept as a type new
spaces are made of - with an empty type shipped for a craft kilna does not
ship. Blog, Novel and Podcast speak Russian whole, as Studio does, and a
stored word still reading as it shipped in English gains its Russian at
start. A field can be a list - the anchors of an experiment are one; a scene
has a layer beside its kind of shot; an entry shipped later stands after its
shipped neighbour in every list; the critiques read the version before and
the last score, and Studio gains "Place in the album", which reads the
collection.

### Bug Fixes
- Give an axis's word the width its numbers took
- Name a glyph for the empty profile's release

### Features
- Edit, check, keep and move the whole profile

### Testing
- Run the upgrade on a real space, read a plugin's reason as a key

## [0.111.0] - 2026-11-15

The calendar and the queue. A clip, a short and an audio release carry the
tier of the song they are made from, so the plan is read by strength again -
and a month can be held to goals by tier or by kind of work
(`rhythm.monthly`), with the dashboard saying which one it falls short of.
The day's plus chooses a work that exists and says what its song would repeat
on that day before it books; a work carried from the pane is told the same.
Carrying a chip no longer waits for the guard of repeats: the workspace is
read once and each day answered from that reading (ADR 0078), a day is asked
about only once the pointer rests on it, and the date is written first - on
the owner's workspace seven days cost five seconds, and now cost one reading.
The queue narrows by the catalogue's stage funnel, "no stage" included; the
catalogue slices by repeats, all of them or the red ones. Plans whose day has
passed are answered together on the dashboard - marked as out on their own
days, or taken off the calendar - and a "not ready" line of the history says
what is missing. Underneath, dowel 1.0: the week is BigCalendar's, the
catalogue's headings sort through TableSortButton, every pressable picture
and card is a TileButton, and the registry check has no exceptions left.

### Bug Fixes
- Warn of the first match while the highlight catches up

### Features
- Plan by the song's tier and ask a day once per carry
- Pick, filter and answer the plan where it is read

### Refactoring
- Take the shapes dowel 1.0 built for kilna

## [0.110.0] - 2026-11-13

dowel's primitives, taken from the registry instead of drawn by hand. The
month is dowel's BigCalendar, and every release on the calendar - the month,
the week, the agenda and the copy under the pointer - is one line of
BigCalendarItem: 24 pixels, the work's colour, and a release that went out
drawn with a weaker fill rather than faded. The month is one stop for Tab, and
its week starts on Monday in English too. The keyboard, the search box and the
sheet of shortcuts read one list of commands, so `G` and a screen's letter work
on any layout and `?` lists exactly what answers. A work's fields are drawn
from the schema its profile describes: a date can be cleared, a yes or no is a
switch. The frame of every screen is dowel's, and the window is at least 960
pixels wide, so a list always stands beside what is open in it. No screen
skips a level of heading.

### Bug Fixes
- Sort the stop words with a slice swap
- Take out the sheet's words nothing asks for any more
- Let no screen skip a level of heading

### Features
- Draw the month on dowel's BigCalendar
- Draw a work's fields from the schema its profile describes

### Refactoring
- Take Frame, Pane and ListDetail from dowel's registry
- Declare the shell's keys on dowel's list of commands
- Scroll the open note in the text scroller a version uses

## [0.109.0] - 2026-11-10

Spaces: each channel, blog or book is a folder with a database of its own -
its works, calendar, canon, register of repeats, history and trash - and the
window switches between them without a restart (ADR 0077). The first start
moves the old database into spaces once, keeping a copy of it beside it as
`kilna.before-spaces.<moment>.db`, and asks each new space its name. Settings
fall in two, the application's and the open space's; the assistant's settings
and the week's budget are the application's, counted across every space. A
space opens by double-clicking its `space.kilna` or by a
`kilna://<space>/work/<id>` link, is read only while another machine holds it,
and a copy of kilna with `kilna.portable` beside it keeps everything beside it.

### Bug Fixes
- Leave a window closed to the tray behind it at start
- Give a release marked out of the queue its day
- Score a work against a draft only
- Hold a number setting to its step
- Say a card's other names only through the work's lens
- Offer as heroes only the cards a work shows
- Make a new draft current only of its own role
- Read letter shortcuts by the place of their key
- Open the version a search found, not the newest
- Keep fonts out of the bundle's data URLs
- Part the space's settings from the application's

### Documentation
- Say what a space is and where its files live

### Features
- Keep each space in a folder with a database of its own
- Switch spaces from the menu and keep them in settings

### Testing
- Give the profile isolation tests a second profile of their own
- Pass over the start's link kept in the application's state

### Breaking Changes
- Profiles are no longer switched inside one database;
activate_profile and list_profiles are gone, the assistant's settings
moved to app.db, and kilna --mcp --workspace names an application
directory.

## [0.108.1] - 2026-11-08

A refactoring release: nothing a user sees has changed. The Rust tests no longer
migrate a database of their own each (ADR 0076).

| Measured | 0.108.0 | 0.108.1 |
| --- | --- | --- |
| Lib tests on CI, ubuntu / windows / macOS | 595 s / 471 s / 911 s | 32-41 s / 40 s / 45 s |
| All Rust test binaries on CI, ubuntu | 636 s | 47 s |
| CI test step, ubuntu / windows / macOS | 11:35 / 10:03 / 18:11 | 1:55 / 2:16 / 1:33 |
| Opening a test workspace in memory (debug) | 320 ms | 7 ms |
| Seeding the shipped profiles (debug) | 35 ms | 11 ms |
| 104 tests of note, profile config and score, one thread | 28.5 s | 3.4 s |
| 60 tests of the assistant's runs, one thread | 15.3 s\* | 6.3 s |

\* With the in-memory step already in: what is left is their workspaces on disk.
CI timings vary by runner: the same commit's lib tests took 41.6 s and 69.0 s on
two runs, so each figure above is the run read, not a promise.

### Documentation
- Say why a test workspace is a copy of one migrated database

### Refactoring
- Copy one migrated database instead of migrating each in-memory workspace
- Read the shipped profiles once per seed
- Parse the shipped profiles once per process
- Read a stored document's format without building it
- Write a workspace file from the migrated copy

### Testing
- Pin what an in-memory workspace starts from
- Pin how a stored document's format is read
- Open workspaces on disk from the migrated copy

## [0.108.0] - 2026-11-05

### Bug Fixes
- Ask only the plugin meant, and the rest at once
- Let a head with nothing in it cost no gap
- Hand the example's run its settings

### Documentation
- Say the assistant's settings on the front page

### Features
- Set up the assistant and the plugins from Settings
- Draw the engine's overlay scrollbar through dowel-ui 0.43

### Testing
- Count the table a plugin's settings are kept in

## [0.107.0] - 2026-11-03

### Bug Fixes
- Tell a hidden window by the system's word, and say what the calendar put on tomorrow
- Let a weekly budget off the whole-dollar grid be saved
- Give the refusals of a budget and a moment their words

### CI
- Set up the newest Node 22 the docs' Astro asks for

### Documentation
- Say the tray, the notifications and the price of a run on the front page

### Features
- Keep work going behind the tray, say outside the window what ended, and price every run

### Testing
- Read a formatted hole by its name

## [0.106.0] - 2026-11-01

### Bug Fixes
- Open a starred answer at its first line

### Features
- Read the score, the version before, the collection and the week
- Star answers and save a chat as Markdown
- Show what an answer changes before it is inserted

## [0.105.0] - 2026-10-27

### Bug Fixes
- Say the refusal of a week that starts on no day

### Documentation
- Describe a line's day and work, Today, the visit and the week

### Features
- Count a week from the operations log and the visit since the history was seen
- Give a line a day and a work, and say it on its day
- Count the visit in one line and save the week as Markdown

### Refactoring
- Add days through lib/month rather than in each screen

## [0.104.0] - 2026-10-27

### Bug Fixes
- Restore the line continuation in the deadline query
- Say a month short of the rhythm once the queue is known

### Documentation
- Describe rhythm gaps, deadlines in the layout, CSV plans and calendar files

### Features
- Lay out a collection due by a day first
- Read a release plan from a CSV file into the layout's preview
- Say a month short of the rhythm, and save the month and the calendar as files

## [0.103.0] - 2026-10-27

### Documentation
- Describe where to cut from, the suggestions reply and ADR 0071

### Features
- Keep what a plugin heard beside a video and offer where to cut from
- Mark where a video's parts begin, land edges on them and take a part

## [0.102.0] - 2026-10-27

### Bug Fixes
- Stop what a plugin started along with it
- Name the file a finished cut made the work's finished file

### Documentation
- Add the guide to cutting a short with kilna-plugin-cut
- Name the cutting plugin where the core hands it the shot list

## [0.101.0] - 2026-10-27

### Documentation
- Describe the finished file, {n}, the cutter protocol and ADR 0070

### Features
- Know the file a work is finished as, and number a work in its family
- Hand a cutter the whole short, with time, progress and a stop

### Refactoring
- Spell the refusal of a path outside the folder where the refusal gate reads it

### Testing
- Write the Studio fixture again for its folders and rules
- Lay a workspace's kinds out by hand, as the disk has them

## [0.100.3] - 2026-10-25

### Bug Fixes
- Write another take when a release already has words
- Let the style prompt a song is made with be chosen again

## [0.100.2] - 2026-10-25

### Bug Fixes
- Hand the generator plain quotation marks

### Documentation
- Describe drawn pools, titles for later and ADR 0068

### Features
- Draw a pool into a prompt and settle a section at once
- Keep titles for later and write meta from drawn moves

## [0.100.1] - 2026-10-23

### Bug Fixes
- Write release meta without one shape for every release

### Documentation
- Describe the premiere tail, the excerpt and ADR 0067

### Features
- Title the release that opens a song as its premiere

## [0.100.0] - 2026-10-21

### Bug Fixes
- Order where an entry worked by title and rowid, not a uuid
- Keep a translation out of the quote, show hints as examples

### Documentation
- Describe the research, the library and ADR 0066

### Features
- Research a work on a board linked to a library

## [0.99.0] - 2026-10-20

### Bug Fixes
- Read a brief before typing into it; trim the order
- Find the assets from the repository, not this machine's paths
- Take dowel-ui 0.38.1's copies of the registry components
- Let the brief's skeleton be selected and copied

### Documentation
- Describe the conception, its boards and the order
- Part every release from the one above it
- Show the conception on the front page

### Features
- Prepare a work on a brief, an idea and boards

### Testing
- Hold harvests and the dialog to the rule that drafts are current

## [0.98.0] - 2026-10-19

### Bug Fixes
- Give a deleted shelf its look in the history
- Open the Notes screen on the phrases in an older workspace too
- Call a tag a tag in Russian, not a mark

### Documentation
- Two shelves - phrases on shelves, notes, ADR 0064

### Features
- Keep material on two shelves - phrases and notes
- Rename a tag on every note, or take it off them all

### Performance
- Find a row of the index by its key, not by reading it all

### Testing
- Hold the promotion that copies nothing in replay and undo
- Order the keyed index by rowid, which the ordering rule reads as an order

## [0.97.0] - 2026-10-14

### Features
- Check what a place needs, read links, zone the hour

### Testing
- Run the window's tests on four workers

## [0.96.0] - 2026-10-11

### Features
- Close days off, shift the plan, read it by week or agenda

## [0.95.2] - 2026-10-10

### Bug Fixes
- Read a stored proposal through today's type
- Say how many trials a proposed work brings

## [0.95.1] - 2026-10-09

### Bug Fixes
- Keep an anchor by its words in any order

## [0.95.0] - 2026-10-09

### Bug Fixes
- Take the first match of a typed name with Enter

### Features
- Experiments with a board of trials

## [0.94.1] - 2026-10-09

### Bug Fixes
- Carry the House sound section into a stored channel card

## [0.94.0] - 2026-10-09

### Features
- Write a song's style from a dictionary of sound

## [0.93.1] - 2026-10-09

### Features
- Number works by a code their profile gives

## [0.93.0] - 2026-10-08

### Features
- See a work's folder on disk, play its sounds, paste pictures into notes

## [0.92.0] - 2026-10-08

### Features
- Give collections a screen, a goal and a verdict

## [0.91.1] - 2026-10-07

### Bug Fixes
- Promise the collections screen to v0.92

## [0.91.0] - 2026-10-07

### Features
- Draw the tree of versions and the history on one axis

## [0.90.3] - 2026-10-07

### Features
- Add accents and gradient grounds, lettering from references

## [0.90.2] - 2026-10-07

### Features
- Ship eastern, folk, ancient and trending styles

## [0.90.1] - 2026-10-02

### Bug Fixes
- Let every widget scroll itself and the wheel reach every list

### Features
- Open each kind of work on its own tab
- Draw marks as icon tiles named on hover

## [0.90.0] - 2026-10-02

### Features
- Release a publication once and keep its words in check

### Testing
- Compare the media directory by its real path

## [0.89.3] - 2026-10-01

### Bug Fixes
- Show a package of pictures and descriptions on its cards

## [0.89.2] - 2026-10-01

### Features
- Let an agent propose a card's pictures and description

## [0.89.1] - 2026-10-01

### Features
- Let an idea keep out what its scene risks

## [0.89.0] - 2026-10-01

### Features
- A board of ideas for a publication's cover

## [0.88.0] - 2026-10-01

### Bug Fixes
- Fit the stage's scheme and export the mark for real

### Features
- Build a publication's cover from a concept

## [0.87.0] - 2026-10-01

### Features
- Ship a starter set and say where a style came from

## [0.86.0] - 2026-09-30

### Features
- A song goes out as what is made from it
- Weigh an unjudged publication by its song
- A song's card lists what goes out for it

### Refactoring
- Read works through their publications

## [0.85.0] - 2026-09-30

### Build
- Raise the MSRV to 1.90 and take Tauri 2.12

### Features
- A register of repeats, and ideas and phrases as material

## [0.84.0] - 2026-09-30

### Features
- Cards of facts for the world the works share

## [0.83.0] - 2026-09-28

### Refactoring
- One function per gesture
- Every refusal is a code the window says
- Generate the window's types from Rust

## [0.82.0] - 2026-09-28

### Features
- The overview as a board of widgets

## [0.81.0] - 2026-09-28

### Features
- Scenes and the assistant to the mockup

## [0.80.0] - 2026-09-28

### Features
- The card to the mockup

## [0.79.0] - 2026-09-28

### Features
- Shell screens to the mockup
- Read a style's and a comment's task before it is sent

## [0.78.0] - 2026-09-28

### Bug Fixes
- Hide Base UI's own scrollbars under the window's CSP

### Build
- Import package.json with a JSON import attribute

### Refactoring
- Set every size on the dowel scale
- Hold every screen on one frame and dowel's primitives

## [0.77.0] - 2026-09-27

### Bug Fixes
- Close the runs of spaces left inside backend sentences

### Features
- Keep a style's description from the answer that wrote it
- Say numbers, dates and durations in the window's language

### Refactoring
- Keep only registry copies in components/ui
- Drop the column list carried over from the machine
- Remove what nothing calls
- Lay the frontend out by feature
- One list of screens and one list of tabs
- Split the wire to the backend by domain
- Query factories, write sets and one mutation hook
- Read through factories, write through one hook, hear runs once

### Testing
- Mount components against a mocked backend
- Find backend code by markers, not by file paths
- Hold the window's rules on what the frontend draws
- Hold the window and the backend to one list of commands
- Read past the pipe without racing the suite
- Find the window's unions by their declaration

## [0.76.2] - 2026-09-25

### Features
- Pin a dialog's actions and make the window compact

## [0.76.1] - 2026-09-24

### Bug Fixes
- Send every deletion to the trash and every write to the log

## [0.76.0] - 2026-09-23

### Features
- A screen for every note, with checklists and a way to grow into a work
- An inbox for the audience, read from screenshots and answered in the channel's voice

## [0.75.3] - 2026-09-22

### Bug Fixes
- Raise the minimum Rust to 1.87

## [0.75.2] - 2026-09-22

### Bug Fixes
- Name each screen once, and give every trashed kind a word

### Features
- Stand on dowel's frame, and fold the menu to icons
- A header that stands, and two columns for versions and scores

## [0.75.1] - 2026-09-20

### Features
- Pick the parts before the prompt is built

## [0.75.0] - 2026-09-20

### Features
- One dictionary of the parts a prompt is built from
- A screen for the dictionary, and links that resolve

### Bug Fixes
- Say whether there are references, and stop doubling cancel

## [0.74.4] - 2026-09-20

### Bug Fixes
- Check colour tokens where the theme is installed

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
