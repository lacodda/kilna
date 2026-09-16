import type { Scene, SceneBlock, SceneFrame, Work } from '@/lib/api'

/** What a row of a scene's material is: the still, or the clip cut from it. */
export const FRAME = 'frame'
export const VIDEO = 'video'

/** How far a scene is from being ready to shoot, and then from being shot. */
export type Readiness = 'empty' | 'started' | 'ready' | 'shot'

/**
 * Whether a scene is filled in, by what the profile asks a scene to carry.
 *
 * Readiness is *computed*, never stored: ADR 0020 turned down a status column
 * for the frame because it derives from what the scene holds — the description,
 * the prompt blocks, and now the picture drawn from them.
 *
 * - `empty` — nothing written yet: a row the frame made and nobody has touched.
 * - `started` — some of it: a description without prompts, or the reverse.
 * - `ready` — a description and every block the profile names: ready to draw.
 * - `shot` — and a frame chosen, so the video has something to cut.
 *
 * `shot` is a step past `ready` rather than a condition inside it. A scene
 * written out in full is genuinely ready — that is the state the person worked
 * towards, and demoting it to "unfinished" the moment frames exist elsewhere on
 * the board would move the goalposts under them. Choosing the frame is the next
 * thing that happens, not a debt against what already happened.
 *
 * Frames that are merely attached do not count: four candidates and no verdict
 * is the middle of the work, not the end of it. The scene is shot when one of
 * them has been chosen.
 *
 * Shot is about the STILL. Since v0.69 a scene also holds the clips animated
 * from it, in the same list under a kind, and a chosen clip must not stand in
 * for a chosen picture: a board where the clip was picked and the still never
 * was is not a board that has been shot, it is one in a state nobody meant.
 * Whether the video is cut is a further question, and the montage list is
 * where it gets asked.
 *
 * A kind that names no blocks asks only for a description; a board of such a
 * kind is ready as soon as it is described.
 */
export function readinessOf(
  scene: Scene,
  blocks: SceneBlock[],
  frames: SceneFrame[] = [],
): Readiness {
  const described = scene.description.trim() !== ''
  const written = blocks.filter((block) => (scene.blocks[block.key] ?? '').trim() !== '').length

  if (!described && written === 0) return 'empty'
  if (described && written === blocks.length) {
    const picked = frames.some((frame) => frame.kind === FRAME && frame.is_selected)
    return picked ? 'shot' : 'ready'
  }
  return 'started'
}

/** The frames of each scene, by scene id — one pass over a board's frames. */
export function framesByScene(frames: SceneFrame[]): Map<string, SceneFrame[]> {
  const byScene = new Map<string, SceneFrame[]>()
  for (const frame of frames) {
    const list = byScene.get(frame.scene_id)
    if (list) list.push(frame)
    else byScene.set(frame.scene_id, [frame])
  }
  return byScene
}

/** One kind of a scene's material, in the order it stands in. */
export function ofKind(frames: SceneFrame[] | undefined, kind: string): SceneFrame[] {
  return (frames ?? []).filter((frame) => frame.kind === kind)
}

/** The still a scene is cut from, if it has chosen one. */
export function chosenFrame(frames: SceneFrame[] | undefined): SceneFrame | undefined {
  return frames?.find((frame) => frame.kind === FRAME && frame.is_selected)
}

/** The clip a scene is cut to, if one has been chosen. */
export function chosenVideo(frames: SceneFrame[] | undefined): SceneFrame | undefined {
  return frames?.find((frame) => frame.kind === VIDEO && frame.is_selected)
}

/**
 * The board's order with one scene put at a number, the rest closing up.
 *
 * The arithmetic behind both gestures the board offers — "this one happens
 * third" typed on the number, and "add a scene after this one" — written once
 * so the two cannot disagree about what a number means. `to` is the number
 * the person asked for, counted from 1 in the board as they see it; the scene
 * is taken out of the order and put back there.
 *
 * The result is what `renumber_scenes` takes: the whole order, which it
 * checks names every scene exactly once before setting 1..N from it. A number
 * past either end means that end, because a person who asks for a number off
 * the board means the edge of it.
 */
export function orderMoving(scenes: Scene[], id: string, to: number): string[] {
  const order = scenes.map((scene) => scene.id).filter((held) => held !== id)
  const at = Math.min(Math.max(to - 1, 0), order.length)
  order.splice(at, 0, id)
  return order
}

/** The key a work's length is kept under, in the profile's meta fields. */
export const DURATION = 'duration'

/**
 * A work's length in seconds, when its meta carries one.
 *
 * Read leniently, and read the same way `scene::duration_of` reads it: the
 * field is a number since migration 0018, but a workspace whose owner typed
 * into it before the retype — or typed a timecode back into it after — can
 * still hold `"3:45"`, and that means the same length as `225`. A value that
 * means nothing readable is NO length rather than a zero, so a board is left
 * unchecked against a length instead of being judged against one that is not
 * there.
 *
 * This restates the backend's reading rather than asking for it, and that is
 * a real cost: two readers of one field can drift. It is paid because the
 * alternative is a round trip for a single number the screen already holds
 * the work for, and the reading is fixed by a migration rather than by
 * anything that changes — `duration_of` in `scene.rs` is where it is decided,
 * and this follows it.
 */
export function durationOf(work: Work): number | null {
  const value = work.meta[DURATION]
  if (typeof value === 'number') {
    return Number.isFinite(value) && value > 0 ? value : null
  }
  if (typeof value !== 'string') return null
  const text = value.trim()
  if (text === '') return null
  let seconds = 0
  for (const part of text.split(':')) {
    const number = Number(part.trim())
    if (part.trim() === '' || !Number.isFinite(number) || number < 0) return null
    seconds = seconds * 60 + number
  }
  return seconds > 0 ? seconds : null
}
