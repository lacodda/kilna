---
title: The assistant
description: How the AI panel works in kilna — it runs through your own Claude Code installation, a run belongs to its chat rather than to the screen you were on, and you can leave it working and come back.
---

kilna has a panel on a work's card that talks to an assistant about that work.
It does not carry a model of its own: it runs the **Claude Code** CLI you have
installed, under your own subscription and your own session. If you have not
installed it, the panel says so and everything else in kilna carries on
working.

The buttons above the composer come from your profile's prompts — *Critique the
lyrics*, *Suggest a title*, whatever the profile defines. Each one is a
template filled in with the work in front of you, so the same button means the
right thing in a music profile and in a novel one. Pressing one does not send
anything: it fills the composer with the rendered prompt, so you read — and
edit — exactly what is about to be sent and paid for. Enter does the sending.
See [The profile document](/kilna/reference/profile-document/) for how the
templates are written.

## Chats

A work can carry several chats — one per question worth keeping apart. The
row of chips above the conversation switches between them; **+** starts
another, and the menu next to the chips renames or deletes the open one.
Deleting a chat is one of the few truly irreversible acts in kilna, and it
asks first.

A chat does not exist until something is asked in it: the first message
creates it, and an unnamed chat borrows its first question as its name.

Answers render as text with formatting — headings, lists, code. Every code
block carries a copy button, and every answer has one for the whole reply.
The header shows what the open chat has cost so far, summed from what each
answer reported.

## A run belongs to the chat

Asking something starts a **run**. The run belongs to the work's chat, not to
the screen you happened to be on when you asked: you can leave the card, open
another work, go to the calendar and come back. The run keeps going, and the
panel picks it up where it is.

That also means nothing is lost when you look away. Everything a run says is
stored as it arrives, so coming back replays what happened rather than showing
a gap — the same view you would have seen had you stayed.

## Watching one work

While a run is going the panel shows what it is doing rather than a spinner:

- the tools it uses, one line each, with the telling argument — the file being
  read, the pattern being searched for;
- blocks of the answer, as they arrive.

The reply lands in the chat when the run finishes, with what it cost.

**Stop** ends a run. What it had already said stays — a stopped run is a short
answer, not an erased one. Stopping is not instant: if the answer arrives in
the moment between the click and the process ending, kilna keeps the answer
rather than throwing away something you can read.

## Several at once

Up to **three** runs can be going at the same time. Each one is a separate CLI
process costing a few hundred megabytes, so the limit is about your machine
rather than about queueing: asking for a fourth is refused with a sentence
saying so, and nothing is sent.

## Where a run starts

A run is a process, and a process starts somewhere. kilna starts every run in
a dedicated `assistant/` directory next to your workspace, kept empty on
purpose: everything the assistant should know about a work arrives in the
prompt itself, so there is nothing for it to find on disk — and a run that
decides to look around sees an empty folder rather than kilna's own files.

This is a default, not a sandbox: the CLI's tools can still read a file by its
full path if you ask them to. Files a run creates land in that same folder,
where you can inspect or delete them; kilna never reads them back.

## When kilna closes mid-run

A run is a separate process kilna started, and it does not stop by itself when
the window goes: closing kilna stops the runs it was carrying, so nothing keeps
working — and spending — for an answer nobody will read.

If kilna is ended some harder way — a crash, or the task manager — there is no
chance to stop anything, and a run can carry on in the background with nowhere
to put its answer. Either way, runs left over are marked as interrupted the
next time you open kilna, and the panel says so rather than showing work that
nothing is doing. What a run had said before it stopped is still there to
read.
