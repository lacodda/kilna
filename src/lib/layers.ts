import type { CoverBricks } from '@/lib/api/types'

/**
 * The layers of a picture above its style, in the order a prompt says them
 * and its contracts are read in (v0.134) - the backend's `cover::LAYERS`.
 * Each is a brick of the type of its own name.
 */
export const LAYERS = [
  'composition',
  'environment',
  'look',
  'pose',
  'angle',
  'layering',
  'lighting',
  'optics',
  'grading',
] as const satisfies readonly (keyof CoverBricks)[]

export type Layer = (typeof LAYERS)[number]
