# 0009 — A queued task lives in memory

Date: 2026-08-27
Status: Accepted

## Context

Three assistant runs may be alive at once (`PARALLEL_LIMIT`), because each
spawned CLI costs a few hundred megabytes. Until v0.31 a request past that
limit was refused outright, and the panel showed the refusal as a sentence.

That is the right answer for someone typing in the panel. They are watching the
screen, and "three runs are already going" is information they can act on —
wait, or cancel one.

It is the wrong answer for a batch. v0.31 offers the profile's actions over a
catalogue selection, and a selection is routinely larger than three. Ticking
ten works and receiving three runs and seven errors is not the limit doing its
job; it is the feature not working. The person asked for ten and has no way to
express "and the rest when you can" other than sitting there re-clicking.

So work past the limit has to wait somewhere. The question is where.

Two candidates:

- **memory** — a queue held next to the run registry, gone when the process is;
- **a table** — queued tasks written to the workspace, resumed on the next
  start.

## Decision

The queue is held in memory. A queued task is lost when kilna closes.

## Consequences

A queued task has produced nothing yet. There is no chat, no rendered prompt,
no row anywhere — only the pair *(action, work)*, which is exactly what a task
key already is. Nothing is lost that was written down, because nothing was
written down.

Persisting it would mean an application that starts spawning CLI processes on
launch, under the user's own subscription and at their own cost, for work asked
of a session that is over. A person who queued forty works, saw it would take
the evening, and closed the application has already said what they wanted. A
workspace that resumed the batch a week later on a different machine would be
obeying a stale instruction.

The queue therefore says so plainly rather than pretending otherwise: the
counter across the top of the window is what is left *this session*, and
**Drop the queue** empties it.

What is deliberately *not* symmetric: dropping the queue leaves runs already
going alone. Their answers are partly paid for, and killing three live
processes is a different act from cancelling work that has not begun. Those are
stopped one at a time from the panel.

The queue shares its key space with the run registry — both name a task the
same way — so a card's button reads "busy" whether the task is third in line or
already talking to the CLI, and a batch never queues a second copy of something
already running.

A slot frees in exactly one place: when `pump` finishes reading a run. The
queue is drained there, in the thread that run owned, rather than on a timer —
that is the only moment anything is known to have changed. A queued task that
cannot start (its work deleted, the profile's action gone) is dropped rather
than retried, because those reasons do not improve with waiting.

If a later stage makes the limit a setting, or introduces batches that should
outlive a session, this is the decision to revisit — and it would need an
answer to "what happens when the workspace opens with work queued", which this
one avoids by not having the situation.
