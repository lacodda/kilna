---
title: Comments
description: What the audience wrote, from every channel, in one inbox — pasted in as screenshots, answered with drafts in the channel's own voice, posted by you.
---

A work that went out gets written about. kilna keeps what people wrote under
it, and your answers, in one place: **Comments** in the menu, or the
**Comments** tab of a work.

## What a comment is

- **A channel** — where it was written, in your own word for it: a channel's
  name, a site, a platform. There is no list to fill in first. The channels
  you have used are offered in a list under the field as you enter it,
  narrowed by what you type, so the next comment from the same place is
  filed under the same word; a word that is not in the list is simply kept
  (a row of buttons under the field until v0.118).
- **The words**, exactly as written, and **who** wrote them.
- **The day** it was written.
- **The work** it is under, when it is under one - what the audience saw: a
  clip, an audio release, a short. Never a song: a song is not in front of
  anyone as the song, so a comment cannot be filed on one (since v0.86). A
  comment about the channel itself is about nothing in particular, which is
  fine.
- **The reply**, and where the comment stands.

## Getting comments in: paste a screenshot

Take a screenshot of the comment and press `Ctrl+V` anywhere on the Comments
screen or a work's Comments tab. kilna asks the one thing a picture cannot
say — which channel it came from — and, off the Comments screen, which work
it is about. Then **Read it**.

The reading runs in the background, through the profile's **Read a comment**
action and the assistant. You can paste the next screenshot straight away.
Each reading comes back at the top of the list as a card: the text, the name,
the day — "3 weeks ago" is turned into the date it means — and the work, when
the picture showed its title and exactly one work goes by it. Correct
anything that was misread, then **Keep**. Nothing is saved until you do.

The picture itself is not kept. It is a way in, not a record: once the words
are in, nobody needs it, and a folder of screenshots only grows.

Several screenshots copied at once are one batch (v0.162): one channel for all
of them, and each is read on its own.

**+ Comment** types one in by hand, for when there is no screenshot. With
**Several comments** on, each paragraph of what you paste - comments copied
from a page, a blank line between them - becomes a comment of its own.

## Answering

Open a comment and write the reply in the box under it. It saves when you
leave the box.

**Draft a reply** asks the assistant for one, in the background. It is given
the comment, the work it is under with the opening of its text - and of the
song it was made from, which is usually what the comment is about - the
**voice** section of the space's passport (since v0.162: each space answers
as itself), and the replies you have **already posted on the same channel**
as samples of how it speaks. A reply you have started is rewritten rather
than ignored.

The draft comes back as **three replies** in the language the comment is
written in - live speech, no long dashes, never repeating the commenter's
words back - each with its translation into the window's language, along
with the comment's own translation and what in it a literal reading misses:
sarcasm, slang, a reference. **Use this one** puts the reply you choose in
the box, where it can still be changed; in the chat the same three are a
choice before **Keep as the reply**.

kilna never posts anything. **Copy** puts the reply on the clipboard to paste
where the comment is; **Mark as answered** is you saying it went. **Archive**
is for a comment that needs no reply.

## Where a comment stands

| On the list | Means |
| --- | --- |
| Waiting | Open, no reply yet |
| Reply drafted | Open, a reply is written but not sent |
| Answered | You posted the reply |
| Archived | Needs nothing |

The chips along the top narrow the list to one channel and show how many wait
on each; **Open**, **Answered** and **Archive** switch between the three. The
search finds comments by their words, their reply or who wrote them — and so
does `Ctrl+K`, which opens a comment where it lives.

A work's **Comments** tab counts its comments; the number turns to the accent
colour and shows how many still wait when any do.

Only a work that goes out has a **Comments** tab: the audience comments under
a clip, an audio release, a short - never under the song they were made from.
A song and an experiment have no tab and no address for one (since v0.117;
a song's tab summed up its publications' comments until then). What was said
about a song is on each thing made from it, and all of it on this screen.

## Actions behind it

The reading and the drafting are two actions of the profile, both with
`"scope": "comment"`: `read-comment` produces `comment`, `reply-to-comment`
produces `reply`. Their methods — how to read a screenshot, how to write a
reply — are in the profile document and can be rewritten like any other
action's. A profile without them still keeps comments; it only has no
reading or drafting. See [the profile document](/kilna/reference/profile-document/).

Comments are part of the [export](/kilna/reference/data/#export-to-markdown)
and go to the [trash](/kilna/guides/the-trash/) like everything else: deleting
a work takes its comments with it, and restoring it brings them back.
