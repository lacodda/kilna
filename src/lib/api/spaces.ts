import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { CurrentSpace, NewSpace, Opened, Space, SpaceLook } from '@/lib/api/types'

// Spaces (v0.109, ADR 0077): each a folder with a database of its own. The
// window has one open; opening another swaps the database without a restart.

export const listSpaces = () => invoke<Space[]>('list_spaces')
export const currentSpace = () => invoke<CurrentSpace>('current_space')
export const createSpace = (space: NewSpace) => invoke<Space>('create_space', { space })
export const openSpace = (id: string) => invoke<CurrentSpace>('open_space', { id })
export const renameSpace = (id: string, name: string) => invoke<Space>('rename_space', { id, name })
export const restyleSpace = (id: string, look: SpaceLook) =>
  invoke<Space>('restyle_space', { id, look })
export const trashSpace = (id: string) => invoke<void>('trash_space', { id })
export const restoreSpace = (id: string) => invoke<void>('restore_space', { id })
export const forgetSpace = (id: string) => invoke<void>('forget_space', { id })

/** The link the first start was handed, once - `kilna://...` - or nothing. */
export const takeLaunchTarget = () => invoke<string | null>('take_launch_target')
/** Open what a link or a double-clicked `space.kilna` names: the space, and
 *  the screen in it the link points at. */
export const openTarget = (target: string) => invoke<Opened>('open_target', { target })

/** A link or a space's file handed to kilna while it is open. */
export const onOpenRequest = (handler: (target: string) => void): Promise<UnlistenFn> =>
  listen<string>('app:open', ({ payload }) => handler(payload))
