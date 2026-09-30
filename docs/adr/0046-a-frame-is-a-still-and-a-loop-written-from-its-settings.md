# 0046 — A frame is a still and a loop written from its settings

Date: 2026-09-30
Status: Accepted

## Context

An audio release on a video platform is a track with a picture on it. The
picture stays on screen for the whole song, and it usually moves a little: a
leaf drifts across, dust turns in a beam of light, rain runs down a window. A
generator makes that from two prompts - the still, and the motion made from
it - and the motion has to loop: it repeats for three minutes, so its last
frame must meet its first, and the camera must not drift away and jump back.

Before v0.86 the owner kept this in the storyboard of a video: a scene called
"still frame" with the kind's `still`, `motion` and `negative` blocks. It
worked, and it said the wrong thing - a board of one scene, on a video that is
not a video, with the loop's length and seamlessness written as prose in the
motion block and forgotten half the time.

ADR 0029 made the parts of a cover's prompt the craft's words
(`cover_blocks`): the code does not know what a cover is made of. The frame
cannot follow it all the way. Its length in seconds, a still camera and a
seamless join are not words of a craft; they are facts about a loop, with a
number and two switches, and the prompt sentence each of them needs is the
same in every craft. The owner decided the same for the cover's layout on
2026-09-27: the frame is built into the code, and the schema and the prompt
are drawn from the same parameters.

## Decision

**A kind says whether its works play under a frame (`frame: true`).** The
shipped Studio profile says so for the new `audio` kind. Only such a kind has
the Frame tab, and writing a frame on any other kind is refused
(`work.noFrame`).

**The frame is one column, `work.frame`, with a shape the application owns:**
`still`, `motion`, `seconds`, `still_camera`, `seamless`, `negative`. A frame
read from `{}` is a frame at its starting settings - six seconds, a still
camera, a seamless join - because a loop under a song nearly always is one.
The whole frame travels in a patch, as the cover does, so the log's `before`
holds it whole and an undo puts it back.

**The loop is written from its settings.** `Frame::prompts` gives three
blocks to copy: the still as written, the loop - "LOOP (6 s): the beam turns.
The camera does not move at all. Seamless loop: …" - and the negative as
written. Each setting says its sentence and nothing else does: a camera
allowed to move is not told to hold still. An empty motion is "barely
noticeable breathing of the light", not a sentence with a hole in it. The
blocks are written once, in Rust, and read by the tab, by an agent through
MCP `work`, and by the command the tab copies from; there is no second copy
in the window to drift from this one.

**The frame's text is searched**, like the cover's: the triggers of
migration 0031 take it in.

## Consequences

- Migration 0031 adds the column and rewrites the clock and search triggers.
- v0.88's constructor builds the still from the cover's concept without the
  words on it; the settings of the loop stay where they are.
- The owner's three audio releases made as videos keep their "still frame"
  scenes after they become audio works (ADR 0047): the Scenes tab stays while
  a work holds scenes, and the prompts are moved into the frame by hand.
