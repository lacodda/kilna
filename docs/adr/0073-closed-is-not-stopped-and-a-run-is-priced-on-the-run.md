# 0073 — A closed window is not a stopped one, and a run is priced on the run

Date: 2026-11-03
Status: Accepted

## Context

A task started from a card exists to be walked away from (v0.29), and its
end was told by a toast inside the window. Three things were still missing
for a person who actually walks away: closing the window ended everything it
had started - the runs were stopped at exit, the queue dropped, a plugin's cut
killed with its job object; nothing told a person outside the window that an
answer had landed or that tomorrow's release was not ready; and the money
the runs cost was visible only per chat.

The price of a turn was kept as `cost_usd` in the meta of the answer it
wrote. A turn the CLI ends with an error result carries `total_cost_usd` as a
finished one does, but writes no answer, so its price had nowhere to go and
was dropped. The run's own row already settled its `state` and `detail` from
the event that ended it.

## Decision

- **Closing the window with something going hides it** (`tray::close_requested`).
  "Something going" is one count, `Activity` (`src-tauri/src/activity.rs`):
  assistant runs and plugin commands alive, and tasks waiting in the queue.
  The registries say when they change through a `Watcher`, and the window hears
  the count as `app:activity`; the close button, the badges and the tray read
  the same number, so they cannot disagree about whether kilna is busy. With
  nothing going a close is a close, as before.
- **The tray is the backend's, its words are the page's.** The icon, its
  click, **Open** and **Quit** are built in Rust (`tray.rs`): a door back to
  the window must open whatever the page is doing. What the menu says is sent
  by the page in its language (`set_tray_words`), and the badge on the tray and
  the taskbar is drawn by the page on a canvas, in the theme's accent - the
  system only takes a picture, and Windows has no badge count, only an overlay.
- **The tray stays after the work is done**, saying so, until the window is
  opened or kilna quit. An icon that vanished at the end would leave the person
  to guess whether the answers are there.
- **One kilna per workspace.** Hiding makes a second launch likely - from the
  Start menu, while kilna waits in the tray - and a second process on the same
  workspace would sweep the first one's live runs as broken at its start.
  `tauri-plugin-single-instance` brings the open window back instead. Not for a
  run on another workspace (`--workspace`): a rehearsal on a copy must stand
  beside the real one, and the MCP server (`--mcp`) never reaches it.
- **What is told outside the window is the news the history gained**
  (`journal::record_news`): a once-entry is news when it is written and never
  after. A release unready the day before its slot writes its own line
  (`release.notReadyTomorrow`, once per release and date) and is said; the same
  gap at the next start is not. A week over its budget is said the same way
  (`assistant.overBudget`, once per week and budget). In front of the window it
  is a toast; not in front, a notification whose click brings the window back
  on what it was about (`lib/outward.ts`).
- **Endings out of sight are gathered** (`lib/endings.ts`): said at once when
  nothing else is going, otherwise together when the last run ends and at most
  ninety seconds after the first. A question typed before the window lost focus
  is told too: the reason it was not - the person is looking - no longer holds.
- **A run's price is the run's**, `chat_run.cost_usd` (migration 0048), written
  from the event that ends it, finished or failed. It leaves the answer's meta:
  the chat list, the chat's Markdown and the spend all sum one column. The
  migration moved every priced answer's number onto its run; an answer whose
  run is unknown keeps its number unread.
- **The day and the week are the window's.** The spend is counted from the
  moments the window's midnight and Monday fell on, which it sends; the backend
  only knows UTC (the v0.23 lesson).
- **The weekly budget is soft, and a column.** `assistant_settings` is one row
  (`CHECK (id = 1)`) with a column per setting, so that a budget of nothing is
  not a value the table can hold; the configuration stage adds its columns
  there rather than a table of loose keys.

## Consequences

- A window closed mid-batch no longer loses the batch; quitting from the tray
  is now the way to stop everything at once.
- The tray, the notifications and the badge cannot be seen by the jsdom tests;
  they are checked by a rehearsal through the window, with a stand-in CLI.
- `kilna --mcp` still opens the workspace through `AppState::open`, whose sweep
  marks running runs broken; noted as a wish to settle separately.
