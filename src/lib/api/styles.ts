import { invoke } from '@tauri-apps/api/core'
import type {
  Asset,
  NewStyleBrick,
  StartedTask,
  StyleBrick,
  StyleBrickFilter,
  StyleBrickPatch,
} from '@/lib/api/types'

// The style dictionary: bricks, their references, and the task that describes one.

export const listStyleBricks = (filter?: StyleBrickFilter) =>
  invoke<StyleBrick[]>('list_style_bricks', { filter })
export const styleBrickCounts = () => invoke<[string, number][]>('style_brick_counts')
export const styleBrickReferences = (id: string) =>
  invoke<Asset[]>('style_brick_references', { id })
export const createStyleBrick = (brick: NewStyleBrick) =>
  invoke<StyleBrick>('create_style_brick', { brick })
export const updateStyleBrick = (id: string, patch: StyleBrickPatch) =>
  invoke<StyleBrick>('update_style_brick', { id, patch })
// Returns the trash entry, which is what the undo on the toast restores.
export const deleteStyleBrick = (id: string) => invoke<string>('delete_style_brick', { id })
/** A reference pasted straight onto a brick, the way a frame is. */
export const pasteStyleReference = (id: string, bytes: number[], name: string) =>
  invoke<Asset>('paste_style_reference', { id, bytes, name })
/** Describe a brick from its references — the dictionary's own AI action. */
export const startStyleTask = (id: string, action: string) =>
  invoke<StartedTask>('start_style_task', { id, action })
