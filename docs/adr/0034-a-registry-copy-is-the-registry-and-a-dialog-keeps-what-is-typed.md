# 34. A registry copy is the registry, and a dialog keeps what is typed into it

Date: 2026-09-25

## Status

Accepted.

## Context

kilna's primitives are copies of dowel's registry in `src/components/ui/`, with
a lowercase name, beside the application's own shapes with a PascalCase one.
The rule was that a copy is never edited. On 2026-09-24 twelve of them had
drifted:

- **Seven carried a popup layer of kilna's own.** A select, a menu or a date
  picker opened inside a dialog or the full-screen version stage drew under
  it; kilna fixed that with its own `layer.tsx`, a mirror of dowel's z-index
  ladder in `lib/layers.ts` and a test that the mirror had not drifted, and
  patched the seven copies to use it. The fix was right and in the wrong
  place: the layer host sat on the body beside a modal's portal, where Base UI
  hides it from a screen reader.
- **One had a hand-set height** (`input` at `h-9` where the registry reads the
  density row), and the rest had been left behind by an upgrade.

`dowel diff` showed every line of it to anyone who ran it, and nobody had to.

The dialogs had the same shape of problem one level up. `AppDialog` put the
whole popup in one scroller, so the actions went with the content - at a
1280x720 window the style editor's Save stood 137px below the edge; it took
width as a class the popup's own width outranked; it opened focused on the
close cross; and a click beside it threw away whatever had been typed.

## Decision

**A lowercase file in `components/ui/` is byte-identical to its twin in the
installed `dowel-ui`, and a check says so.** `tools/check-registry.mjs`, run by
`pnpm lint`, compares every lowercase file with the registry the package ships
(line endings aside) and fails on a copy that differs and on a lowercase file
with no twin. It has an exceptions list, and the list is empty: an entry would
be a debt with a name, not a way to pass. A fix a copy needs goes to dowel and
comes back with the next copy. The popup layer did exactly that - it is dowel's
`layer.tsx` since 0.32, its floor computed in CSS, so `lib/layers.ts` and its
drift test are gone.

**`AppDialog` is dowel's dialog anatomy with four behaviours of its own:** the
width is a `size` (`md` to `full`); the header and the actions stay put and
only the body scrolls; focus opens on the first field, or on Cancel when there
is none; and once anything has been typed into it, a click beside it does not
close it - `Escape` and Cancel still do. Typing is noticed by the `input`
event bubbling up to the popup, so no caller reports it; a form whose changes
live in custom controls passes `dirty`. The two rules that dialogs built on
dowel's parts directly also follow live in `lib/dialogGuard.ts`.

**The window is compact:** `data-density="compact"` on `<html>`, so portalled
popups inherit it. A field and a default button are 32px, the small button
28px; kilna's own controls stand on `h-control` and `h-control-sm` rather than
on numbers.

## Consequences

- Upgrading `dowel-ui` without re-copying fails the gate, which is the point:
  a copy of an older dowel than the one installed is found at once.
- A behaviour kilna needs from a primitive cannot be patched in locally; it has
  to be asked of dowel, and waits for dowel's release.
- A dialog with only custom controls and no `dirty` still closes on a stray
  click. The check-list for a new dialog is `size`, `aside` and `dirty`.
- Button md is 32px at compact density, not the 28px the mockup draws: dowel
  puts a default button on the same row as a field. Recorded as a decision in
  the project record on 2026-09-25.
