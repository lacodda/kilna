# 0008 — Assistant runs start in an empty directory

Date: 2026-08-25
Status: Accepted

## Context

The assistant is the user's own Claude Code CLI, spawned as a child process.
A child inherits the working directory of its parent, and until now kilna never
set one — so a run started wherever kilna itself happened to be started from.
In development that is `src-tauri`, and a live probe in v0.27 caught the
consequence: asked about "the README", the assistant read kilna's own source
tree and answered about that. In production the inherited directory is whatever
the launcher used, which is no better — it is merely harder to predict.

The directory matters for trust as much as for correctness. Everything a
prompt is supposed to carry — the work's body, its versions, its title — is
rendered into the prompt text before the CLI is called. The process has no
legitimate need to find anything on disk, so whatever it *can* see from its
working directory is pure exposure.

Three candidates were on the table:

- **nothing** — a dedicated directory that is kept empty;
- **the workspace directory** — where the SQLite database and backups live;
- **the work's assets** — the folder of files attached to the work.

## Decision

Runs start in a dedicated directory, `assistant/`, created next to the
workspace database and deliberately left empty. `AppState::assistant_dir`
prepares it; the blocking turn and the streamed run both set it as the child's
working directory. If the directory cannot be created, the run proceeds
without one rather than failing — a broken filesystem should degrade the
sandbox-of-convenience, not the feature.

The workspace directory was rejected because it hands the model the raw
database and every backup — the one place in kilna where all of the user's
writing sits in a single file. The work's assets were rejected for now because
the `asset` table has no writer until the assets stage; when files can really
be attached to a work, pointing the run at them becomes worth designing —
including whether that is opt-in per action.

## Consequences

This is a **default, not a sandbox**. The CLI can still read any absolute path
its tools are allowed to touch; kilna does not restrict its tool set. What the
decision removes is accidental context — a run that decides to look around
sees an empty directory instead of kilna's sources or the user's launcher
directory.

A run that writes files (the CLI may create scratch files when asked) leaves
them in `assistant/`, where the user can inspect and delete them. kilna does
not clean the directory; nothing in it is ever read back.

When the assets stage arrives, "what a run may see" becomes a real setting and
this default becomes one of its options.
