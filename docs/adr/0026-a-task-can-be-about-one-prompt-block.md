# 0026 — A task can be about one prompt block, and a revision is laid over the scene

Date: 2026-09-15
Status: Accepted

## Context

A scene carries several prompt blocks — a still frame, an animation, a
negative — and regenerating one of them is the commonest thing a person does
with a board: the frame is right and the motion is wrong. Until now the
finest an action could be aimed was a whole scene (ADR 0023), and a scene
action came back proposing the scene, blocks and all.

Two things stood in the way of aiming finer.

A task is identified by a key — `action:work:scene` — and the registry
refuses a second run of the same key (a person means "this is already
running"). Two blocks of one scene share that key, so regenerating the
animation would be refused while the still was going, and the button for one
would grey out for the other.

And applying a revision set `blocks` as a whole map, because the board's own
editor does: it edits one box and sends them all. An answer carrying only the
block it was asked for would therefore delete the two it said nothing about.

## Decision

**A task about one block carries the block as a fourth segment of its key** —
`action:work:scene:block`. Two blocks of a scene run side by side; the same
block twice does not. The key is built the same way on both sides, as it has
been since tasks existed: the window draws the busy button from it, the
backend refuses a second run by it, and a difference between them is a button
that lies.

**A revision's blocks are laid over the scene's, not put in their place.** A
block the revision does not name is kept; a block whose text comes back empty
is cleared, which is what emptying the box on the board means. This is what
makes a single-block task safe, and it is also the more honest rule for a
whole-scene revision, whose instruction already says a field left out is
kept.

**The action is held to the block it was aimed at.** The instruction names it
("give the `motion` block only"), and an answer that writes another block is
refused rather than applied — the same way an answer about another scene is
refused. Applying it would work, since blocks are laid over; but it would
rewrite a prompt nobody asked about, and quietly.

**The block is a parameter of the task, not a third scope.** The action is
the same action — *Prompts for the scene* — aimed at less. A `block` scope
would mean a second action in every profile that already has a scene one, and
two vocabularies to keep in step.

The scene is still rendered whole into the prompt: the other blocks are what
a good animation is written against. Only the answer is narrowed.

Rejected: a block-level `produces` (`scenes:block`) — what it produces is a
revision either way; a separate `regenerate` action per block in the shipped
profiles (three actions where there is one, and a profile that gains a block
would need a fourth); narrowing what `{scene}` renders (the context is the
point).
