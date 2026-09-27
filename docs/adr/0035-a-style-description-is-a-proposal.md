# 35. A style's description is a proposal

Date: 2026-09-27

## Status

Accepted.

## Context

v0.75 gave the style dictionary an action, *Describe from the references*:
the assistant reads a brick's pictures and writes the description the brick
will carry. Its answer had to be kept onto the brick, and that was built as a
command of its own, `describe_style_brick(message, brick)`, rather than as a
`produces` value. The reason given was that a brick is not a work, and the
proposal machinery hung on one.

By v0.76 that reason no longer held: a drafted reply to a comment is a
proposal about something that is not a work either. And the command never got
its button - the answer could only be copied by hand, which the owner asked to
fix (wish 3034). A command the window never calls was also one of the thirteen
the audit of 24.09 found.

## Decision

**An action about a style produces a `description`,** the way an action about
a comment produces a `comment` or a `reply`:

- `"produces": "description"` is valid only with `"scope": "style"`, and an
  action with that scope must produce it - an answer about a brick with nowhere
  to go is a button that ends in a copy and paste. `ProfileConfig::validate`
  says both.
- The finished run's answer is marked with `{ kind: "description", style_id }`,
  the brick read out of the task key the dictionary started it with
  (`describe-style:style:<id>`).
- Applying it goes through `apply_proposal` like every other proposal, and is
  written as a `style.update` - the edit a person makes in the dictionary - so
  undo takes it back and a rebuild from the log plays it: the text travels in
  the patch. A kept description lets a draft out into ready; a retired brick
  stays retired.
- `describe_style_brick` is removed. The bell lists the waiting description,
  and its line opens the chat in the drawer.

## Consequences

- One way for an answer to become data. The chat, the bell and the "applied"
  mark treat a description like a note or a reply, with no special case.
- Old log entries of kind `style.describe` still reverse; nothing writes new
  ones.
- A profile whose style action says nothing about `produces` picks up
  `description` from the shipped copy on the next start (`carry_forward` fills
  a `produces` the stored copy leaves empty); one that says something else no
  longer validates on save, with the reason.
