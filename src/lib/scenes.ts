import type { Scene, SceneBlock, SceneFrame } from '@/lib/api'

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
    return frames.some((frame) => frame.is_selected) ? 'shot' : 'ready'
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

/** The frame a scene is cut from, if it has chosen one. */
export function chosenFrame(frames: SceneFrame[] | undefined): SceneFrame | undefined {
  return frames?.find((frame) => frame.is_selected)
}
