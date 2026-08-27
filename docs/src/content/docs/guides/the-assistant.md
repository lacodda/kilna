---
title: The assistant
description: How the AI panel works in kilna — it runs through your own Claude Code installation, a run belongs to its chat rather than to the screen you were on, and you can leave it working and come back.
---

kilna has a panel on a work's card that talks to an assistant about that work,
and a floating button in the corner of every screen that opens the same
conversations from anywhere. It does not carry a model of its own: it runs the
**Claude Code** CLI you have installed, under your own subscription and your
own session. If you have not installed it, the panel says so and everything
else in kilna carries on working.

The buttons above the composer come from your profile's prompts — *Critique the
lyrics*, *Suggest a title*, whatever the profile defines. Each one is a
template filled in with the work in front of you, so the same button means the
right thing in a music profile and in a novel one. Pressing one in the panel
does not send anything: it fills the composer with the rendered prompt, so you
read — and edit — exactly what is about to be sent and paid for. Enter does the
sending. See [The profile document](/kilna/reference/profile-document/) for how
the templates are written.

Typing **`/`** in the composer opens the same actions as a list you can filter:
a few letters of the name, arrows to move, Enter to choose, Escape to close. It
does what the button does — fills the composer — so the shortcut never means
something different from the thing it is a shortcut for. Matching is by word
starts, so `lyr` finds *Critique the lyrics*; a `/` anywhere but at the very
start of an empty composer is just a slash, because paths and dates have them
too.

The same actions appear a third time, on the work's **Overview** tab, where they
behave the other way round — see [Actions without the
panel](#actions-without-the-panel).

## Actions without the panel

Reading the prompt first is right when you are in the panel and the answer is
the point. It is in the way when you are working on the piece itself and simply
want the thing done.

So the profile's actions sit on the **Overview** tab as well, and there a click
starts the work immediately. Nothing opens, nothing waits for you: kilna says
which action went and leaves you where you were.

Each one gets a **chat of its own**, named after the action and the work —
*Critique the lyrics · Harbour lights*. It never lands in a conversation you
already have going, for two reasons: it would bury the answer under someone
else's subject, and it would hand the action that conversation's session as
context, so the reply would be shaped by whatever was being discussed.

While an action is working, its button says so and cannot be pressed again. The
same action on the same work will not start twice at once — and that holds even
if you close the card and open it again, because it is the run itself that is
remembered, not the screen. Another action on the same work, or the same action
on a different one, is a different thing and starts normally.

When it ends you are told wherever you are, with a link straight to the chat
holding the answer. A run you stopped by hand says nothing — you already know.

What comes back is an answer in a chat. The assistant never writes to your work
itself — **Insert as version** is how an answer becomes a version, and an action
that proposes a score puts the numbers in front of you with a button. You press
it, or you do not.

## Asking for many at once

The same actions are offered in the
[catalogue](/kilna/guides/the-catalogue/). Tick the works you mean and the bar
that appears carries the profile's actions next to **Delete** — a click asks
that action of every ticked work.

Each one becomes exactly the task a click on its own card would have made: its
own chat, its own answer, applied by you the same way. Nothing is done in bulk
except the asking.

Only three runs happen at a time, so a larger batch **queues**. kilna tells you
what it did with it — *"3 started, 37 waiting for a free slot"* — and a bar
across the top of the window counts down what is left, from whatever screen you
are on. **Drop the queue** clears what has not started yet; runs already going
are left alone, since their answers are already half paid for. Stop those from
the panel, one at a time, if you mean to.

A work already running that action is passed over rather than asked twice, and
so is one already waiting in the queue. That is why the number kilna reports
back can be smaller than the number you ticked.

The queue lives only as long as the application. Closing kilna with forty works
waiting drops the forty — nothing has been asked of them yet, and an application
that spawned forty processes on startup for a session you had finished with
would be worse.

## When it stops to ask

An action can come back with a question rather than an answer — it needed a
decision only you can make. That is the one thing about walking away that could
go wrong: the question would sit in a chat you are not looking at, and the work
would sit with it.

So it does not sit quietly. A banner appears above whatever screen you are on —
*the assistant is waiting on you in …* — with the chat one click away. It stays
until you answer or dismiss it, because it is asking for a decision and a
notification that fades is not. **Answer** opens the chat; **Nothing is needed**
clears it. Saying anything in that chat clears it too: replying *is* the answer.

The chat lists mark a waiting chat as well, in the card and in the drawer.

kilna works this out two ways. Every action asks the assistant to end with a
marker when it needs you, and — because an instruction can be ignored — the last
lines of the answer are read for a question as well. That second reading errs
towards asking: a banner you dismiss with one click costs a glance, a question
you never noticed costs the hour the work stood still. Only actions are read
this way. A question you asked yourself in the panel is already on your screen.

## Actions that come back with a score

Most actions answer in prose and you decide what to do with it. An action can
also ask for something kilna knows how to act on — the profile's `score` action
does, and any action can by declaring `"produces": "score"`.

Such an answer arrives with the numbers laid out under it, axis by axis, with
the assistant's one-line reason and a button that applies them. Applying writes
an ordinary score — same snapshot, same history, tied to the current version,
carrying that reason as its note. Nothing marks it as machine-suggested,
because once you have pressed the button it is your score.

If the answer judged an axis your profile does not have, or skipped one it does,
the panel says so rather than quietly dropping it. A proposal that only half
fits is still worth applying — but not without knowing.

The rule underneath is the same one everywhere in kilna: **the assistant
proposes, you apply.** It is never given a way to write to your workspace, which
is why a proposal is something you read before it becomes a fact.

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

**Insert as version** on an answer keeps it properly: the answer becomes a
version of the work, under the role you pick, exactly as written. It does not
become the current version unless you say so — an answer worth keeping is not
yet an answer worth standing behind. The assistant itself never writes to
your data; this button is you doing it.

## From anywhere

The floating button in the bottom corner carries a badge while runs are in
flight, and opens a drawer with every chat of the profile — the ones about
works and the ones about nothing in particular. **New chat** there starts a
conversation that belongs to no work; a chat about a work links back to its
card. Ask something, close the drawer, keep working: the badge says when the
answer has landed.

It is also what tells you an action from a card has finished, and the **Open**
on that message takes you straight into the chat holding the answer.

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
