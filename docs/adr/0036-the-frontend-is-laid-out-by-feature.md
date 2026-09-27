# 36. The frontend is laid out by feature, with one list of screens and one of tabs

Date: 2026-09-27

## Status

Accepted. Amends 0034: casing no longer tells a registry copy from the app's
own component; the folder does.

## Context

By v0.76 the frontend had grown by accretion. `src/components/` held
twenty-three screen components at its root beside the folders some screens had
been given, dowel's registry copies shared `components/ui/` with the app's own
primitives and were told apart by the case of the first letter, and `lib/` held
both pure logic and hooks of single screens. The audit of 24.09 counted the
screens described in four places - the router, the rail, the title bar's names
and the keyboard's jumps - and a card's tabs in four more; a screen added to
one and forgotten in another had shipped three times.

The refactor chain that follows (v0.78 to v0.82) moves most of these files
again, screen by screen. It needs to know where each belongs first.

## Decision

**The folders say what a file is for:**

- `src/app/` - the window's root: `App`, the providers it is rendered in (the
  same ones the tests render it in), toasts, the keyboard, and the list of
  screens.
- `src/shell/` - the frame around every screen: the rail, the title bar, the
  command palette, the splash, the shortcut sheet.
- `src/features/<area>/` - a screen and what only it uses.
- `src/features/work/` - the open work's frame; `tabs/<tab>/` holds each tab.
- `src/components/` - what two features or more share; `components/ui/` holds
  dowel's registry copies and nothing else, held byte for byte by
  `pnpm registry`.
- `src/lib/` - logic with no screen of its own, and the wire to the backend.

**One list of screens,** `app/screens.tsx`: each entry carries its address, its
word, its place in the rail, how it scrolls, its shortcut letter and what it
draws. The router, the rail, the title bar and the shortcut sheet all read it.
A screen still to come is listed with the version bringing it, and a test fails
once that version is the one being built.

**One list of tabs,** `features/work/tabs.ts`: the order, which tabs are two
columns held to the card's height, and which a work must have a storyboard or
a splice for. What each tab draws is a `Record` over the same keys in
`TabBody.tsx`, so a tab added to the list does not compile without a body.

Both lists are held to the smoke test: a screen or a tab it does not open fails.

## Consequences

- A file's place is a decision made once: a screen's component that another
  feature starts to use moves to `components/`.
- The tests, the lint rules and the gates find what they check by markers or
  by these lists, not by paths, so a later move does not blind them.
- One formatting commit and one pure move sit before this in the history; both
  are rename-only for `git log --follow`, and the formatting one is in
  `.git-blame-ignore-revs`.
