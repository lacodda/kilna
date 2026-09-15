# 0028 — A scene keeps its frames, and chooses one

Date: 2026-09-16
Status: Accepted

## Context

A scene carries a description and prompt blocks; the blocks are copied into a
generator and the generator answers with pictures. Four of them, usually —
comparing the candidates is the work, not an accident of the tool.

The previous stage (ADR 0027) gave the application a place to put a file: the
`asset` table, with the bytes copied into the workspace's `media/`. That
solved where a picture lives. It did not say whose picture it is. `asset`
hangs on `work_id` and `release_id`; a scene is not in it at all, and a
picture belonging to a scene also needs an order among its siblings and a
mark for the one the video will be cut from.

The predecessor could not answer "which scene is this picture" at all —
pictures sat in folders named by hand, and the link between a still and the
scene it was drawn for lived in the person's memory.

## Decision

**Frames are their own table, pointing at assets.** `scene_frame` holds a
profile, a scene, an asset, a position and a mark. Three columns that would
stand empty on every cover and every attachment is the cost of putting them
on `asset` instead, and it would mean one table holding two different lives.
The file keeps its one home: `asset` says what the bytes are and what the
world called them, `scene_frame` says whose frame that is and where it
stands.

**One chosen frame per scene is enforced by the schema.** A partial unique
index — `UNIQUE (scene_id) WHERE is_selected = 1` — makes two chosen frames
unrepresentable rather than something every writer must remember to prevent.
Rejected: a `selected_frame_id` column on `scene`, which points a parent at
its child and needs deferred constraints the moment a frame is deleted.

**A scene with no frame chosen is ordinary.** Four candidates and no verdict
is the middle of the work, and a person may go back to it; `clear_selection`
exists for that and is not an error state.

**Attaching a frame is irreversible; choosing and ordering are not.**
Attaching carries a file, so it is not replayed from the log and not offered
to undo — the log holds the log, not the bytes, exactly as ADR 0027 says of
`asset.attach`. An order and a verdict carry no bytes and are pure judgement,
so both are replayed and both are undoable, and the operation records what
they were *before*: an undo that forgot the previous verdict would silently
drop a decision the person had already made, which is worse than no undo.

**Readiness gains a step rather than a condition.** A scene is `shot` when it
is filled in *and* has a chosen frame — after `ready`, not instead of it. A
scene written out in full is genuinely ready; demoting it to unfinished the
moment frames exist elsewhere on the board would move the goalposts under the
person. ADR 0020 promised readiness would be derived from what a scene holds
rather than stored, and this keeps that promise: the new step is computed the
same way the others are.

**A pasted picture travels as bytes.** The clipboard hands the window pixels,
not a path. Writing them to a file in the window would mean giving the
application filesystem permissions for one temporary write into a directory
the backend already owns; instead the bytes go to the backend, land in a
temporary file there, and take the same copy path a picked file takes. One
road, not two.

## Consequences

The storyboard answers in both directions: a scene shows the frame it is cut
from beside its number, and a frame names its scene. A generator's four
answers can sit side by side until one is chosen.

Frames are not synchronised by the operation log, so a workspace rebuilt from
the log alone comes back with its scenes and its verdicts but without the
pictures — the same limit assets already have, and the backup (ADR 0027)
remains the way pictures travel.

A future stage that puts video beside a frame (v0.69) has a place to hang it:
the frame is already the row that knows which scene and which order.
