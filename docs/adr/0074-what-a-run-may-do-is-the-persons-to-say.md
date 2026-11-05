# 0074 — What a run may do is the person's to say

Date: 2026-11-05
Status: Accepted

## Context

Three things about a run had waited for a screen. How many may go at once was a
constant since v0.27, "a profile setting in the configuration stage". Which model
a run starts with could not be said at all: every run took whatever Claude Code
was set to, whatever the work. And ADR 0008 ended on a promise: when files could
really be attached to a work, what a run may see would become a setting, the
empty directory one of its options.

ADR 0008 also left a question open: whether seeing files should be opt-in per
action. And two of the three are about this machine rather than about the craft,
which argues against the profile the constant had pointed to.

## Decision

**Three columns on `assistant_settings`, beside the week's budget.** One typed
row, past the operations log, for the reason the budget is there: the money and
the memory are this machine's, whatever profile asked. The bounds of the limit,
one to eight, are in the schema as well as in the code.

**The limit lives in the registry while the application runs.** `Runs` holds it,
read from the row when the workspace opens and told again when it changes, since
the queue asks from threads with no connection in hand. A higher limit starts what
the queue holds at once; a lower one stops nothing and starts nothing more until
there is room under it. The window offers the numbers as eight segments, so a
number out of bounds is not something it can say.

**A model is a name, not a choice from a list.** The CLI knows which models exist,
its list moves under kilna, and a name it does not know it answers in the chat in
its own words. kilna checks only the look of a name, because the name becomes an
argument of a `.cmd` on Windows: letters, digits and `. _ - : [ ]`.

**What a run reads has two answers: `nothing` or `work`.** With `work`, a run in a
chat about a work is given the work's folder on disk and the folder of what is
attached to it, with `--add-dir`, and told where they are in its system prompt -
at every turn, because the CLI keeps no system prompt across resumes, and a
follow-up sees what the first saw. The directory it starts in stays the empty one.

## Why not the alternatives

**Per action.** The setting is about what a program on this machine may read, and
a person answers that once. Per action would be a second question on every action
of every profile, and a profile travels between machines while its folders do not.

**The working directory set to the work's folder.** A run's scratch files would
land among the person's renders. `--add-dir` gives the same reading and leaves the
writing where ADR 0008 put it.

**A list of files copied somewhere.** Nothing is copied into `assistant/`, for the
reason attachments are not: a second copy of a clip is a second truth and a slow
turn.

**The folder given without being named.** A run does not look in a folder nobody
told it about, so the setting would have done nothing visible. Hence the brief. It
names at most forty attachments and says how many more lie in the same folder.

## Consequences

A run that reads the web gets no folders, whatever the setting says: it has no
file tools, by ADR 0066, and is not told otherwise. A chat about no work has none
to get.

The attached files live together in `media/`, so giving a run their folder gives it
every workspace file in that folder. The brief names this work's, and it is told to
change nothing. This remains a default and not a sandbox, as ADR 0008 said: the CLI
is the person's own, with whatever it is allowed elsewhere, and the screen says so.

The preview of an action names the same paths before it starts, read through the
same call the run uses.

Moving a workspace to another machine carries the settings with the backup. They
are this machine's in the sense of the operations log: nothing syncs them.
