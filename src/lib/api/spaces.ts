import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  CurrentSpace,
  NewSpace,
  Opened,
  ProfileConfig,
  ProfileDocument,
  Space,
  SpaceLook,
  SpaceType,
} from '@/lib/api/types'
import type { Reason } from '@/lib/api/generated/Reason'

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

// Types of space (v0.112, ADR 0079): a profile kept as what a new space is
// made of, and the profile as a document that travels in a file.

export const listSpaceTypes = () => invoke<SpaceType[]>('list_space_types')
/** Keep the open space's profile, as it is saved, as a type under a name. */
export const saveSpaceType = (name: string) => invoke<SpaceType>('save_space_type', { name })
/** Keep a document read from a file as a type. */
export const addSpaceType = (document: ProfileDocument) =>
  invoke<SpaceType>('add_space_type', { document })
export const renameSpaceType = (id: string, name: string) =>
  invoke<SpaceType>('rename_space_type', { id, name })
export const deleteSpaceType = (id: string) => invoke<void>('delete_space_type', { id })
/** A type's document as the text of its file. */
export const spaceTypeDocument = (id: string) => invoke<string>('space_type_document', { id })
/** The open space's profile as the text of a file. */
export const profileDocument = () => invoke<string>('profile_document')
/** A profile document read from a file the person picked, checked whole. */
export const readProfileDocument = (path: string) =>
  invoke<ProfileDocument>('read_profile_document', { path })
/** What is wrong with a profile as the editor holds it, each problem a reason
 *  with its place. */
export const checkProfileConfig = (config: ProfileConfig) =>
  invoke<Reason[]>('check_profile_config', { config })

/** The link the first start was handed, once - `kilna://...` - or nothing. */
export const takeLaunchTarget = () => invoke<string | null>('take_launch_target')
/** Open what a link or a double-clicked `space.kilna` names: the space, and
 *  the screen in it the link points at. */
export const openTarget = (target: string) => invoke<Opened>('open_target', { target })

/** A link or a space's file handed to kilna while it is open. */
export const onOpenRequest = (handler: (target: string) => void): Promise<UnlistenFn> =>
  listen<string>('app:open', ({ payload }) => handler(payload))
