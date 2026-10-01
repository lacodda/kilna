# 0048 — A craft ships a starter set of styles

Date: 2026-09-30
Status: Accepted

## Context

ADR 0031 made the style dictionary one table owned by the workspace, and the
profile named the types a brick can be. Every brick was one the owner made, so
a new workspace opened on an empty dictionary - and the cover constructor of
v0.88 is built out of bricks: a picture with no image styles, no typography,
no backgrounds to pick has nothing to be built from.

A craft knows a good deal about what its pictures are made of. That knowledge
belongs with the craft - in the shipped profile - and it has to reach the
workspace as bricks the owner can then make their own. Three things make that
harder than an insert:

- **The owner edits them.** A brick of the set is theirs from the moment it
  lands. A newer set must not write over what they changed - and must reach
  what they did not, or the set is frozen at the version that first seeded it.
- **The owner deletes them.** A brick deleted on purpose must not come back on
  the next start.
- **Workspaces multiply.** A rebuild from the log (ADR 0014) and a second
  device both seed the same set; each seeded brick has to be the same brick.

## Decision

**The set lives in the shipped profile beside the document, and is seeded into
`style_brick` every time the workspace opens.**

1. **Beside, not inside.** `style_set` is a top-level list of the shipped
   profile file, next to `config`, not a field of the profile document. The
   bricks it seeds are the workspace's copy; a second copy inside every stored
   document would be two truths, one of them never read.
2. **A derived id.** A set brick's id is a UUID v5 of the profile's key and the
   entry's key. Two devices and a rebuild seed the same brick; a brick the owner
   deleted is found by its tombstone and stays deleted.
3. **A fingerprint, not a flag.** `set_digest` is the fingerprint of the entry
   last written into the row, over what the brick *says* - type, name, the name
   per language, family, description, when to take it, colours, sample. The
   row's own fingerprint equal to it means nobody touched the brick: it reads
   *from the set*, and a newer entry rewrites it. Different means *changed*: it
   keeps the owner's words and offers **Restore as in the set** - one ordinary
   `style.update` carrying every field and the new fingerprint, so undo takes it
   back. Status and the steer are not in the fingerprint: they are about the
   brick, not what it says. An edit typed and then typed back is no edit.
4. **The name per language.** A set brick carries its shipped name as
   `{en, ru}` (`label`); `name` stays the one word the dictionary is unique on.
   Renaming a brick clears the label in the same patch - the new name is the
   owner's, in one language.
5. **The form is the type's.** What a brick is made of beside its description -
   pictures, a lettering sample, slots, one colour - is `form` on the style
   type, so the application knows four forms and never a type (ADR 0001).
   A type the craft no longer builds from is `retired`, not removed: its bricks
   still read with their word, and none are made.
6. **Slots drop with their phrase.** A dressing's description names captions
   as `{slot}` or `{slot.N}`. Filled from the captions of the channel's card
   (and, from v0.88, of the work), a phrase in `[square brackets]` whose slot
   is empty drops out whole; a slot outside brackets takes its clause. The set
   brackets every slot, and a test holds it to that.

Seeding is not a gesture and writes no log entry: no person made it.

## Consequences

- A new workspace opens with ninety bricks, and a newer build improves the ones
  nobody touched.
- The set is shipped data held to tests: every entry of a type the profile
  names and has not retired, a family its type has, a name in both languages,
  English words for the model, a colour as `#RRGGBB`, every slot in brackets.
- The set is neutral by the project's privacy rule: no channel, mascot or
  number of anyone's.
- A brick of the owner's own that already has a set entry's name in its type
  keeps the name; the set entry stays out, and is logged.
