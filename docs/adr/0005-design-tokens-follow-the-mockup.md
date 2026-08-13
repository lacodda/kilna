# 0005 — Design tokens follow the mockup vocabulary

Date: 2026-08-13
Status: Accepted

## Context

The approved v1.0 interface mockup defines a complete token set — surfaces
(`bg`, `raise`, `soft`), strokes (`line`, `line-2`), ink (`text`, `dim`,
`faint`), the brand accent and four semantic colors, each with a soft tint.
shadcn/ui, which ADR 0003 adopts as the component source, names the same
ideas differently: `background`, `card`, `border`, `muted-foreground`,
`primary`, and its `accent` means a subtle hover surface rather than the
brand color.

Two vocabularies for one palette means translating in your head every time a
screen is checked against the mockup, and `accent` meaning opposite things in
the two systems is a standing invitation to pick the wrong token.

## Decision

CSS variables and Tailwind utilities use the mockup's names, not shadcn's.
shadcn components are copied into the repository (per ADR 0003) and re-pointed
at these tokens by hand as part of the copy.

Themes are three-state: no class on `<html>` follows the OS, an explicit
`light` or `dark` class pins the theme. Components never use Tailwind `dark:`
variants — every color goes through a token, so a component is theme-correct
by construction. The default Tailwind palette is disabled (`--color-*:
initial`), which turns any non-token color into a visibly unstyled element
rather than a silent hard-coded value.

## Consequences

**Positive.** Screens and the mockup speak one language, so checking a screen
against it is a comparison, not a translation. Theme-correctness cannot be
forgotten per-element. Adding a color means adding a token, which keeps the
palette closed.

**Negative.** Every shadcn component picked up later needs its classes
re-pointed by hand — the copies drift from upstream diffs by exactly that
re-pointing. The token set must stay small for this to stay cheap.
