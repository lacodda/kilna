# 38. Every screen holds the window, on dowel's primitives

Date: 2026-09-27

## Status

Accepted.

## Context

v0.75 said the shell never scrolls and each screen scrolls inside itself. The
audit of 24.09 found the rule kept halfway: five screens still flowed (the
dashboard, styles, history, trash, the style guide), the calendar scrolled as a
page below 1280px, and nine of twelve card tabs were pages that scrolled within
the card. Their heads went up with their content, a table's sticky header had
nothing to stick to, the assistant's chat was read through a 384px letterbox,
and the fields to add a note or a release followed the last row off the card.

The same audit counted what the screens were built from: 95 raw `<button>`s in
42 files, about 25 switchable chips in five recipes, a segment drawn by hand,
nine inline editors with their own ideas of Enter and Escape, six list-and-detail
layouts in five widths, nineteen failed loads with no way to try again, and
kilna's own Field, EmptyState, Skeleton, SaveState, Sparkline, TierRuler and
SegmentedScale beside registry copies that dowel had taken from kilna itself.
160 text sizes were written in pixels, half of them a step of the scale spelled
the long way.

## Decision

**No screen and no tab scrolls whole.** `ScreenSpec.scroll` and the tabs'
`held` rule are gone: every `<Screen>` is held, and so is the card's tab body.
What they draw lays itself out on `components/frame.tsx`:

- `Frame` - a column that takes the height: a head that stands, a body that
  takes the rest, a foot that stands. Ten pixels between its blocks.
- `Scroll` - the part of a frame that scrolls, on dowel's ScrollArea: an overlay
  bar that takes no width, the line's scrollbar.
- `Pane` - a panel with a bar at the top, a scrolling body and a bar at the foot.
- `ListDetail` - a list beside what is open in it: a list's width (264px) or a
  rail's (216px), one gap, the list on either side.

The screen's margins are the mockup's: 12 under the title bar, 16 at the sides,
14 above the window's edge.

**A screen presses dowel's primitives.** `dowel/no-raw-button` is on: an action
is a Button, a row is a RowButton, a switch is a Chip with `pressed` or a
ChipGroup, one of a few is a SegmentedControl, a value edited in place is an
InlineField, a copy is a CopyButton or a Copyable, a box is a Checkbox, a number
a NumberField. Six raw buttons remain, each disabled on its line with the reason:
two for dowel's NavRail and one for its TableSortHeader, which kilna has not
taken yet; two drawn on a work's own colour; one a pressable picture.

**The states are the registry's.** Field and FieldGroup (a Field for an input it
can hand its id to, a group for anything else), EmptyState in its three kinds,
Skeleton and its shapes, SaveState, Sparkline, TierRuler and AxisBar. A read
goes through `components/Loaded`, which hands a query to dowel's QueryState: a
failed load says what failed and offers to try again, and an empty one invites
the one action that fills it.

**Sizes come from the scale.** `dowel/no-arbitrary-scale` is on outside the
registry's copies; the mockup's half steps went down to the scale's.

## Consequences

The smoke test holds the frame: `<main>` and the tab body clip and never scroll,
and the one root they draw takes the height. A root that grew with its content
would be cut off at the window's edge with nothing to scroll it - held clips
silently, which is why the rule is structural. The live sweep
(`sweep-078.mjs`) measures what jsdom cannot, at three window sizes.

The Versions editor with its form open, and the note being edited, still
scroll a column of their own natively: their text areas size against the
column, which ScrollArea's content box does not give them. They move to the
card-by-the-mockup stage with the rest of those tabs.
