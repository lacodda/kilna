# 0061 — An experiment is a work, and its trials are rows of its board

Date: 2026-10-09
Status: Accepted

## Context

The owner's search for a sound lived in six markdown files outside kilna -
about a hundred and forty style prompts for a music generator, a few of them
a year old. Each file had the same parts under different headings: a
direction, a few anchors every prompt had to keep ("without them the
generator slides into plain rock"), who to listen to (never written into a
prompt), a vocabulary of phrases with what each does, ways of setting one
thing on another, series of prompts - a sweep of the field, variations of one
core moving a single thing at a time, reworks of an existing song's style -
and a table of what to do when a run went wrong. What the files did not hold
was the result: which prompt was run, what came out, what was kept and where
it went. The only memory of a kept prompt was a version of some song's
style, with nothing saying where it came from.

The owner asked for this to be done in kilna, "standardly, not as a wall of
text", and added: either it is general, not tied to music and one
generator, or it is narrow and then a plugin. A novelist trying five openings
of a chapter, a blogger trying ten headlines, are doing the same thing.

## Decision

**An experiment is a work** of a kind the profile marks with `lab`. It is
not a new screen and not a new table of its own: a card, versions (its brief
and its findings), fields (the anchors, who to listen to, the direction),
notes, links, a history, the trash, undo, the export, MCP and search all come
with being a work. Studio ships the kind `experiment`; any profile may
declare one under any key.

**A trial is a row of `trial`** (migration 0038), on the experiment's board:
its series, the trial it varies (`parent_id`, recorded where the variation is
made, never guessed - ADR 0055), what it moves (`angle`), its text (`body`,
the truth), the bricks it was picked from (provenance, not a second copy),
who to listen to (`reference`, never part of the text), what came out
(`outcome`), one verdict - kept, dropped, or none (ADR 0050) - the version of
a work's text it reworks (`source_version_id`), and whether a run starts
there. Takes are assets of the trial (`asset.trial_id`), played where it is
judged. Field clocks and tombstones as every table a person's work lives in
(0011).

**What a kept trial becomes is recorded where it went.** The lab names the
role its trials go into (`lab.harvest`: Studio's `style`). Into a work: a
version of that role, written from the version the trial reworks when that
is one of the work's - a branch beside its original - and not made current
unless asked: a found sound is put beside the song's style, not over it.
Into the dictionary: a brick of a type the trial is read against. A new
work: its first text is the trial's. Each remembers the trial
(`work_version.trial_id`, `style_brick.trial_id`); the board shows where a
trial went by asking. Material is spent, not moved (ADR 0045). Only a kept
trial goes anywhere - the version's and the brick's own way in refuses
anything else, so every path (the board, a package, a replay) is held to it.

**A trial is read as the text it would become.** The composition that writes
the harvest role (ADR 0060) reads a trial's body - phrases marked with what
they mean, unknown tags offered to the dictionary - and writes one from the
dictionary, closed by the experiment's own fields. The anchors are a field
the lab names (`lab.anchors`), one to a line; a trial whose text lost one is
marked.

**A link of the lab is not a link of making.** An experiment made to rework a
song, and a song found in an experiment, are linked with the role `lab`.
The card shows the link like any other; the walks that say what a work is
made of (`link::descendants`, `ancestors`) do not cross it. Otherwise a song
found in an experiment would take the experiment for its origin - its
folder, its `{origin}` - and a song an experiment reworks would count as
released when a song found there went out.

**The assistant proposes trials** through an action of the scope `lab` that
produces `trials`, as a cover's board asks for ideas: a sweep of the field,
variations around a core, or the fix of a trial that was heard. What was
asked travels in the task's key, because the answer is read by it: a
variation is the child of the trial it varies, whatever the answer says. The
board's kept trials go in as examples and the dropped ones as anti-examples;
the phrases of the dictionary with their ids as `{choices}`. An agent
proposes the same shape with `propose_trials`, or a whole experiment with
its board in `propose_work`.

The exchange with the generator itself - sending a trial, fetching its takes
- is narrow and stays outside the core: a plugin's job, later.

## Consequences

- An experiment is never judged on axes, booked or released: its statuses
  name only a draft among the derived meanings, and the profile test says so
  for a lab kind alone.
- A trial in the trash takes its harvest's memory with it - the column is set
  to nothing - and the trash keeps the list (`spent`) to point it back on
  restore. Restoring any row whose parent, trial or source is still gone
  now names nothing rather than failing on the foreign key (`trash::SOFT`),
  which also closes the same hole a version's parent had since v0.50.
- The export is format 4: an experiment's page carries its trials by series
  with their verdicts and where they went, a version taken from a trial says
  so - and a publication's page carries its cover and its board of ideas,
  which format 3 left out.
- Studio's fields that say nothing about an experiment (a length, a
  language, a tagline, a point of view, a premise) now name the kinds they
  belong to.

Rejected: an experiment as a note of its own kind (no verdicts per trial, no
series, no harvest - the wall of text again); a table and a screen of its
own (rewrites a card kilna already has); trials as versions of the song
(versions are the history of one text, trials are rivals, and a sweep of the
field belongs to no song yet); a copy of atlas's laboratory; the whole
feature as a plugin (a plugin owns no tab, no trash and no undo).
