import type { Scene, SceneBlock } from '@/lib/api'

/** How far a scene is from being ready to shoot. */
export type Readiness = 'empty' | 'started' | 'ready'

/**
 * Whether a scene is filled in, by what the profile asks a scene to carry.
 *
 * Readiness is *computed*, never stored: ADR 0020 turned down a status column
 * for the frame because it derives from what the scene holds, and until assets
 * arrive (0.67) what it holds is its description and its prompt blocks. When a
 * scene has a frame of its own to show, that becomes part of this answer
 * rather than a second, contradicting one.
 *
 * - `empty` — nothing written yet: a row the frame made and nobody has touched.
 * - `started` — some of it: a description without prompts, or the reverse.
 * - `ready` — a description and every block the profile names.
 *
 * A kind that names no blocks asks only for a description; a board of such a
 * kind is ready as soon as it is described.
 */
export function readinessOf(scene: Scene, blocks: SceneBlock[]): Readiness {
  const described = scene.description.trim() !== ''
  const written = blocks.filter((block) => (scene.blocks[block.key] ?? '').trim() !== '').length

  if (!described && written === 0) return 'empty'
  if (described && written === blocks.length) return 'ready'
  return 'started'
}
