---
title: Cutting a short
description: Install the plugin that builds a short, and build one into its own folder from its Build tab - with the progress, the stop, and the check of its length.
---

kilna keeps where in a video a short is taken from and what the channel's
shorts look like, and never opens the video itself. The building is done by a
plugin, [kilna-plugin-cut](https://github.com/lacodda/kilna-plugin-cut):
**Build the short** on a short's Build tab, and a new vertical file lies in
the short's folder, where kilna finds it as the short's
[finished file](/kilna/guides/files-and-covers/#the-finished-file).

## Before the first build

1. **ffmpeg on the PATH.** The plugin runs `ffmpeg` and `ffprobe`. Without
   them it stands in the plugin list with the reason instead of a button. On
   Windows, `winget install Gyan.FFmpeg` puts both on the PATH.
2. **The plugin**, version 0.4 or later. In PowerShell:

   ```powershell
   irm https://raw.githubusercontent.com/lacodda/kilna-plugin-cut/main/tools/install.ps1 | iex
   ```

   or, on macOS and Linux:

   ```sh
   curl -fsSL https://raw.githubusercontent.com/lacodda/kilna-plugin-cut/main/tools/install.sh | sh
   ```

   It lands in the `plugins` folder beside your workspace. The next time kilna
   lists its plugins - a short's card opened half a minute later, or kilna
   started again - the button is there.
3. **The look.** What a short looks like is not the plugin's: it is the
   space's [short style](/kilna/guides/the-short-style/), built in
   **Settings → Shorts**. **Settings → Plugins** holds only where the file
   goes inside the short's folder and how well it is encoded.
4. **Folders.** The short needs a folder of its own, and the video it is cut
   from a finished file: a media folder in **Settings → Data**, a folder for
   each kind - Studio ships `{kind}s/{id}`, a folder per work - and
   a rule for the finished file. The Build tab says which of these is missing.

## Building

On the short's **Build** tab press **Build the short**. The window draws the
still layers of the frame - the ground and its blots, the stamp, the sign,
the header - and hands them to the plugin with the look and the words. The
button becomes a bar - how far the build has got and what it is doing - with
**Stop** beside it, which ends it at once.

The stretches are spliced in the order the cut list has them and laid out in
one pass:

- the picture, 16:9, pushing in slowly, with grain, a vignette and a tear of
  colour where the sound hits;
- the ground, whose blots drift and are thrown sideways on a hit;
- a strip that fills as the short plays, and the sound as a wave or as bars;
- the words of the song, lit as they are sung - when
  [timings lie beside the song](/kilna/guides/the-short-style/#the-words);
- the stamp, the sign moving with the loudness;
- the short's cover at the end - the one on its Cover tab, or else the newest
  picture in its folder's `covers`.

Every block is the style's to turn off. The file goes into the short's folder
as the next `v{k}.mp4` under `video`: the build before it stays, and one
stopped halfway leaves nothing kilna would take for the short's file. When it
is done the tab plays it, and the cut list on the Cuts tab shows it with how
long it runs beside how long the stretches add up to - the cover's second is
the whole difference when all is well.

The same command stands on the Cuts tab under the cut list, and does the same.

## Finding the parts first

The plugin's second command, **Find the parts**, listens instead of building:
where each video the short is made from changes into its next part, and the
parts worth a short, each with its reason. kilna draws them on the Cuts tab -
see [Where to cut from](/kilna/guides/shorts/#where-to-cut-from). It takes a
few seconds a video, and is worth running again after the clip is rendered
again.

## When something is in the way

The Build tab says what stands in the way before you press anything: a video
with no finished file, no folder for the short, no stretches. A build that
fails says why in the plugin's words - a file ffmpeg cannot read, an ffmpeg
built without the subtitle filter the words need. A plugin older than 0.4 has
no look to build in and says which kilna it was made for; install it again.

See [Shorts](/kilna/guides/shorts/) for taking the stretches,
[The short style](/kilna/guides/the-short-style/) for what a short looks
like, and [Writing a plugin](/kilna/guides/writing-a-plugin/) for everything
kilna hands a builder.
