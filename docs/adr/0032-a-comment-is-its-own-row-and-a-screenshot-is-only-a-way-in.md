# 32. A comment is its own row, and a screenshot is only a way in

Date: 2026-09-22

## Status

Accepted.

## Context

The audience's comments were the one part of the predecessor's studio loop
kilna did not hold. The predecessor kept them in a table of their own with a
channel slug, a free-text song title, a status and a screenshot file, and
three things about it did not work:

- **The channel was a slug, and four channels out of five were not rows of
  its database.** Comments come from wherever a work went out.
- **The song was matched by title.** A comment found its song by comparing
  normalised titles, which broke on a rename or a pair of quotes.
- **The screenshot was a column nobody kept.** A comment saved from a picture
  got a placeholder body until someone transcribed it, and at the end not one
  of its 121 comments still held a screenshot. The upload button was the step
  that got skipped.

Answers were drafted by an assistant told the channel's name and asked for
its voice, with nothing to hear that voice in.

## Decision

**A comment is a row of its own table, not a note with a kind.** It has a
channel, an author, the day it was written, a reply and a state: facts about
the row, and so columns (ADR 0013). As a note they would be fields in a JSON
column only one kind of note has.

**The channel is the word the person uses.** Free text, with the words already
used offered back. A channel table would be a form to fill in before the first
comment, for a list the comments already are.

**The work is a key.** A comment about no work in particular has none. It goes
to the trash with its work and comes back with it.

**Three stored states — open, posted, archived.** "A reply is drafted" is read
off the reply being present on an open comment, never stored, so it cannot
disagree with the reply. Posting is always by hand: kilna does not speak on
anyone's channel.

**A screenshot is read, not kept.** A pasted picture goes to a temporary
folder outside the workspace and is read in the background by a profile
action (`"scope": "comment"`, `"produces": "comment"`). The answer is a
proposal (ADR 0018) with every field open to correction; the comment exists
only once the person keeps it. The channel travels in the task's key, where
the paste put it, and is never taken from the answer: the picture cannot say
which channel it is.

**The voice of a channel is what was posted on it.** A reply is drafted by a
second action (`"produces": "reply"`) given the replies already posted on the
same channel as examples. The channel is only a word, so a voice setting
would have nowhere to live, and the posted replies describe how the channel
speaks better than a setting would.

## Consequences

- There is never a comment with no words: no placeholder bodies, no
  screenshot waiting for a transcription.
- Pasting is a background task. Several screenshots can be read in a row, each
  coming back to be checked.
- The first reply on a new channel has no voice to match, and says so in its
  instruction rather than inventing one.
- An agent outside the window cannot propose comments or replies; they are
  read and answered where they were pasted and opened.
