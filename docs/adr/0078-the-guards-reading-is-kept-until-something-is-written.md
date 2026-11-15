# 0078 — The guard's reading is kept until something is written

Date: 2026-11-15
Status: Accepted

## Context

The guard of repeats is derived (ADR 0054): every time it is asked, every sung
text is stemmed, the register is matched in each, and every song that has not
gone out is held against every song that has a day. Nothing is stored, so a
rewrite that drops a word drops the finding.

The calendar asks it about a day - `preview_schedule`, "what would this
release repeat if it went out on the 24th" - and asked by reading the whole
workspace again for that one day. Measured on the owner's workspace in v0.96:
0.75-0.9 s a day in a debug build. A chip carried from Monday to Friday
crosses five days, each day under the pointer ordered its own reading, and the
backend answers one command at a time on one connection - so the date written
when the chip was let go stood in line behind them, 4-6 s after the drop
(wish 4764).

What the day changes in a reading is one thing: whether a rare word's
neighbour said it within the window of that day - red - or outside it -
orange. Everything else - the stems, the register's hits, which songs share
what, which pairs the owner kept - is the same for every day asked about.

## Decision

**The guard reads once and answers by day.** `register::guard::read` returns a
`Reading`: for every song that has not gone out, what it shares with each
neighbour, each rare word with its neighbour's day. `Reading::on(day)` and
`Reading::at_slot(work, day)` decide red from orange, pick the loudest
neighbour a stem and sort - no SQL but a work's ancestors, no stemming.
`guard::all`, `of_work` and `at_slot` are a reading and one question of it, so
what they say is what they said.

**The window keeps the last reading until something is written**
(`AppState::guard_reading`). It is kept with what it was read from: the space,
the profile, and SQLite's two counters of writes - `total_changes()` for this
connection, `PRAGMA data_version` for every other one, which is how a commit
by the agent's server is seen. A dry run takes the kept reading when all four
still match and reads again when any moved. Only the calendar's dry runs use
it; the marks, the card and the agent read fresh, as before.

**A day is asked about once the pointer has rested on it** (150 ms), in the
window: the days a chip crosses are not asked about at all. And **the date is
written before the day is asked what it repeats**: the answer is for the line
that says the release landed, and does not depend on whether the release
already stands on the day.

## Consequences

- A carry costs one reading, and a day already asked about during it answers
  from the window's own cache. The write no longer waits for any of it.
- Nothing is stored and nothing can go stale across a write: the kept reading
  is a derivation remembered for as long as its inputs provably have not
  changed, not a second truth. ADR 0054 stands as written.
- The counters are coarse - any write to the space drops the reading, a
  journal line as much as a rewritten lyric. That errs towards reading again,
  which is the safe side, and a carry writes nothing until it ends.
- A work that has no release yet is asked about the same way
  (`release::Landing::Work`, the `preview_day` command): the day's `+` and a
  work carried from the pane hear what its song would repeat before it lands.
  Until now the window read such a day off its own list of releases and said
  nothing of repeats.
- Measured on a copy of the owner's workspace, debug build: see the v0.111.0
  entry of the record (`rigger event`), where the before and after stand.
