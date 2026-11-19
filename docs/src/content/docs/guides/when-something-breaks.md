---
title: When something breaks
description: What to send when kilna misbehaves - the diagnostics in one copy, an issue already written from a crashed screen, and how to tell whether a newer kilna is out.
---

kilna sends nothing about you anywhere by itself: there is no telemetry. When
something goes wrong, what to send is gathered on a press and goes where you
paste it.

## Copy the diagnostics

**Settings → About kilna → Copy the diagnostics** (v0.162) puts one text on
the clipboard: the version, the system, the WebView the window is drawn by,
the assistant's CLI as it answered, and the last errors of kilna's own log.
Any path in those errors that runs through your home folder is written `~`,
so the report does not carry your user name. **Open a new issue** opens the
project's issues to paste it into.

## A screen that stopped working

When a screen fails, kilna shows a panel instead of it, writes the error into
its log, and offers **Try again** - and **Report it on GitHub**, which opens a
new issue already written: the error's message and the top of its stack, a
line for what you were doing, and nothing of yours - paths under a home
folder are cut to `~`. Read it before you send it; add the diagnostics if you
can.

## Is a newer kilna out?

**Check for updates** on the same page asks the project's public releases and
nothing else, only when pressed. When a newer version is out, **Download**
opens its page; when it cannot tell - offline, or the page is down - it says
so rather than guessing.
