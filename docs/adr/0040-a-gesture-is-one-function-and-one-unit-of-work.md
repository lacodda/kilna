# 40. A gesture is one function, and one unit of work

Date: 2026-09-28

## Status

Accepted. Builds on ADR 0014 (an operation records the intent) and ADR 0033
(every write a person makes is in the log).

## Context

A gesture - rename a work, plan a release, keep an assistant's proposal - has
a ceremony: find the profile it acts in, read what the change replaces, pick
the moment, write the rows, record the operation that asked for them, write
the journal line, bring the work's status in line. Until v0.83 each Tauri
command wrote that ceremony out for itself: sixty-nine copies in one file of
4,400 lines, and twelve more in the code that applies a proposal. The copies
had drifted - a release planned by a package left no line in the history - and
the tests reproduced the ceremony by hand a third time to drive the
workspace.

Each copy opened its own transaction, with `conn.transaction()`, which cannot
nest. So a proposal that wrote a work, three versions and a board was nine
transactions: a failure at the board left the work and its versions behind,
the message unmarked, and the next click made a second work.

## Decision

**A unit of work is a savepoint** (`db::unit::atomically`). The first unit on
a connection begins the transaction and commits it on release; a unit inside
another is a point to roll back to. A domain function that writes several rows
wraps them in a unit and stays correct alone, and the same function called
inside a larger unit becomes part of it. A unit dropped unreleased - by an
error, an early return or a panic - rolls itself back.

**What cannot be rolled back waits for the commit.** A file removed because the
row naming it was purged or detached is removed once the outermost unit
commits (`db::unit::after_commit`); a unit that rolls back drops what it
queued. Before this, emptying the trash and detaching a file removed the bytes
inside the transaction.

**A gesture is one function in `src/actions/`**, over one primitive:
`gesture(conn, "work.update", |act| …)`. The act carries the profile, one
moment, and the operation's params; the closure writes the rows and the journal
lines through it; the gesture records the operation after the closure returns.
There is no way to write through a gesture and forget the log - the only way
to record nothing is to say so, `act.unchanged()`, for a batch that moved
nothing. Several gestures in one unit are several operations landing together:
a work derived from another is `work.create` and `link.create`, or neither.

**Commands are adapters** (`src/commands/<domain>.rs`): they take the
connection and call a read in the domain or an action. `tests/operation_coverage.rs`
holds both halves from the source: no command calls a domain writer except
through `actions::`, and every writer called in `src/actions/` sits inside a
`gesture(…)`. The exceptions are named with their reasons: a chat's rows and
which profile is open are this device's, and the import predates the log it
starts.

**A proposal is kept whole** (`actions::proposal`). Everything that can be
known wrong before a row is written is checked first, all of it - a package
with a role the kind lacks and a release the work does not ship is refused with
both - and then the rows, their operations, their journal lines and the mark on
the message are one unit.

The operations' params are unchanged, byte for byte: replay and undo read the
logs of workspaces that already exist.

## Consequences

A new command is a few lines over an action that already has its tests, and a
test drives the workspace the way the window does by calling the action. A
package that fails while it is written leaves nothing - no work, no operation,
no line in the history - and the proposal still waits; the test that says so
checks that it failed after the check and before the end.

Undo still takes back one operation. A gesture that records several - a
package, a derived work - is undone one operation at a time; taking a whole
gesture back is its own stage.
