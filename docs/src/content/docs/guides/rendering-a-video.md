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
  newest track on its Files tab, or else the finished track of the work it
  is made from: a clip renders over the song's audio.

The video goes to the work's folder, which it has once the workspace's media
folder is chosen; until then it goes to the folder the plugin's setting
**Where a video goes when the work has no folder** names.

Two presets:

| Preset | Frame | What it is for |
| --- | --- | --- |
| **Stream, 1080p** | 1920x1080 | A channel that publishes often: the fastest encode, a still picture, AAC at 192k, a six-second fade to black and silence at the end. |
| **Master, 4K** | 3840x2160 | A master: a slower encode, AAC at 320k, the file's index at its front so it plays while it arrives. |

**Bring the sound to the loudness** brings the track to -14 LUFS - what
YouTube and Spotify play at - with its peaks under -1 dBTP.

## The Build tab

Since v0.149 a work that goes out itself - a clip, an audio release, a
short - has a **Build** tab while the plugin is installed and on. It lays the
video out on two tracks:

- **V1 - the picture**: the cover, a still or a loop, and the bar of the
  whole video - the picture held **before the sound**, the sound, the picture
  held **after** it - with its fades **from black** and **to black**.
- **A1 - the sound**: the track's waveform, what plays lit and the rest
  dimmed. Drag an edge of the lit part to trim it, or type **From** and
  **To**; its **fade in** and **fade out**; its **gain**, from -24 to +18 dB,
  by the slider or a decibel at a time. **Optimal loudness** measures the
  track and sets the gain that brings it to the loudness set in the plugin
  (-14 LUFS unless set otherwise); the gain says **clips** in red when the
  track's peaks would go over full scale.

**Preview** plays it all before anything is rendered - the picture with its
fades, the sound with its trim, gain and fades - by the button or the space
bar, and the slider under it goes to any second.

What is set is kept in the work's fields under `simple_video`, a moment after
the last change, as an ordinary edit: undo takes it back. It is the shape the
plugin renders by - see
[the plugin's README](https://github.com/lacodda/kilna-plugin-simple-video#the-works-own-timeline)
- so the preview is what the render makes. **Render the video** runs the
render, with its progress and **Stop**.

**Render by the preset** is the same render with no editor, marked in
Settings as **for many works at once**: a render that needs nothing but the
preset, or each work's own timeline.

## Measuring the loudness

**Measure the loudness** reads the track once and keeps what it found in the
work's fields, under `audio_loudness`: its integrated loudness, its true
peak, its range, and how many dB it is from the loudness set. Nothing of the
track is changed.
