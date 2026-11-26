---
title: The short style
description: What a space's shorts look like - the blocks of the frame, the variants a short wears by the day it goes out, the words lit as they are sung - and how a short is built in it.
---

A channel's shorts are one frame worn many ways. The frame is the same for
every short: the song's title and a call at the top, the channel's stamp over
the picture's top edge, the picture, the sound drawn under it, the words of
the song, the channel's sign, and the short's cover at the end. What is drawn
in it is the space's **short style** - a record of the space, built of blocks
in **Settings → Shorts** and saved with the rest of the profile.

## The blocks

Each block has its settings and, where it can be left out, its switch. One
frame is drawn live beside them, by the same code that draws a build - what
you see there is what a short is built in.

| Block | What it draws | What is set |
| --- | --- | --- |
| **Colours** | Everything: the wave, the sung words and half the blots take the first, the call and the other half the second. | Two colours. |
| **Header** | The song's title, fitted to the width, and a call under it. | The call, the font, whether the title is in capitals. |
| **Sign** | The channel's mark under the words. It grows with the loudness, sways, and blinks when it has a second picture with its eyes shut. | A picture or an SVG, its size at rest, how much the sound grows it, how far it sways. |
| **Ground** | What is under everything: a colour and a pattern - ink blots, dots, a grid, stripes, or the colour alone. | The colour, the pattern, each blot's place, size, colour and shape, and whether the blots drift and are thrown where the sound hits. **Draw anew** keeps the places and draws other ink. |
| **Stamp** | The channel's name as a rubber stamp or a plate - or a stamp of your own, as a picture. | The name, a second line, the ink, the tilt, where its middle stands, how worn it is. |
| **Wave** | The sound: a filled wave, two lines, or mirrored bars - and a strip that fills as the short plays. | The kind, the strip. |
| **Words** | The song's words, lit word by word as they are sung. | The font and the size. |
| **Effects** | What is done to the picture. | How far it pushes in, the grain, a vignette, a tear of colour where the sound hits. |
| **The cover at the end** | The short's own cover, pulling back to rest whole. | How long it stands. |

A picture - the sign, a stamp of your own - is kept in the style as text: an
SVG, or a small PNG. The style travels with the profile, so a sign is a mark,
not a photograph.

A space that never opened the section still builds its shorts: in kilna's own
style, with every block on that needs nothing of the channel's.

## Variants

A **variant** is a few values laid over the style: the two colours, the sign,
the ground, the tilt and the place of the stamp, the kind of wave. Each value
has a box that says whether the variant replaces it at all; what it leaves out
is the style's. Where things stand is never a variant's to change - the frame
stays the channel's.

A short wears one variant:

- **By the calendar**, unless told otherwise: the variants go round by the day
  the short goes out, one after another, so the shorts of neighbouring days
  differ. A short with no day yet takes the day it was made.
- **By hand**, on its Build tab: pick a variant there and the short keeps it,
  whatever day it moves to. **By the calendar** gives it back to the round.

Take a variant out of the style and the shorts that wore it by hand go back
to the round.

## Building a short

A short's **Build** tab draws the frame it will wear - with its own title and
its own cover where the picture will be - and says what it is built from: how
long its stretches run, where its words come from, and what it still lacks.

| It says | What to do |
| --- | --- |
| *No stretch is marked yet* | Mark one on the [Cuts](/kilna/guides/shorts/) tab. |
| *…which a stretch is cut from, has no finished file* | Render the video, or choose its file on its Files tab. |
| *The short has no folder on disk* | Set the media folder in **Settings → Data**. |
| *No cover is chosen* | The short is built without its cover at the end; choose one on its [Cover](/kilna/guides/the-cover/) tab to have it. |

**Build the short** draws the layers of the frame, hands them over and runs
[kilna-plugin-cut](/kilna/guides/cutting-a-short/); the bar shows how far it
has got and **Stop** ends it at once. The file lies in the short's folder as
the next `v{k}.mp4` under `video` - the build before it stays - and plays
right on the tab. kilna itself never opens a video: the plugin does the
listening and the building, with ffmpeg.

## The words

The words under the picture are the text of the song the short is made from;
when each is sung comes from a **word alignment** of its recording, laid as
`*.alignment.json` in the song's folder - or the folder of the video the
short is cut from, or the short's own. The newest file of the nearest folder
is read.

```json
{
  "words": [
    { "word": "The", "start_s": 12.48, "end_s": 12.71 },
    { "word": " tide", "start_s": 12.71, "end_s": 13.4 }
  ]
}
```

An alignment hears a song its own way - syllables for words, a misheard
word, lines broken elsewhere - so kilna matches the two letter by letter and
gives every word of the text the time of the letters it landed on. A line the
alignment misheard is spread over what was heard between its neighbours.
Section tags and backing lines in brackets are not shown, and a capital vowel
that marks the stress for the singer goes back to lower case.

Three keys are optional. `audio` names the recording the times are of, as a
file beside this one; without it the newest recording in the folder is taken.
The plugin lays the video over that recording to find how far one runs behind
the other - a video often opens a little later than its track - or reads
`offset`, in seconds, when you would rather say it. `replace` shows a word
otherwise than the text writes it: `{ "Again»": "again»" }`.

With no alignment the short is built without words, and the tab says so.

## Through an agent

An [agent outside the window](/kilna/reference/mcp/) reads a short with
`work` - its stretches, the variant it wears and who chose it, where its build
stands - and the style with `workspace`. It proposes a style or a variant
with `propose_short`; what it proposes waits in the chat, drawn as a frame,
until you take it. The profile's **Shape the short** skill is the method it
works by. The build is yours to start.

See [ADR 0090](https://github.com/lacodda/kilna/blob/main/docs/adr/0090-a-short-is-drawn-in-the-window-and-moved-by-the-plugin.md)
for why the window draws the layers and the plugin moves them.
