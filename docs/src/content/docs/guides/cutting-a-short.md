---
title: Cutting a short
description: Install the cutting plugin, write the channel's words, and cut a short into its own folder from the Cuts tab - with the progress, the stop, and the check of its length.
---

kilna keeps where in a video a short is taken from, and never opens the video
itself. The cutting is done by a plugin,
[kilna-plugin-cut](https://github.com/lacodda/kilna-plugin-cut): **Cut the
short** on a short's Cuts tab, and a new vertical file lies in the short's
folder, where kilna finds it as the short's
[finished file](/kilna/guides/files-and-covers/#the-finished-file).

## Before the first cut

1. **ffmpeg on the PATH.** The plugin runs `ffmpeg` and `ffprobe`. Without
   them it stands in the plugin list with the reason instead of a button. On
   Windows, `winget install Gyan.FFmpeg` puts both on the PATH.
2. **The plugin.** In PowerShell:

   ```powershell
   irm https://raw.githubusercontent.com/lacodda/kilna-plugin-cut/main/tools/install.ps1 | iex
   ```

   or, on macOS and Linux:

   ```sh
   curl -fsSL https://raw.githubusercontent.com/lacodda/kilna-plugin-cut/main/tools/install.sh | sh
   ```

   It lands in the `plugins` folder beside your workspace, with an example
   `kilna-plugin-cut.json` next to it. The next time kilna lists its plugins -
   a short's Cuts tab opened half a minute later, or kilna started again -
   the button is there.
3. **The channel's words.** Open `kilna-plugin-cut.json` and write your
   channel's name and the call under the picture:

   ```json
   {
     "channel": "Harbour Radio",
     "call": "FULL VERSION"
   }
   ```

   Every key is optional - fonts, colours, how long the cover stands - see the
   plugin's [settings](https://github.com/lacodda/kilna-plugin-cut#settings).
   The words are the plugin's, not the workspace's: a channel renamed is one
   line in this file, and the shorts already cut keep the name they went out
   with.
4. **Folders.** The short needs a folder of its own, and the video it is cut
   from a finished file: a media folder in **Settings → Data**, a folder for
   each kind - Studio ships `songs/{origin.title}/shorts/{n}` for a short - and
   a rule for the finished file. The cut list says which of these is missing,
   stretch by stretch.

## Cutting

On the short's **Cuts** tab, under the cut list, press **Cut the short**. The
button becomes a bar - how far the cut has got and what it is doing - with
**Stop** beside it, which ends the cut at once.

The stretches are cut in the order the list splices them and joined in one
pass:

- the stretch's picture, full width, over a darkened blur of itself;
- the channel's name and the song's title above, the title lettered to fit;
- the call below, with a pointing hand;
- the short's cover for half a second at the end - the one on its Cover tab,
  or else the newest picture in its folder's `covers`.

The file goes into the short's folder as the next `v{k}.mp4` under `video`:
the cut before it stays, and a cut stopped halfway leaves nothing kilna would
take for the short's file. When the cut is done kilna names the file, and the
cut list shows it with how long it runs beside how long the stretches add up
to - the cover's half second is the whole difference when all is well.

## When something is in the way

The cut list says what stands in the way before you press anything: a video
with no finished file, no folder for the short, no stretches. A cut that
fails says why in the plugin's words - a file ffmpeg cannot read, a font that
is not where the settings say.

See [Shorts](/kilna/guides/shorts/) for taking the stretches, and
[Writing a plugin](/kilna/guides/writing-a-plugin/) for everything kilna hands
a cutter.
