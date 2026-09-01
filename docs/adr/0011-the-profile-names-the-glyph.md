# 0011 — The profile names the glyph

Date: 2026-09-01
Status: Accepted

## Context

ADR 0001 states the rule the whole product hangs on: the schema is one, and
what differs between crafts lives in the profile. A consequence of it, written
into the calendar at v0.35, is that **the code is not allowed to know which
kinds of release exist.** Music ships clips, shorts and audio releases; a
novel ships beta reads and submissions; a blog ships a site publish and a
newsletter send. A `switch` on the kind key would be a place where the code
decides what a craft is, and the next craft would have to be added to it.

That rule left the calendar chip with a problem. A day is ~130px wide and can
hold several releases, and the kind had to fit beside the title without taking
it. v0.35 solved it by shortening the label to two letters — `Video clip` →
`VC` — which is a rule about strings and knows nothing about crafts. It fit,
and it was legible in the sense that it did not overflow.

The pilot's calendar, and the owner's own working calendar in atlas, put a
glyph there instead: a film frame, a phone, a disc. Two letters and a glyph
cost the same width; the glyph is read at a glance and `VC` is decoded. The
question this stage had to answer was whether kilna is allowed to draw one.

## Decision

**The profile names the glyph, from a vocabulary the frontend holds.**

`ReleaseKind` gains an optional `icon`: a name like `film` or `rss`. The
profile document — the file the owner writes and edits — states it alongside
`label` and `requires`. `src/lib/releaseIcon.tsx` holds the vocabulary those
names may point into, currently eighteen glyphs.

The code still does not know what a clip is. It knows how to draw a film frame,
and the profile is what says a clip looks like one.

## Consequences

**A closed vocabulary, not a lookup into the icon library.** The name arrives
from a file a person edits. An open lookup would let a typo take down the
calendar, and would let any string reach anything `lucide-react` exports —
including, as a test found before this shipped, `constructor`, which is not
undefined but `Object.prototype.constructor`, handed to React as a component.
The lookup is guarded by `Object.hasOwn` for exactly that reason.

**An unknown name costs an icon, not a screen.** A kind whose glyph is absent,
misspelled, or dropped from the vocabulary is drawn with a neutral calendar
mark. There is no warning: the label is still in the chip's tooltip and in the
filter row above the grid, so nothing is unreachable — the glyph is a shortcut,
never the only place a fact lives.

**The vocabulary and the shipped profiles must agree, and a test says so.** A
glyph renamed in one file and not the other would silently cost every chip of
that kind its mark. The test reads the four shipped profiles and asserts each
kind names a glyph the vocabulary holds — and that it names one at all, since a
missing glyph and a renamed one look identical on screen.

**A kind the owner invented gains nothing.** The backfill that carries new
fields into an older workspace matches kinds by key against the shipped
profile. `Vinyl pressing` is not in it, so it keeps the neutral mark until the
owner names a glyph. Guessing one is not something this code can do.

**This is a change to the profile document, made ahead of the package that was
meant to carry it.** The plan collects every breaking change to the schema and
the profile config into v0.50–v0.51, so that they land as one migration. This
one is additive — an optional field, absent in every profile written before it,
defaulted on load — so it costs that package nothing and does not make the old
model more expensive to leave. The decision to take it now was the owner's.

## Alternatives considered

**A colour per kind, derived from the key.** Nothing to configure, works for
any profile, and the hashing machinery already exists for cover gradients. But
the chip's ground is already the work's own colour — the same song is the same
colour in the catalogue, the card and the calendar — so a second colour on the
same chip would have been two colour systems arguing on 130 pixels.

**Keeping the two letters.** Defensible: v0.35 widened the day from 77px to
131px, and most of what made the pilot's calendar unreadable was the width, not
the abbreviation. Rejected because the glyph is the half that does not have to
be decoded, and because the filter row this stage adds needs a legend — the
same marks, read twice, teach themselves.

**A glyph per kind hard-coded in the frontend.** Simplest to write and the
fastest to read, and wrong: it is the `switch` on the kind key that ADR 0001
exists to prevent.
