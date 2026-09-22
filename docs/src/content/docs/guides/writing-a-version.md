---
title: Writing a version
description: The editor — click into the text and it is the next revision, one version per sitting, the whole screen for writing, how each role reads, naming a version, and comparing two drafts side by side.
---

Every draft of a work is kept whole. Nothing is stored as a diff — so going
back to what you wrote three weeks ago is opening it, not reconstructing it.

This is the **Versions** tab of a work's card.

## Revising is clicking into the text

The open version is there to be read. Click into it and it is there to be
written: the text becomes a box, the card steps aside so the text has the
whole content area, and you type.

The first change starts the next revision. kilna mints it from the version
you clicked into, makes it the current one, and every change after that goes
into it — written a moment after you pause, with a quiet *saved* beside the
title. There is no save step. `Ctrl+S` writes right now instead of a moment
later, for the hand that presses it anyway.

The revision you started from stays exactly as it was. That is the promise
the history makes, and it is why the first keystroke makes a new version
rather than changing the one you opened.

### One version per sitting

What makes this one revision rather than fifty is the sitting. All the
changes you make to the open text are one version, until you leave it — you
open another version or another role, or you go to a different tab — or
until you pause for twenty minutes. The next change after that starts the
next revision.

So an evening of work on a lyric is one version, and the next morning's
changes are the next one. Neither "a version every time the editor opens" nor
"a version every ten minutes" says anything true about the writing; the
sitting does.

### A judged version does not change

A score is a snapshot of the text it read. Once a version has been scored,
its text is frozen: the next change you type into it starts a new revision
instead, silently, because that is what you meant anyway. The version the
score judged stays as the score saw it.

## Reading

Each pane has three ways of showing the text, as icons in its corner:

- **Read** shows the body the way its role reads (below). This is the
  default, and clicking into it is how you start writing.
- **Edit** is the same text in a box, in place, for the small fix that does
  not need the room.
- **Changes** compares the open version with the revision before it, without
  your having to find that revision first. On the first revision of a role
  there is nothing before it, and kilna says so rather than comparing it
  with itself.

### How a role reads

The profile says how each role's text is drawn, because kilna cannot tell a
lyric sheet from an essay by looking at it:

- A **plain** role — lyrics, a style prompt — is a monospace column, exactly
  as typed. A verse is a shape on the page, and the shape is part of it.
- A **markdown** role — a review, a chapter — draws its headings, quotes,
  lists and tables.

Music ships lyrics and style as plain, review and critique as markdown; the
[profile document](/kilna/reference/profile-document/#version_roles) is where
that is set. Either way kilna stores the text exactly as you typed it.
Markdown is a way of *looking* at it, never a conversion.

## The whole screen

Clicking into the text gives it the whole content area, with the sidebar in
place. The **expand** icon does the same without starting to write, for
reading a long text. The **focus** icon goes one further: the text alone,
over the whole window, nothing else.

`Esc` steps back one level at a time — from focus to the content area, from
the content area to the card. Nothing about the text changes on the way in
or out.

## Beside the text

A role written *about* another — a review or a critique of the lyrics — sits
beside the text it discusses, matched revision for revision: a review of
revision 2 is shown beside revision 2 and not beside revision 5. It reads,
edits and expands the same way the text does, in its own sitting.

## Writing a version from nothing, or from a copy

Two moments need a decision before the text, and they open the form:

- **There is no version in this role yet.** The form is there because there
  is nothing to click into.
- **You want a copy to work on.** The **new version from this one** button
  on any row copies that version's text into the form — for a rewrite that
  keeps the original open beside it, or for a version that wants a name.
  **New version** at the top of the tab opens the same form empty.

The form is the only place a version is **named**. `tightened chorus` says
more in the list a month later than `Revision 4`; unnamed versions are listed
by number, so there is no penalty for skipping it. **Make this the current
version** is ticked by default; untick it to record an experiment without
promoting it — scores and exports keep reading the version the work pointed
at before, and the star beside any row promotes it later.

Text in the form is kept as you type, and is still there after you close
kilna and open it again — a small *Draft kept* note says so. The draft
disappears by becoming a version, and in no other way except your own
deletion. If you copy a version into a form that already holds a draft, the
draft is set aside rather than guarded by a question: the toast offers
**bring the draft back**.

Once saved, the new version opens and the list scrolls to it. A version made
anywhere else — from the assistant's *insert as version*, say — does the
same: the tab opens on it, so a version is never only a toast.

## Comparing two versions

The **±** button beside *Read* and *Edit* puts another version of the same
role on the right of the text. With one other version to choose from it opens
at once; with several it lists them by name, the revision immediately before
the open one first — *what moved since last time* is the question asked most
often of a history, and it should not cost a hunt through the list. The **±**
on any row of the list does the same for that row.

The comparison is a second column, not a third way of reading. On the left
the open text stays as it was — read as its role reads, or edited — with the
lines that are new marked green; on the right the other version stands whole,
with the lines that are gone marked red. A line above the right column says
how much moved. Because the left side is the live text, you can rewrite with
the original in view, and the marks follow every keystroke.

The two columns appear at any width, on the card and on the whole screen.
The comparison is by line rather than by word, because a version here is
prose — a verse, a scene, a script — and prose is revised by the line; a
word-level diff of a rewritten verse is confetti.

Press **±** again, or the cross on the right column, to stop comparing.
Opening the version you were comparing with clears it too.

## Repeated words

While you edit, words the text uses more than once are tinted — the same tint
for the same word wherever it appears — and a strip under the toolbar counts
them: *лестница ×3*. Forms of a word count together: *лестница*, *лестницы*
and *лестницей* are one word leaned on three times, *ladder* and *ladders*
likewise. The matching is a light stemmer for Russian and English, not a
dictionary, so an unusual word may occasionally be grouped with a neighbour
it does not belong to; it is a prompt to look, not a verdict.

Function words — *и*, *the*, *не* — are left out, as is anything inside
square brackets: `[Verse 2]` names a section and repeats by design. Nothing is
drawn when nothing repeats.

## Walking the history

With the version list focused, the arrow keys step through it: **↓** to the
next revision back, **↑** toward the newest, **Home** and **End** to either
end. Reading through six revisions is six presses rather than six aimed
clicks. A comparison with the previous revision follows the step — each
revision against its own predecessor in turn — while a comparison with a
version you picked by hand stays pointed at it, so an original can be kept
beside a history being walked.

The tab is two columns, each scrolling on its own: the list of revisions on
the left, the open one on the right. Scrolling back through twenty revisions
does not move the text you are reading, and reading to the end of a long text
does not take the list away. **New version** sits at the foot of the list,
where the next row will appear.

## Roles

A work's versions are grouped by **role**, and the roles come from your
[profile](/kilna/concepts/profiles/): a song has `lyrics` and `style`, a chapter
has `text`, `outline` and `notes`. Roles advance independently — lyrics v4 has
nothing to do with style v2 — so the tab shows one role at a time rather than
interleaving them. The roles are chips at the top of the list, each with how
many versions it holds; a role written *about* another — a review, a critique —
is not among them, because it opens beside the revision it discusses.
