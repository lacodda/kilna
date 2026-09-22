---
title: The assistant
description: How the AI panel works in kilna — it runs through your own Claude Code installation, a run belongs to its chat rather than to the screen you were on, and you can leave it working and come back.
---

kilna has a panel on a work's card that talks to an assistant about that work,
and a button in the title bar that opens the same conversations from any
screen. It does not carry a model of its own: it runs the
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
which action went and leaves you where you were. Only the actions for the
work's kind are there — a critique of lyrics is not offered on a video,
because the profile says which kinds each action is for.

Beside every button is an **eye**. It opens exactly what the click would send:
the message with the work's text filled in and the instruction kilna appends,
and the method the run is briefed with. It is the same text, composed by the
same call — the preview and the run cannot part. From there you can also hand
the run **reference files**: paths, one per line, or chosen with the file
dialog. They are named in the message with an instruction to read them first,
and the run is given leave to read their folders; a path that is not a file is
refused in the preview, not by a run ten minutes later. **Start** sends it.

They sit under the text on the **Versions** tab too, and there they are about
the revision you have open — not whichever is current. The template reads that
revision, and what comes back is bound to it: a score applied from that chat is
a snapshot of that revision, a critique kept as a version says which revision
it discusses and is shown beside it. The line over the buttons names the
revision, so there is no guessing which text a critique read.

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

## An action's method

A message with placeholders says *what* to do. How it is done — the role the
assistant takes, what it checks and in what order, the shape of the answer,
what it must never say — is the action's **method**: a markdown document of
its own, shipped with the profile and edited in Settings beside the message.
It reaches the model as a system instruction on every turn of the chat the
action opened, so a follow-up question in a critique chat is still answered
by the critic.

Studio ships two: a critique of lyrics in blocks — rhymes, images, structure,
singability, fit with the craft's DNA — with a section on the craft's DNA
written to be rewritten for your own project; and a way of judging finished
work along the profile's axes. The other profiles ship the second. Rewrite
them in **Settings → Assistant actions**; the key of an action stays fixed,
because a running task and its chat are recognised by it. An action can also
be added there, or removed.

Where an action is started from the panel's composer, the method does not
ride along: the composer is for reading the message before it is sent, and
the message is all that goes. Start the action from a card to have the
method with it — the eye on the card shows the method too.

An action is refused before it opens a chat when it would send a hole: a role
the work has no version in yet (*“Harbour lights” has no Plot yet: write it
first*), a donor the work is not made from, a scene action started without a
scene. And a template cannot lose the placeholder that carries the text
without Settings saying so at save — see
[the profile document](/kilna/reference/profile-document/#template-placeholders).

## Actions that come back with a score

Most actions answer in prose and you decide what to do with it. An action can
also ask for something kilna knows how to act on — the profile's `score` action
does, and any action can by declaring `"produces": "score"`.

The instruction kilna appends names the axes as your profile defines them —
label, the question each asks, the marks of its rubric, its weight — and the
tiers with what the total means, so the model judges by your rubric rather
than by its own idea of a hook.

Such an answer arrives with the numbers laid out under it, axis by axis, with
the assistant's one-line reason and a button that applies them. Applying writes
an ordinary score — same snapshot, same history, tied to the revision the
action was started on (the current one, from the overview), carrying that
reason as its note. Nothing marks it as machine-suggested, because once you
have pressed the button it is your score.

An action can also produce a **version**: `"produces": "version:critique"`
offers the whole answer as a version in that role, with the same *Insert as
version* button an agent's proposal gets. Studio's critique does, so a
critique is kept beside the text it read rather than lost in a chat.

If the answer judged an axis your profile does not have, or skipped one it does,
the panel says so rather than quietly dropping it. A proposal that only half
fits is still worth applying — but not without knowing.

## Actions on the board

A kind with a storyboard — Studio's *Video* and *Short* — has its actions on
the **Scenes** tab as well, above the board, and an action can produce
**scenes**: `"produces": "scenes"` asks for the whole board, and the answer
comes back as the same proposal an agent's `propose_scenes` makes — the board
as a table, every block under it, **Replace the board** under that. Studio
ships three:

- **Plot from the source** reads the lyrics of the song the video is
  [made from](/kilna/guides/made-from/) and writes the plot — kept as a
  version in the `plot` role. A video made from nothing is told to link its
  source first.
- **Storyboard from the plot** reads the plot, the context and the board as it
  stands, and proposes the board: sections, kinds of shot, descriptions — no
  prompt blocks, those come per scene.
- **Prompts for the scene** sits on each row of the board. It reads the
  context, the board for continuity and the scene itself, and proposes the
  scene's prompt blocks as a **revision** — only that scene, only the blocks;
  the rest of the board is not touched. Scene 2's prompts can be written while
  scene 1's are still going.

An empty board is an entrance to these, not an empty tab: it says to write the
plot and the context and let the action draw the board. When an answer's block
cannot become a proposal — a kind of shot the profile does not have, a
revision that numbered another scene — the chat says why under the answer
rather than showing nothing. The methods behind the three are in **Settings →
Assistant actions** like any other, and the words they use — the kinds of
shot, the blocks — are the profile's own.

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

The assistant's button in the title bar, beside the bell, carries a badge
while runs are in flight, and opens a drawer with every chat of the profile — the ones about
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

The reply lands in the chat when the run finishes, with what it cost and how
long it took — the two facts the CLI reports about a finished turn, under the
answer it belongs to.

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

## An agent outside the window

The same rule works the other way round. `kilna --mcp` serves your workspace
to an agent outside kilna — a Claude Code session in a terminal, say — over
the Model Context Protocol: it can read your works, their versions, scores
and calendar, and it can *propose* a version, a score, a note, a storyboard
for a video, or a whole work as one package. The proposal lands in a chat on
the work, named after the agent, with the same buttons the panel's own
proposals have: **Insert as version**, **Apply**, **Add as note**, **Add to
the board**, **Create the work**. The bell counts each one. See [MCP server](/kilna/reference/mcp/)
for the tools and how to register it; the **Settings** screen shows the
command for the build you are running.

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
