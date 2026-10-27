# 0070 — A work is finished as a file in its folder, and a cutter is handed all of it

Date: 2026-10-26
Status: Accepted

## Context

The Cuts tab has kept a short's stretches since v0.60 and handed them out as
a shot list, each stretch beside "the donor's video". That video was the
newest asset of kind `video` on the donor - and only the scenes board ever
writes one, as the animation of a single scene. On the owner's workspace
(25.10.2026) 145 shorts had stretches and every one of their lines had no
file: the finished clips lay in the clip's folder on disk, which kilna has
looked at since v0.93 (ADR 0057) but never asked which file in it is the
clip.

A cutter needs more than the stretches. It needs to know where to put what it
makes - the owner has kept each short in a folder of its own since before
kilna, `shorts/v2` for the second - and the names to letter it with, and the
cover to end it on. A plugin was handed a work's row and bodies, could merge
fields back, had two minutes and said nothing while it ran.

## Decision

**A work is finished as a file in its folder.** The kind names a rule, `file`
in the profile - a path inside the work's folder whose steps may hold `*` and
`?`: `v*/video/*.mp4` for the owner's clips, `video/*.mp4` for a short. The
newest file the rule matches is the work's finished file. Newest by when it
last changed, not by name: a render made again is the replacement, and the
owner's `v4.mp4` is the newest of four by both measures anyway. A step
matches a name whatever its case, and only one level: `v*/video/*.mp4` never
reaches the scene animations in `v1/video/hd/`.

**A person's choice stands over the rule.** The Files tab makes any clip or
sound of the folder the finished file; the choice is kept on the work as a
path inside its folder (`work.file`, migration 45), so a folder template
changed later still points at it while the layout inside is kept. A chosen
file that is gone is said, not replaced by what the rule finds: the person
said which file, and cutting another in its name would be a guess. **Back to
the rule** clears the choice. Both are ordinary edits of the work: logged,
undone, replayed.

**A work has a number in its family.** `work.ordinal` is the work's number
among the works of its kind made from the same origin - the 2 of "Song
(short 2)" - and `{n}` reads it in a folder template as it always did in a
title. It is given once, past the greatest given in the family, live or in
the trash, never into a gap (ADR 0059's rule and reason: a folder named after
a number that moved would be another work's). `derive` gives it with the
title, one count for both, before the work is logged; a work linked to its
source by hand is given it by the link. The works made before v0.101 take
the number their title carries, once, when the workspace opens. A number can
be typed over on the Files tab, never onto a sibling's.

**A number is the folder's, not the title's.** The rehearsal on the owner's
workspace found the two apart: v0.90 numbered the titles in the order the
shorts went out, and atlas had numbered the folders in the order they were
made, so in 17 families a short's file lay in another number's folder.
Titles that went out stay as they went out; 54 numbers were set from the
files on disk, as the owner's data rather than code: each short matched to
the folder whose file sounds like its first stretch of the donor (the
loudness over twelve seconds - a picture cannot tell two stretches of an
audio release's still frame apart), checked against a frame of every stretch
and against the length, and a tie between two shorts with the same stretches
given to the one that went out.

**A cutter is handed all of it, once.** A command of a plugin targets
`work`, `release` or, since v0.101, `cuts` - the Cuts tab. A work travels
with its shot list (each stretch beside its donor's finished file), its
folder and finished file, each donor's folder and file, the chain of titles
up to what it is all made from, and its cover. A reply may name the files it
made: the first one inside the work's folder becomes the finished file when
the rule would not find it by itself, so a new `v2.mp4` beside `v1.mp4` is
found by the rule and the next render made outside kilna is found too.

**A cutter takes the time it says, says how far it is, and can be stopped.**
A command declares `timeout` in seconds (two minutes when it says nothing, an
hour at most). A plugin writes lines of JSON to stderr - `{"progress": 0.4,
"message": "cutting 2 of 3"}` - which the window draws as a bar under the
command; any other line of stderr is kept for the message if it fails.
**Stop** kills the run. All of it is optional on both sides, so the protocol
stays 1: a plugin written for it runs unchanged.

**The core still opens no video.** The length of a short's finished file,
shown beside the sum of its stretches, is read by the window's player from
the file's header; ffmpeg belongs to the plugin (decision of 2026-09-11).

## Consequences

- Of the owner's 135 short files, 133 are found at their own short - 131 of
  them run as long as its stretches to half a second - and 145 of 149 shorts
  have every stretch's file: a cutter can run on them today. The two left
  over belong to no short kilna holds: two planned shorts repeat the
  stretches of two that went out.
- The shipped Studio profile keeps a clip in `songs/{origin.title}/video`, an
  audio in `.../audio` and a short in `.../shorts/{n}`, each finished as the
  newest `.mp4` there. A workspace whose kind keeps its folder where the
  shipped profile does is given the shipped rule; one that laid its folders
  out itself is not, since the rule is a path inside a folder it did not name.
- A short whose donor has no finished file says why on its Cuts tab: no
  folder, nothing matching yet, no rule, or a chosen file gone.
- A number stays with its work through the trash; a work emptied from the
  trash takes its number out of the count, as a code does.
