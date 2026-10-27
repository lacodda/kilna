# 0071 — Where to cut from is heard and offered, never taken for the person

Date: 2026-10-27
Status: Accepted

## Context

Since v0.101 a short's stretches reach a cutter with everything it needs,
and kilna-plugin-cut makes the file. What is still done by ear is the part
that takes longest: deciding where in the video a short starts and stops.
The owner has about 150 shorts, one stretch each, median 30 seconds (from 21
to 40), and a short cut from the wrong place is the most common weak one.

A spike on those shorts (27.10.2026) measured what sound alone can say. The
song was heard as half-second frames of loudness and chroma - how much of
each of the twelve notes sounds - and parts were taken to begin where the
notes turn (novelty on the chroma's self-similarity). Two findings:

- **Where parts begin carries the signal.** The owner's stretches start
  within two seconds of a beginning heard this way 76% of the time against
  46% for random beginnings of the same density, and within three seconds
  94% against 58%. People start a short where a part starts.
- **Which part is best does not.** Ranking the parts by loudness and by how
  closely they come back beats random windows only modestly (the five best
  overlap a stretch the owner took, by half or more, 58% of the time against
  41%). The owner's stretches were picked by agents for a picture or a line
  as often as for the music, and they tile the whole song - four shorts out
  of one three-minute track.

kilna itself does not open a video (decision of 2026-09-11). It does know a
video's scenes - their sections and seconds - and the song's words, and
which stretches every short already took.

## Decision

**A plugin hears; kilna keeps what it heard beside the video, for that file
only.** A plugin command on the Cuts tab may answer with `suggestions`: for
each video the short is made from, the file it heard, the seconds where its
parts begin and up to a few parts worth a short, each with a reason
(`chorus`, `loudest`, `repeats`, `lift`, `part`). kilna keeps them in
`donor_map`, one row per video, with the file's path, size and moment of
last writing, and offers them only while the video's finished file is still
that file as it stood. A new render is new sound; what was heard in the old
one is not drawn over it. The row is a record of a file, written past the
log like the media folder (ADR 0057): a replay has no file to rebuild it
from, and listening again rebuilds it. kilna-plugin-cut 0.2.0's
**Find the parts** is the first such command.

**The words place the chorus where the scenes play it.** A video whose
scenes are timed and named by section offers, from the core and without
opening anything, the runs of scenes that play a chorus, and those whose
lines come back elsewhere in the song, with the section and its first line.

**Beginnings are a grid, parts are an offer.** The beginnings are drawn on
the video's track as hairlines, and an end let go within a few pixels of one
lands on it - Alt held lets it stay where it was let go. The parts are drawn
on a lane under the track, dashed, and listed in words with **Take**, which
makes one an ordinary stretch: one cut, one line of history, one undo. What
the short's siblings took out of the same video is drawn grey, and a part
lying mostly inside one says so. Nothing is chosen for the person.

The plugin protocol stays 1: `suggestions` is optional, and a kilna that does
not know it ignores it.

## Consequences

- The grid is the part that measurably helps, so it is where the window
  spends its effort; the parts carry their reason, so a person can tell a
  heard chorus from a guess.
- A video with no finished file cannot be heard; one rendered again since
  says so and is listened to again. Nothing heard survives the file it was
  heard in.
- `donor_map` is not exported, merged or replayed. A workspace restored on
  another machine has nothing heard until its videos are listened to there.
- The ranking can improve in the plugin alone - its reasons are keys the
  window words, and a key it does not know is shown as it is.
