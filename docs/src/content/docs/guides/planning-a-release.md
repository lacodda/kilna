---
title: Planning a release
description: Queue, slots and the auto-layout — how a release gets a date, how a month is read at a glance, and how one click paces the whole queue to your rhythm.
---

A **release** is a plan to ship a specific work through some place — YouTube,
the streaming services, a newsletter send, a feed — on some date, or with no
date yet at all. Creating a release doesn't schedule it; scheduling is a
separate step, and it's the one where kilna pushes back.

**A publication goes out once.** A work has one release: the clip on
YouTube is one publication, the same clip as a premiere is another, each with
its own title, cover, comments and numbers. Where it goes out is chosen when
it is made (**Make… → Audio → Streaming**) and can change until it goes;
once it went out, it went out there.

**A door belongs to the work that goes through it.** In Studio a song has no
door at all: it goes out as what is [made from it](/kilna/guides/made-from/).
Its audio release is an **Audio** work - the track under one picture - going
out on YouTube (and, when you add one, on streaming); a clip is a Video work
going out on YouTube; a [short](/kilna/guides/shorts/) is a Short work going
out through its own door. The song's overview lists them with where each
stands; the calendar shows each of them once, on the work it is.

:::note[This changed in v0.74]
Until then a song had three doors — clip, short and audio — and a clip
release on the song said "this goes out as a video" without being able to
say which video. The first time a workspace opened on v0.74, every clip and
short release of a song was moved onto a Video or Short work of its own,
made from the song as its donor, with the song's title and overview fields.
No date, link or text was lost: the release kept its slot, its pin, the day
it went out, its link and everything written under it — only the work it
hangs on and the door it goes through changed. History says how many moved.
See [ADR 0030](https://github.com/lacodda/kilna/blob/main/docs/adr/0030-a-door-belongs-to-the-work-that-goes-through-it.md).
:::

:::note[And in v0.90]
A work that held two releases was split the first time v0.90 opened it: the
work kept its first release, and each other one became a publication of its
own — the same kind, fields, cover and frame, made from the same song, its
files with it. A release's own title, which no screen showed any more, moved
into its title field where that was empty, and was kept beside it as
`former_title` where the field already said something else. See
[ADR 0051](https://github.com/lacodda/kilna/blob/main/docs/adr/0051-a-publication-goes-out-once.md).
:::

:::note[And again in v0.86]
The song's last door, the audio release, came off it too. The first time a
workspace opened on v0.86, every audio release planned on a song moved onto
the song's **Audio** work - one already made from it, a video named after the
song and the audio release (*"Harbour lights — audio release"*), which became
an Audio work keeping its versions and releases, or a new one titled as the
song - going out through YouTube from there, with every day, link and word it
had. Clips named the way kilna names them and made before a link could say so
were linked to their songs. An Audio work left holding two releases through
one door - a plan and the upload it became - is named in History for you to
keep one. Then every song's status was read again through its publications.
See [ADR 0047](https://github.com/lacodda/kilna/blob/main/docs/adr/0047-a-song-goes-out-as-what-is-made-from-it.md).
:::

## The queue

A release with no `scheduled_at` sits in the **queue** — planned, but not yet
claiming a calendar date. The queue is sorted strongest first, by each
release's work's score, with unscored releases sorting last. This is where you
look when deciding what deserves the next open date.

**Which score** is the one the catalogue shows: the current version's, or the
strongest if no version is current — see
[Scoring](/kilna/concepts/scoring/). The queue, the catalogue and the calendar
all read that one number: until v0.24 this screen used the most recent snapshot
instead, and a work showed one figure here and another there.

## The rhythm and the auto-layout

The profile can state a [rhythm](/kilna/reference/profile-document/#rhythm) —
how many days the craft keeps between releases, and optionally the usual time
of day, which is shown beside the date when editing a release. With a rhythm
set, **Lay out the queue** plans a date for everything waiting, in one click;
without one, the button explains what to set first.

The plan is a **preview, not a booking**. Ghost chips — dashed, in each
work's own colour — show where every queued release would land, and a bar
above the grid says how many and between which dates. Nothing is written
until you approve it; cancelling leaves the calendar untouched. Approving
books **exactly the previewed plan**: if the calendar changed in between — a
slot claimed by hand, a release scheduled from its card — the whole plan is
refused rather than partially applied, and you preview again from what the
calendar holds now.

The layout follows four rules, in this order:

- **Nothing already on the calendar moves.** Booked days, pinned or not, are
  ground the layout builds around — it fills empty days only, and never puts
  anything beside what is already there.
- **Spacing.** A planned date keeps at least the rhythm's distance from every
  release that has a date — planned or already released. Something that went
  out yesterday sets the pace exactly as a booked slot would.
- **Scatter.** Two releases of the same work never land on neighbouring days.
  With a rhythm of two days or more, spacing already guarantees this; on a
  daily rhythm it is what keeps one work from occupying a whole week.
- **Order.** The queue is walked strongest first, each release taking the
  earliest day the rules allow.

The same inputs always produce the same plan, and placement starts tomorrow —
today is already underway. Applying writes one line to
[History](/kilna/guides/the-history/) for the whole batch, and the works'
[statuses](/kilna/guides/statuses/) catch up silently, the way the mass
resync does it.

## The month

The calendar draws one month at a time, weeks starting on Monday, with the
neighbouring days dimmed at either end so the grid stays rectangular. The
arrows walk it in both directions — work is planned ahead and reviewed
behind — and **This month** comes back to today, whose cell is outlined in the
accent colour.

Each booked release shows as a chip in its day, in the colour the work carries
everywhere else in kilna. The chip's top line holds the grip, the
[ready marks](#ready-marks), the lock if the date is pinned, and the glyph for
the kind of release; the title gets the day's full width underneath. Once
the profile has more than one kind of work, the top line also names the kind
of the work — *Video*, *Song* — because two kinds of work may ship the same
kind of release under the same glyph, and a video's YouTube release should
not read as a song's audio release. The dashboard's week says the same beside
each release.
Already-released chips are dimmed: they are history sitting on a date, not a
plan competing for one.

Which glyph stands for which kind comes from the profile — kilna does not know
whether your craft ships clips or beta reads, so the profile says both what the
kinds are and what they look like. See
[kind glyphs](/kilna/reference/profile-document/#kind-glyphs). Hovering the
title opens a card with the work's score, its tier, what it still needs and the
link it went out on, if it has one — everything the release dialog shows, one
hover earlier.

**Filtering by kind.** Above the grid sits a row of chips: one per kind of
release the calendar holds, plus **All**, each with the count behind it.
Choosing one shows only those releases; choosing it again goes back to all of
them. The row doubles as the legend for the glyphs on the chips, and it stays
out of the way when the calendar holds only one kind of release. The queue is
not filtered with it: the queue is what still needs a date, and hiding part of
it would hide work waiting to be scheduled. A
[layout preview](#the-rhythm-and-the-auto-layout) on screen is narrowed to
match, so the plan never shows what the month is hiding — booking still books
every placement, filter or no filter.

**Picking a release in the queue turns the grid into a way to answer "when".**
Days become clickable; clicking one gives the release that date. Clicking a
chip instead opens the release itself.

## Ready marks

Every chip and every queue row carries a small readiness row, answering the
one question a calendar is read for: **can this actually go out?**

- **Two ticks** — it already went out.
- **One tick** — everything is there: the work has a version for every role
  this kind of release requires, and a score speaks for it.
- **Otherwise, one glyph per gap** — a page for a missing version role, a
  gauge for a missing score. Hover for the list in words; opening the release
  spells the same list out.

Which roles a release needs comes from the profile: each release kind lists
the version roles it *requires* — a video's YouTube release needs a plot, a
beta read needs the text; an audio release requires none of its own.
A role the kind does not require is neither shown nor counted: *not
applicable* and *missing* are different answers, and only the second one
blocks readiness. A kind that lists no requirements is judged on the score
alone.

**The colour belongs to the deadline, not to the gap.** The same missing
style prompt is a quiet grey note in the queue or a month out, amber inside a
week, and red two days before the slot. What changed is not the work — it's
how much time is left to do it.

At the same distance the amber starts, the journal starts talking: a release
due inside the coming week that is not ready writes a warning to
[History](/kilna/guides/the-history/) — once per release and date, checked at
startup and after every calendar change.

## Moving a release

**Pick a chip up anywhere on it** and carry it to another day. The chip itself
travels under the pointer — title, marks and all — and the day underneath
lights up as you cross it. A press only becomes a drag once the pointer has
moved a little way, so a click still opens the release; the grip on the left
stays as the sign that a chip can be moved at all.

**Carry it to either edge and the month turns.** Rest the pointer in the strip
down the left or right side of the grid and the calendar walks back or forward
a month at a time, so a date in April is reachable from January without putting
anything down. **Escape** puts the chip back where it was.

Dropping on another day writes that date. Nothing is refused and nothing is
evicted: a day holds as many releases as you put on it. Dropping on the
**bin** — which appears at the top only while something is in the air —
returns it to the queue, exactly as unscheduling does.

**A day already holding something says so** while a release hovers over it, and
the drop goes through anyway. It is a note about what is there, not a warning
about what will fail.

:::note[This changed in v0.44]
Until then a day held exactly one release, and dropping onto a taken one
started a contest: the stronger work kept the date and the weaker went back to
the queue. The only thing that ever asked for a date was a person pointing at a
day, and a rule that argues with a deliberate gesture reads as a fault rather
than as care — so the contest was retired. What remains of it is the pin, with
a narrower promise: see [Keeping a date](#keeping-a-date).
:::

A day with more than two releases shows the first two and folds the rest into
**+N more**; clicking it opens the day, and **Show fewer** closes it again.

## Keeping a date

Some dates are decided rather than proposed: an announced launch, a slot
booked around something else, a release someone is waiting on. Open the
release and tick **Keep this date**.

**A pin is a message to the auto-layout.** It never puts anything on a pinned
day, so the date stays yours while the queue is laid out around it. The lock on
the chip is how you see it from the month view.

It does not stop you. Dropping a second release onto a pinned day works like
any other drop — you can see the lock, and meaning it is the whole point of a
gesture. Until v0.44 the pin also refused other releases outright, because
scheduling was a contest; now that nothing contests a date, that half of the
promise has nothing to refuse.

The pin belongs to the date, so losing the date loses the pin: unscheduling or
clearing the date drops it, and pinning a release that holds no date is
refused.

## Claiming a slot

A calendar **slot** is a date, and a date holds as many releases as you put on
it. Scheduling a release for a day — from the queue, by dragging a chip, or by
typing the date into the release itself — writes that date. Nothing is
compared, nothing is refused, and nothing already there is moved.

Two exceptions to "nothing else happens", both about the queue rather than the
date: a release that had no date leaves the queue when it gains one, and a
release that loses its date returns to it.

:::note[This changed in v0.44]
A slot used to hold exactly one release. Claiming a taken one started a
contest — the stronger work kept the date, the weaker returned to the queue
with its plan intact, a tie went to whoever was already there, and an unscored
release could never take a date from a scored one. The rule was retired when
it became clear the only thing it ever argued with was a person choosing a
date deliberately. The score still orders the queue and still drives the
auto-layout; it just no longer decides who may have a day.
:::

## Rescheduling and released slots

Scheduling a release into the day it already holds writes the same date again
and changes nothing else. Once a release has been marked **released**, its
date is history rather than a plan — it stays on the calendar, dimmed, and
takes part in nothing.

## Marking released

When a release actually goes out, mark it **released**, optionally attaching
the link it shipped under. This is a state change kilna records, not an
action it performs — nothing is published, uploaded, or posted from here.
Marking a release without a link a second time keeps whatever link was
already recorded rather than clearing it.

**The day is yours to name.** Marking — from the calendar or from the
**Release** block of the work's overview, the same dialog either way — asks
for the day it went out, which starts from the day it was planned for and can be changed. Marks are often made after the fact — days later, or while entering
something that shipped long ago — and a mark that could only ever say "now"
made every late one quietly wrong.

**A mark can be taken back.** *It did not go out* returns the release to
planned and clears the day it shipped, leaving the work's
[status](/kilna/guides/statuses/) to follow on its own. **The link is kept**:
an undone mark is usually a mis-click or a release pulled after the fact, and
the address it was published under is the one part worth not retyping. Clearing
it is a separate edit.

## What a release goes out as

A release carries the text it ships under — a title, a description, tags, a
comment to pin beneath it. Which boxes appear is your profile's answer, not
kilna's: a video's YouTube release is asked for four things, a beta read for
two, and a kind that names no fields simply shows none. The list is the
release kind's `fields` — see
[the profile document](/kilna/reference/profile-document/).

The boxes are in the **Release** block of the publication's overview, under
its place, day and hour. They save as you leave each one, and each can be
copied on its own, which is what you are doing with them: pasting them into
the place the thing is actually being published.

### A title for the audience, and its tail

A publication's own name is kilna's word for it — *Harbour lights (audio)* —
and the audience never sees it: Studio's title fields read **`{origin}`**,
the title of the song the publication is made from, even for a short cut
from the clip.

A field may keep a **tail**: the title of an audio's YouTube release always
ends with *" (audio)"*, a clip's has none. The release keeps it whoever writes
the rest — you typing, the template, the assistant, an agent — and takes it
off again when the release moves to a place that keeps no tail. An empty
title stays empty. The tail is the field's `suffix` in
[the profile document](/kilna/reference/profile-document/).

### Without the singer's marks

A description that quotes the lyric — the audio's `{donor:lyrics}` — reads it
as the public does: the capital vowels that mark stresses for the singer are
lowered and every respelling you keep is put back (*МарсЭль* becomes
*Марсель*). The same goes for what the assistant or an agent proposes for a
field. What you type into a field yourself is left as you typed it. See
[Singing a text](/kilna/guides/singing-a-text/).

### Writing them from the work

A field may carry a **template**, and then kilna can fill it for you. The
template is written in the same language an [assistant
action](/kilna/guides/the-assistant/) uses — `{title}`, `{role:lyrics}`,
`{scenes}` — and it means exactly the same thing in both places.

**Write these from the work** fills every templated field at once. Fields
without a template are never touched: those are the ones you type by hand, and
overwriting them with nothing is the one thing the button must not do. If a
field reads a version role the work has not been written in yet, it is not
filled blank — kilna says which field is waiting on what, and the rest are
still written.

Once something is written, the button asks before replacing it.

### What it goes out under

A template fills a field from the work's own words. Writing a description a
channel would post is a different job: it reads the song the release goes out
for, the channel's voice and signature, and what went out last week, so this
one does not sign off or ask the way the last one did. That is the **Release
meta** button on a release - an [assistant action](/kilna/guides/the-assistant/)
about one release (`"scope": "release"`).

It runs in the background. When the answer comes, **the fields nobody has
written yet are filled** at once - you asked for them. The ones you had already
started are not touched: what the answer says about them waits under the
release, each field beside what you wrote, to take one at a time or all
together, or to turn down. A clip, an audio release or a short made with
**Make…** starts it on its first release by itself, and its Cover tab says
while it is writing.

An agent working through [MCP](/kilna/reference/mcp/) proposes the same way
(`propose_release`), and fills nothing: its fields all wait for you.

### A character count, not a limit

A field may state how many characters the place it is going will accept. The
count is shown beside the box and turns red past the limit, and that is all it
does. kilna is not the authority on what a platform accepts this month, and a
box that refuses to hold what you typed is a box you would type somewhere
else.

### A month at once

On the calendar, **Write what they go out as** fills every release still
planned in the month on screen. It reads the whole month rather than what the
kind chips are showing: narrowing the view is a way of looking, not an
instruction about which releases to write.

What has already gone out is never touched. Its metadata is the record of what
it went out under, and rewriting that from today's text would quietly rewrite
history.

## The link

A release's link opens in your browser and can be copied from the row's menu.
It is shown as its host and path rather than in full, so a long address does
not push everything else off the row; hovering shows the whole thing, and
copying copies the whole thing.

Only ordinary web addresses open. A link is typed by a person, and a `file:`
address or a scheme belonging to some other installed program is not something
a note about a publication should be able to launch — the field says so while
you type rather than failing silently later.

## Editing a release

Clicking a chip opens the release: its place, its date, its hour and the
link. The same fields stand in the **Release** block at the top of the
publication's overview, whose menu carries the rest of the actions on this
page: marking it released, taking that mark back, returning it to the queue,
copying the link, and deleting it. What the calendar can do to a release and
what its work can do to it are the same list, minus the drag that only a grid
can offer.

**The hour** is for the places that ask for one. The first time you set it,
it is said in the zone this computer is in, and the zone is kept with it, so
the release still means one moment when it is read elsewhere. Changing
the date here **moves** the booking rather than bidding for a new one — a date
typed into a form is a correction, and the app pushing back mid-edit would be
answering a question nobody asked. Clearing the date returns it to the queue,
which is the same thing unscheduling does.

The move is written to [History](/kilna/guides/the-history/) and the work's
[status](/kilna/guides/statuses/) is worked out again, exactly as when a slot
is claimed. A release that moved without either would leave the work calling
itself scheduled after its date was cleared.

## Taking a release out of the calendar

**Unscheduling** a release clears its date without touching anything else: the
release stays exactly as planned, just without a slot, and returns to the
queue.
