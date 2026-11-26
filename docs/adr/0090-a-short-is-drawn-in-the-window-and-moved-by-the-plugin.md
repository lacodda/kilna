# 0090 — A short is drawn in the window and moved by the plugin

Date: 2026-11-26
Status: Accepted

## Context

Until v0.168 a short was cut by kilna-plugin-cut into one layout of its own -
the picture over its blur, two bars of words - set by a settings file beside
the executable. A channel's real shorts had outgrown it: a ground of ink
blots, a stamp, a sign that moves with the sound, the words of the song lit
as they are sung, and several variants of all of it so that neighbouring days
differ. That look lived in a script outside kilna, with its pictures in a
folder: a file, not a record, invisible to the window and to an agent.

Three things had to be decided: where the look is kept, who draws it, and who
turns it into a video. The standing decision of 2026-09-11 holds the core to
never opening a video and never running ffmpeg.

## Decision

**The look is the profile's.** `shorts` is a section of the profile document
(`ShortStyle`): blocks - header, sign, ground, stamp, wave, karaoke, effects,
tail - each with its settings and its switch, and `variants`, each a few
values laid over the style. It is saved, logged and taken back by
`profile.update` like the rest of the space's vocabulary, and travels with the
profile. A space with none builds in the line's own default.

**The frame is the application's.** Where the picture, the words and the sign
stand is one set of numbers (`src/lib/shorts/frame.ts`, mirrored in the
plugin), the same for every space. A style changes what is drawn, a variant
how it is worn, neither where things stand.

**A short's variant is a key of its fields.** `work.meta.short_variant`,
merged and undone like any field. Without it the variants go round by the day
the short goes out (`ShortStyle::round`), so neighbouring days differ with
nothing stored.

**The window draws what stands still.** The ground, the blots, the header,
the stamp and the sign are drawn as SVG and canvas in the window
(`src/lib/shorts/`) - once for the live preview in the constructor and on the
Build tab, and again, by the same code, as PNG layers laid in a temporary
folder before a build (`stage_short_layers`). A blot is drawn from a seed by a
small generator, so the same style draws the same ink on every machine.

**The plugin moves it.** kilna-plugin-cut 0.4 reads `subject.short` - the
look, the layers, the words - listens to the short's own sound (loudness per
frame, the hits, the bands), and runs ffmpeg once: the picture pushed in, the
blots drifting, the sign piped in frame by frame, the words as a subtitle
script. Its old layout and its word settings are gone.

**The words are matched in the core, placed by the plugin.** The song's text
is matched letter by letter against a word alignment laid beside the song
(`karaoke.rs`) - text work, no recording opened. How far the video runs
behind the recording the alignment was made on needs decoded sound, so the
plugin measures it.

## Consequences

- One code path draws the preview and the build's layers; they cannot drift.
  The plugin needs no SVG engine and no fonts for the header.
- A build needs the window: an agent proposes a style or a variant
  (`propose_short`) and reads where a build stands, and the person starts it.
- The words' font is named, not embedded: the subtitle script is lettered by
  ffmpeg's libass with the system's fonts.
- A picture kept in the style - a sign, a stamp of the channel's own - is
  text in the profile document. It is held to a size, and an agent reads it
  as a mark of its length rather than as its text.

## Rejected

- **ffmpeg in the core.** A second place that runs it, and a reversal of the
  decision that keeps the core a record of intent.
- **A rasteriser in Rust** (resvg and a font stack) for the layers: a second
  renderer beside the window's, which is exactly what makes a preview lie.
- **A table for the style.** It is one document per space with no rows to
  join; a table would add a clock, a trash kind and a replay arm to hold what
  `profile.update` already holds.
- **A column on `work` for the variant.** One key of one kind of work does
  not earn a column every reader of `work` carries.
