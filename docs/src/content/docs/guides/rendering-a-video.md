---
title: Rendering a video
description: Install the simple-video plugin and render a work's cover over its track into a video in its folder - by a preset or by the work's own timeline - and measure how loud the track is.
---

kilna keeps a work's track and its cover, and never opens a video itself. A
simple video - the cover, or a short loop, over the track - is made by a
plugin, [kilna-plugin-simple-video](https://github.com/lacodda/kilna-plugin-simple-video)
(v0.148): **Render the video** on a work, and `<title>.mp4` lies in the
work's folder, where kilna finds it as the work's
[finished file](/kilna/guides/files-and-covers/#the-finished-file) when the
kind's rule would not find one by itself.

## Installing it

1. Install **ffmpeg**, which brings **ffprobe** with it (`winget install
   Gyan.FFmpeg` on Windows). Without them kilna lists the plugin with the
   reason instead of its buttons.
2. Install the plugin. On Windows, in PowerShell:

   ```powershell
   irm https://raw.githubusercontent.com/lacodda/kilna-plugin-simple-video/main/tools/install.ps1 | iex
   ```

   On macOS and Linux:

   ```sh
   curl -fsSL https://raw.githubusercontent.com/lacodda/kilna-plugin-simple-video/main/tools/install.sh | sh
   ```

3. In **Settings → Plugins** it stands with its three commands and its
   settings: the preset, the codec, the quality, the frames a second, the
   loudness, the fades.

## Rendering

**Render the video** stands in the head of a work's Overview. It takes:

- **the picture** - the work's cover: a still, or a loop that plays over and
  over for as long as the video runs, fitted into the frame over black;
- **the track** - the work's finished file when it is a track, or else the
  finished track of the work it is made from: a clip renders over the song's
  audio.

Two presets:

| Preset | Frame | What it is for |
| --- | --- | --- |
| **Stream, 1080p** | 1920x1080 | A channel that publishes often: the fastest encode, a still picture, AAC at 192k, a six-second fade to black and silence at the end. |
| **Master, 4K** | 3840x2160 | A master: a slower encode, AAC at 320k, the file's index at its front so it plays while it arrives. |

**Bring the sound to the loudness** brings the track to -14 LUFS - what
YouTube and Spotify play at - with its peaks under -1 dBTP.

A work can keep a timeline of its own, under `simple_video` in its fields:
what of the track plays, how loud, its fades, the picture held before and
after the sound. A render reads it when it is there - see
[the plugin's README](https://github.com/lacodda/kilna-plugin-simple-video#the-works-own-timeline)
for its shape. The timeline editor that writes it comes in v0.149.

**Render by the preset** is the same render with no editor, marked in
Settings as **for many works at once**: a render that needs nothing but the
preset, or each work's own timeline.

## Measuring the loudness

**Measure the loudness** reads the track once and keeps what it found in the
work's fields, under `audio_loudness`: its integrated loudness, its true
peak, its range, and how many dB it is from the loudness set. Nothing of the
track is changed.
